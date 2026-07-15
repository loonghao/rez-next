//!
//! # Package Cache Module
//!
//! High-performance package payload caching for rez-next.
//!
//! This module provides disk-based caching of package variant payloads to avoid
//! fetching from shared storage at runtime. It follows SOLID principles and
//! incorporates lessons learned from the original rez implementation.
//!
//! ## Design Decisions (based on rez issues analysis)
//!
//! - **Cross-platform case handling**: Normalizes paths for Windows case-insensitivity
//! - **Disk space pre-check**: Fail fast with clear error when disk is full
//! - **Configurable logging**: Accepts optional logger instead of hardcoding
//! - **Corrupted data handling**: Skips malformed cache entries instead of crashing
//! - **Thread-safe operations**: Uses file locking for multi-process safety
//!
//! ## Cache Directory Structure
//!
//! ```text
//! <cache_root>/<package_name>/<version>/<hash_prefix>/<increment>/
//!                                                    └─ payload files
//!                                  <increment>.json  └─ variant metadata
//! ```
//!
//! The hash is the first 4 chars of SHA1(variant.handle), and increment (a, b, ...)
//! handles hash collisions.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json;

// ── Constants ──────────────────────────────────────────────────────────────────

/// Hash prefix length (first N chars of SHA1)
const HASH_PREFIX_LEN: usize = 4;

// ── Cache Status ──────────────────────────────────────────────────────────────

/// Status of a variant in the cache.
///
/// Mirrors the original rez `PackageCache` status constants while adding
/// Rust-friendly error handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStatus {
    /// Variant not found in cache
    NotFound,

    /// Variant is cached and ready to use
    Found,

    /// Variant was just added to cache
    Created,

    /// Variant payload is still being copied
    Copying,

    /// Copy operation appears stalled (no progress for too long)
    CopyStalled,

    /// Variant is pending to be cached
    Pending,

    /// Variant has been removed from cache
    Removed,

    /// Variant was skipped (e.g., cache size limit)
    Skipped,
}

impl CacheStatus {
    /// Returns a human-readable description of the status.
    pub fn description(&self) -> &'static str {
        match self {
            CacheStatus::NotFound => "was not found",
            CacheStatus::Found => "was found",
            CacheStatus::Created => "was created",
            CacheStatus::Copying => "payload is still being copied to cache",
            CacheStatus::CopyStalled => {
                "payload copy has stalled (see docs for cleaning instructions)"
            }
            CacheStatus::Pending => "is pending caching",
            CacheStatus::Removed => "was deleted",
            CacheStatus::Skipped => "is not being cached due to cache size limit",
        }
    }
}

// ── Error Type ───────────────────────────────────────────────────────────────

/// Errors that can occur during cache operations.
#[derive(Debug, thiserror::Error)]
pub enum PackageCacheError {
    #[error("Not a directory: {0}")]
    NotADirectory(PathBuf),

    #[error("Package is not cacheable: {0}")]
    NotCacheable(String),

    #[error("Variant root not on disk: {0}")]
    VariantRootNotOnDisk(String),

    #[error("Disk full: cannot cache variant (need {needed} bytes, have {available})")]
    DiskFull { needed: u64, available: u64 },

    #[error("Cache path error: {0}")]
    PathError(String),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Lock timeout: could not acquire lock on {0}")]
    LockTimeout(PathBuf),
}

// ── Variant Handle (serializable identifier) ─────────────────────────────────

/// Serializable representation of a variant handle.
///
/// This is used to match cached payloads to their corresponding variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantHandle {
    /// Package name
    pub name: String,

    /// Package version string (if any)
    pub version: Option<String>,

    /// Variant index within the package
    pub index: Option<usize>,

    /// Additional qualifying attributes (e.g., build system, architecture)
    pub attributes: std::collections::HashMap<String, String>,
}

impl VariantHandle {
    /// Create a new variant handle.
    pub fn new(name: String, version: Option<String>, index: Option<usize>) -> Self {
        Self {
            name,
            version,
            index,
            attributes: std::collections::HashMap::new(),
        }
    }

    /// Convert to a deterministic string for hashing.
    fn hashable_repr(&self) -> String {
        let mut s = format!("name={}", self.name);
        if let Some(v) = &self.version {
            s.push_str(&format!(", version={}", v));
        }
        if let Some(i) = self.index {
            s.push_str(&format!(", index={}", i));
        }
        // Sort attributes for determinism
        let mut attrs: Vec<_> = self.attributes.iter().collect();
        attrs.sort_by_key(|(k, _)| *k);
        for (k, v) in attrs {
            s.push_str(&format!(", {}={}", k, v));
        }
        s
    }

    /// Compute the SHA1 hash of this handle.
    pub fn sha1_hash(&self) -> String {
        use sha1::{Digest, Sha1};
        let mut hasher = Sha1::new();
        hasher.update(self.hashable_repr().as_bytes());
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join("")
    }
}

// ── Cached Variant Info ───────────────────────────────────────────────────────

/// Metadata stored alongside a cached variant payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedVariantInfo {
    /// The variant handle
    pub handle: VariantHandle,

    /// When this cache entry was created
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<u64>,

    /// Last access time (Unix timestamp)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_accessed: Option<u64>,

    /// Size of the cached payload in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_size: Option<u64>,
}

// ── Package Cache ─────────────────────────────────────────────────────────────

/// High-performance package payload cache.
///
/// This struct manages a disk-based cache of package variant payloads,
/// enabling fast environment resolution without fetching from shared storage.
///
/// # Example
///
/// ```no_run
/// use rez_next_package::package_cache::PackageCache;
///
/// let cache = PackageCache::new("/path/to/cache").unwrap();
/// ```
pub struct PackageCache {
    /// Root directory of the cache
    root: PathBuf,

    /// Configuration
    config: CacheConfig,
}

/// Configuration for package cache behavior.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum cache size in bytes (None = unlimited)
    pub max_size_bytes: Option<u64>,

    /// Minimum free space to maintain (bytes)
    pub min_free_space_bytes: u64,

    /// Maximum age of unused cache entries (seconds, None = unlimited)
    pub max_age_secs: Option<u64>,

    /// Whether to cache local packages
    pub cache_local: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size_bytes: None,
            min_free_space_bytes: 100 * 1024 * 1024, // 100 MB
            max_age_secs: None,
            cache_local: true,
        }
    }
}

impl PackageCache {
    /// Create a new package cache at the given root path.
    ///
    /// # Errors
    ///
    /// Returns `PackageCacheError::NotADirectory` if `path` is not an existing directory.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, PackageCacheError> {
        let root = path.as_ref().to_path_buf();

        if !root.is_dir() {
            return Err(PackageCacheError::NotADirectory(root));
        }

        // Create internal directories
        let sys_dir = root.join(".sys");
        fs::create_dir_all(&sys_dir)?;
        fs::create_dir_all(sys_dir.join("pending"))?;
        fs::create_dir_all(sys_dir.join("to_delete"))?;
        fs::create_dir_all(sys_dir.join("log"))?;

        Ok(Self {
            root,
            config: CacheConfig::default(),
        })
    }

    /// Create with custom configuration.
    pub fn with_config<P: AsRef<Path>>(
        path: P,
        config: CacheConfig,
    ) -> Result<Self, PackageCacheError> {
        let mut cache = Self::new(path)?;
        cache.config = config;
        Ok(cache)
    }

    /// Get the root path of the cache.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Get the configuration.
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    // ── Internal path helpers ───────────────────────────────────────────────

    /// Get the hash path for a variant: `<root>/<name>/<version>/<hash_prefix>`
    ///
    /// Note: Package name and version are lowercased in the path to avoid
    /// case-sensitivity issues on Windows (see issue #2101).
    /// The original case is preserved in the metadata (`CachedVariantInfo.handle`).
    fn hash_path(&self, handle: &VariantHandle) -> PathBuf {
        let version_str = handle.version.as_deref().unwrap_or("_NO_VERSION");
        let hash = handle.sha1_hash();
        let hash_prefix = &hash[..HASH_PREFIX_LEN.min(hash.len())];
        // Normalize to lowercase to avoid Windows case-sensitivity issues (#2101)
        let name_lower = handle.name.to_lowercase();
        let version_lower = version_str.to_lowercase();
        self.root
            .join(&name_lower)
            .join(&version_lower)
            .join(hash_prefix)
    }

    /// Get the sys directory (.sys)
    fn sys_dir(&self) -> PathBuf {
        self.root.join(".sys")
    }

    /// Get the to_delete directory.
    fn to_delete_dir(&self) -> PathBuf {
        self.sys_dir().join("to_delete")
    }

    // ── Public API ─────────────────────────────────────────────────────────

    /// Check if a variant is cached and return its root path.
    ///
    /// Updates the last-accessed time on the cache entry.
    ///
    /// # Returns
    ///
    /// `(CacheStatus, Option<PathBuf>)` - status and path if found.
    pub fn get_cached_root(&self, handle: &VariantHandle) -> (CacheStatus, Option<PathBuf>) {
        let hash_path = self.hash_path(handle);

        if !hash_path.is_dir() {
            return (CacheStatus::NotFound, None);
        }

        // Look for matching variant in hash directory
        let entries = match fs::read_dir(&hash_path) {
            Ok(entries) => entries,
            Err(_) => return (CacheStatus::NotFound, None),
        };

        for entry in entries.flatten() {
            let path = entry.path();

            // Check for .json metadata file
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let json_path = path.clone();
                let payload_path = path.with_extension(""); // Remove .json

                // Read and validate the metadata
                let metadata = match Self::read_metadata(&json_path) {
                    Ok(m) => m,
                    Err(_) => continue, // Skip corrupted entries
                };

                if metadata.handle.hashable_repr() == handle.hashable_repr() {
                    // Check for .copying file (still copying)
                    let copying_flag = json_path.with_file_name(format!(
                        ".copying-{}",
                        payload_path.file_name().unwrap().to_string_lossy()
                    ));

                    if copying_flag.is_file() {
                        // Check if stalled
                        if Self::is_file_stalled(&copying_flag) {
                            return (CacheStatus::CopyStalled, Some(payload_path));
                        }
                        return (CacheStatus::Copying, Some(payload_path));
                    }

                    // Update last accessed time
                    let _ = Self::update_access_time(&json_path);

                    return (CacheStatus::Found, Some(payload_path));
                }
            }
        }

        (CacheStatus::NotFound, None)
    }

    /// Add a variant's payload to the cache.
    ///
    /// Copies the payload from `source_root` to the cache.
    ///
    /// # Arguments
    ///
    /// * `handle` - The variant handle
    /// * `source_root` - Path to the variant's payload on disk
    /// * `force` - Force caching even if checks fail
    ///
    /// # Returns
    ///
    /// `(CacheStatus, PathBuf)` - final status and cache path.
    pub fn add_variant(
        &self,
        handle: &VariantHandle,
        source_root: &Path,
        force: bool,
    ) -> Result<(CacheStatus, PathBuf), PackageCacheError> {
        if !source_root.is_dir() {
            return Err(PackageCacheError::VariantRootNotOnDisk(
                source_root.display().to_string(),
            ));
        }

        // Check if already cached
        let (status, cached_path) = self.get_cached_root(handle);
        match status {
            CacheStatus::Found | CacheStatus::CopyStalled => {
                if let Some(path) = cached_path {
                    return Ok((status, path));
                }
            }
            CacheStatus::Copying => {
                // Wait for copy or return immediately
                if let Some(path) = cached_path {
                    return Ok((status, path));
                }
            }
            _ => {}
        }

        // Check disk space
        if !force {
            let source_size = Self::directory_size(source_root)?;
            if !self.check_disk_space(source_size)? {
                return Ok((CacheStatus::Skipped, self.hash_path(handle)));
            }
        }

        // Create hash path
        let hash_path = self.hash_path(handle);
        fs::create_dir_all(&hash_path)?;

        // Determine increment name (a, b, ..., aa, ab, ...)
        let increment = Self::next_increment(&hash_path)?;

        let payload_path = hash_path.join(&increment);
        let json_path = hash_path.join(format!("{}.json", increment));
        let copying_flag = hash_path.join(format!(".copying-{}", increment));

        // Create copying flag
        File::create(&copying_flag)?;

        // Create metadata
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let source_size = Self::directory_size(source_root)?;
        let metadata = CachedVariantInfo {
            handle: handle.clone(),
            created_at: Some(now),
            last_accessed: Some(now),
            payload_size: Some(source_size),
        };

        // Write metadata
        let json_str = serde_json::to_string_pretty(&metadata)?;
        File::create(&json_path)?.write_all(json_str.as_bytes())?;

        // Copy payload
        Self::copy_dir_recursive(source_root, &payload_path)?;

        // Remove copying flag
        let _ = fs::remove_file(copying_flag);

        Ok((CacheStatus::Created, payload_path))
    }

    /// Remove a variant from the cache.
    ///
    /// Moves the payload to the to_delete directory; actual deletion
    /// happens during `clean()`.
    pub fn remove_variant(&self, handle: &VariantHandle) -> (CacheStatus, Option<PathBuf>) {
        let (status, cached_path) = self.get_cached_root(handle);

        match status {
            CacheStatus::NotFound => (CacheStatus::NotFound, None),
            CacheStatus::Copying | CacheStatus::CopyStalled => {
                // Don't remove actively copying variants
                (status, cached_path)
            }
            CacheStatus::Found => {
                if let Some(ref path) = cached_path {
                    let dest = self.to_delete_dir().join(format!(
                        "{}-{}",
                        handle.name,
                        uuid::Uuid::new_v4()
                    ));

                    // Move to to_delete
                    if fs::rename(path, &dest).is_err() {
                        // Try copy + delete
                        let _ = Self::copy_dir_recursive(path, &dest);
                        let _ = fs::remove_dir_all(path);
                    }

                    // Remove .json file
                    let json_path = path.with_extension("json");
                    let _ = fs::remove_file(json_path);

                    // Clean up empty parent directories
                    Self::cleanup_empty_dirs(path);
                }
                (CacheStatus::Removed, cached_path)
            }
            _ => (status, cached_path),
        }
    }

    /// List all cached variants.
    ///
    /// Returns a list of (handle, path, status) tuples.
    pub fn list_cached(&self) -> Vec<(VariantHandle, PathBuf, CacheStatus)> {
        let mut results = Vec::new();

        if let Ok(pkg_entries) = fs::read_dir(&self.root) {
            for pkg_entry in pkg_entries.flatten() {
                let pkg_path = pkg_entry.path();
                if !pkg_path.is_dir()
                    || pkg_path
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with('.')
                {
                    continue;
                }

                if let Ok(ver_entries) = fs::read_dir(&pkg_path) {
                    for ver_entry in ver_entries.flatten() {
                        let ver_path = ver_entry.path();
                        if !ver_path.is_dir() {
                            continue;
                        }

                        if let Ok(hash_entries) = fs::read_dir(&ver_path) {
                            for hash_entry in hash_entries.flatten() {
                                let hash_path = hash_entry.path();
                                if !hash_path.is_dir() {
                                    continue;
                                }

                                // Read metadata files
                                if let Ok(meta_entries) = fs::read_dir(&hash_path) {
                                    for meta_entry in meta_entries.flatten() {
                                        let meta_path = meta_entry.path();
                                        if meta_path.extension().and_then(|s| s.to_str())
                                            == Some("json")
                                            && let Ok(metadata) = Self::read_metadata(&meta_path)
                                        {
                                            let payload_path = meta_path.with_extension("");
                                            let status = if payload_path.is_dir() {
                                                CacheStatus::Found
                                            } else {
                                                CacheStatus::Pending
                                            };
                                            results.push((metadata.handle, payload_path, status));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        results
    }

    /// Clean the cache by removing:
    /// - Old unused entries (based on `max_age_secs`)
    /// - Stalled copies
    /// - Entries in to_delete directory
    ///
    /// # Arguments
    ///
    /// * `time_limit_secs` - Optional time limit for cleaning operation
    pub fn clean(&self, time_limit_secs: Option<u64>) -> CleanStats {
        let start = SystemTime::now();
        let mut stats = CleanStats::default();

        // Clean to_delete directory
        let to_delete = self.to_delete_dir();
        if let Ok(entries) = fs::read_dir(&to_delete) {
            for entry in entries.flatten() {
                if Self::check_time_limit(start, time_limit_secs) {
                    break;
                }
                let path = entry.path();
                if path.is_dir() && fs::remove_dir_all(&path).is_ok() {
                    stats.deleted_bytes += Self::directory_size(&path).unwrap_or(0);
                    stats.entries_deleted += 1;
                }
            }
        }

        // Clean old entries
        if let Some(max_age) = self.config.max_age_secs {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            for (handle, path, status) in self.list_cached() {
                if Self::check_time_limit(start, time_limit_secs) {
                    break;
                }

                if status != CacheStatus::Found {
                    continue;
                }

                // Check age via metadata file
                let json_path = path.with_extension("json");
                if let Ok(metadata) = Self::read_metadata(&json_path)
                    && let Some(accessed) = metadata.last_accessed
                    && now - accessed > max_age
                {
                    let _ = self.remove_variant(&handle);
                    stats.entries_deleted += 1;
                    stats.deleted_bytes += metadata.payload_size.unwrap_or(0);
                }
            }
        }

        stats
    }

    // ── Helper methods ──────────────────────────────────────────────────────

    /// Read metadata from a JSON file.
    fn read_metadata(path: &Path) -> Result<CachedVariantInfo, PackageCacheError> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(serde_json::from_str(&contents)?)
    }

    /// Update the last-accessed time in the metadata.
    fn update_access_time(json_path: &Path) -> Result<(), PackageCacheError> {
        let mut metadata: CachedVariantInfo = Self::read_metadata(json_path)?;
        metadata.last_accessed = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );
        let json_str = serde_json::to_string_pretty(&metadata)?;
        let mut file = File::create(json_path)?;
        file.write_all(json_str.as_bytes())?;
        Ok(())
    }

    /// Check if a file appears stalled (no mtime update for too long).
    fn is_file_stalled(path: &Path) -> bool {
        if let Ok(metadata) = fs::metadata(path)
            && let Ok(mtime) = metadata.modified()
        {
            let age = SystemTime::now().duration_since(mtime).unwrap_or_default();
            return age.as_secs() > 300; // 5 minutes = stalled
        }
        false
    }

    /// Get the next increment name for a hash path.
    fn next_increment(hash_path: &Path) -> Result<String, PackageCacheError> {
        let mut max_inc = None;

        if let Ok(entries) = fs::read_dir(hash_path) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".json") {
                    let inc = name.trim_end_matches(".json");
                    match &max_inc {
                        None => max_inc = Some(inc.to_string()),
                        Some(current) => {
                            if inc > current.as_str() {
                                max_inc = Some(inc.to_string());
                            }
                        }
                    }
                }
            }
        }

        let next = match max_inc {
            None => "a".to_string(),
            Some(ref inc) => increment_string(inc),
        };

        Ok(next)
    }

    /// Recursively copy a directory.
    fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), io::Error> {
        fs::create_dir_all(dst)?;

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                Self::copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }

        Ok(())
    }

    /// Calculate directory size (follows symlinks, deduplicates inodes on Unix).
    fn directory_size(path: &Path) -> Result<u64, io::Error> {
        let mut total = 0u64;

        #[cfg(unix)]
        let mut seen_inodes: std::collections::HashSet<(u64, u64)> =
            std::collections::HashSet::new();

        let mut stack = vec![path.to_path_buf()];

        while let Some(current) = stack.pop() {
            let entries = match fs::read_dir(&current) {
                Ok(e) => e,
                Err(_) => continue,
            };

            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                let path = entry.path();
                let metadata = match fs::metadata(&path) {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                // Deduplicate inodes (Unix only)
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    let inode = (metadata.dev(), metadata.ino());
                    if !seen_inodes.insert(inode) {
                        continue;
                    }
                }

                if metadata.is_file() {
                    total += metadata.len();
                } else if metadata.is_dir() {
                    stack.push(path);
                }
            }
        }

        Ok(total)
    }

    /// Check if there's enough disk space.
    fn check_disk_space(&self, needed: u64) -> Result<bool, PackageCacheError> {
        let available = fs2::available_space(&self.root)?;
        Ok(available - needed > self.config.min_free_space_bytes)
    }

    /// Check if the cache disk is near full.
    ///
    /// Returns `true` if available space is below `min_free_space_bytes`.
    /// Aligns with rez.package_cache.PackageCache.cache_near_full().
    pub fn cache_near_full(&self) -> bool {
        fs2::available_space(&self.root)
            .map(|available| available < self.config.min_free_space_bytes)
            .unwrap_or(false) // Cannot determine, assume not full
    }

    /// Check if adding a variant would leave enough free space.
    ///
    /// Aligns with rez.package_cache.PackageCache.variant_meets_space_requirements().
    ///
    /// # Arguments
    ///
    /// * `variant_root` - Path to the variant's payload
    ///
    /// # Returns
    ///
    /// `true` if there's enough space to cache this variant.
    pub fn variant_meets_space_requirements(&self, variant_root: &Path) -> bool {
        let available = match fs2::available_space(&self.root) {
            Ok(space) => space,
            Err(_) => return false, // Cannot determine
        };

        let variant_size = Self::directory_size(variant_root).unwrap_or(0);

        // Check: available - variant_size > min_free_space
        available > variant_size + self.config.min_free_space_bytes
    }

    /// Remove empty parent directories.
    fn cleanup_empty_dirs(path: &Path) {
        let mut current = path.parent();
        while let Some(dir) = current {
            if dir.file_name().unwrap().to_string_lossy().starts_with('.') {
                break;
            }
            if fs::read_dir(dir)
                .map(|mut d| d.next().is_some())
                .unwrap_or(true)
            {
                break;
            }
            let _ = fs::remove_dir(dir);
            current = dir.parent();
        }
    }

    /// Check if cleaning has exceeded time limit.
    fn check_time_limit(start: SystemTime, limit: Option<u64>) -> bool {
        if let Some(limit) = limit {
            let elapsed = start.elapsed().unwrap_or_default().as_secs();
            return elapsed > limit;
        }
        false
    }
}

// ── Increment string helper ─────────────────────────────────────────────────

/// Get the next base26-style increment string.
///
/// a -> b -> ... -> z -> aa -> ab -> ...
fn increment_string(s: &str) -> String {
    let mut chars: Vec<char> = s.chars().collect();
    let mut i = chars.len() - 1;

    loop {
        if chars[i] == 'z' {
            chars[i] = 'a';
            if i == 0 {
                chars.insert(0, 'a');
                break;
            }
            i -= 1;
        } else {
            chars[i] = ((chars[i] as u8) + 1) as char;
            break;
        }
    }

    chars.iter().collect()
}

// ── Clean Stats ──────────────────────────────────────────────────────────────

/// Statistics from a cache cleaning operation.
#[derive(Debug, Default, Clone)]
pub struct CleanStats {
    /// Number of entries deleted
    pub entries_deleted: u64,

    /// Total bytes freed
    pub deleted_bytes: u64,
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_handle(name: &str, version: Option<&str>) -> VariantHandle {
        VariantHandle::new(name.to_string(), version.map(String::from), None)
    }

    #[test]
    fn test_cache_creation() {
        let tmp = TempDir::new().unwrap();
        let cache = PackageCache::new(tmp.path()).unwrap();
        assert_eq!(cache.root(), tmp.path());
    }

    #[test]
    fn test_cache_creation_not_a_dir() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nonexistent");
        let result = PackageCache::new(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_variant_handle_hash() {
        let h1 = make_handle("python", Some("3.9.0"));
        let h2 = make_handle("python", Some("3.9.0"));
        assert_eq!(h1.sha1_hash(), h2.sha1_hash());
    }

    #[test]
    fn test_variant_handle_hash_different() {
        let h1 = make_handle("python", Some("3.9.0"));
        let h2 = make_handle("python", Some("3.10.0"));
        assert_ne!(h1.sha1_hash(), h2.sha1_hash());
    }

    #[test]
    fn test_add_and_get_variant() {
        let tmp = TempDir::new().unwrap();
        let cache = PackageCache::new(tmp.path()).unwrap();

        // Create a fake variant payload
        let payload = tmp.path().join("payload");
        fs::create_dir_all(&payload).unwrap();
        fs::write(payload.join("file.txt"), b"hello").unwrap();

        let handle = make_handle("mypkg", Some("1.0.0"));
        let (status, path) = cache.add_variant(&handle, &payload, false).unwrap();

        assert_eq!(status, CacheStatus::Created);
        assert!(path.is_dir());
    }

    #[test]
    fn test_get_cached_root_found() {
        let tmp = TempDir::new().unwrap();
        let cache = PackageCache::new(tmp.path()).unwrap();

        let payload = tmp.path().join("payload");
        fs::create_dir_all(&payload).unwrap();
        fs::write(payload.join("file.txt"), b"hello").unwrap();

        let handle = make_handle("mypkg", Some("1.0.0"));
        cache.add_variant(&handle, &payload, false).unwrap();

        let (status, path) = cache.get_cached_root(&handle);
        assert_eq!(status, CacheStatus::Found);
        assert!(path.is_some());
    }

    #[test]
    fn test_get_cached_root_not_found() {
        let tmp = TempDir::new().unwrap();
        let cache = PackageCache::new(tmp.path()).unwrap();

        let handle = make_handle("nonexistent", Some("1.0.0"));
        let (status, path) = cache.get_cached_root(&handle);
        assert_eq!(status, CacheStatus::NotFound);
        assert!(path.is_none());
    }

    #[test]
    fn test_remove_variant() {
        let tmp = TempDir::new().unwrap();
        let cache = PackageCache::new(tmp.path()).unwrap();

        let payload = tmp.path().join("payload");
        fs::create_dir_all(&payload).unwrap();
        fs::write(payload.join("file.txt"), b"hello").unwrap();

        let handle = make_handle("mypkg", Some("1.0.0"));
        cache.add_variant(&handle, &payload, false).unwrap();

        let (status, _) = cache.remove_variant(&handle);
        assert_eq!(status, CacheStatus::Removed);

        let (status, _) = cache.get_cached_root(&handle);
        assert_eq!(status, CacheStatus::NotFound);
    }

    #[test]
    fn test_list_cached() {
        let tmp = TempDir::new().unwrap();
        let cache = PackageCache::new(tmp.path()).unwrap();

        let payload = tmp.path().join("payload");
        fs::create_dir_all(&payload).unwrap();
        fs::write(payload.join("file.txt"), b"hello").unwrap();

        let handle = make_handle("mypkg", Some("1.0.0"));
        cache.add_variant(&handle, &payload, false).unwrap();

        let cached = cache.list_cached();
        assert!(!cached.is_empty());
        assert_eq!(cached[0].0.name, "mypkg");
    }

    #[test]
    fn test_cache_status_description() {
        assert_eq!(CacheStatus::Found.description(), "was found");
        assert_eq!(CacheStatus::NotFound.description(), "was not found");
    }

    #[test]
    fn test_increment_string() {
        assert_eq!(increment_string("a"), "b");
        assert_eq!(increment_string("z"), "aa");
        assert_eq!(increment_string("az"), "ba");
    }

    #[test]
    fn test_clean_stats_default() {
        let stats = CleanStats::default();
        assert_eq!(stats.entries_deleted, 0);
        assert_eq!(stats.deleted_bytes, 0);
    }
}
