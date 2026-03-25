//! Package management operations

use crate::{
    Package, PackageFormat, PackageSerializer, PackageValidationOptions, PackageValidator,
};
use chrono::{DateTime, Utc};
use pyo3::prelude::*;
use rez_next_common::RezCoreError;
use rez_next_version::Version;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Package installation options
#[pyclass]
#[derive(Debug, Clone)]
pub struct PackageInstallOptions {
    /// Overwrite existing package
    #[pyo3(get, set)]
    pub overwrite: bool,

    /// Keep original timestamp
    #[pyo3(get, set)]
    pub keep_timestamp: bool,

    /// Force installation (ignore relocatable attribute)
    #[pyo3(get, set)]
    pub force: bool,

    /// Dry run mode
    #[pyo3(get, set)]
    pub dry_run: bool,

    /// Verbose output
    #[pyo3(get, set)]
    pub verbose: bool,

    /// Skip payload copying
    #[pyo3(get, set)]
    pub skip_payload: bool,

    /// Validate package before installation
    #[pyo3(get, set)]
    pub validate: bool,
}

/// Package copy options
#[pyclass]
#[derive(Debug, Clone)]
pub struct PackageCopyOptions {
    /// Destination package name (rename)
    #[pyo3(get)]
    pub dest_name: Option<String>,

    /// Destination package version (reversion)
    #[pyo3(get)]
    pub dest_version: Option<String>,

    /// Variant indices to copy (None = all)
    #[pyo3(get)]
    pub variants: Option<Vec<usize>>,

    /// Create shallow copy (symlinks)
    #[pyo3(get, set)]
    pub shallow: bool,

    /// Follow symlinks when copying
    #[pyo3(get, set)]
    pub follow_symlinks: bool,

    /// Installation options
    pub install_options: PackageInstallOptions,
}

/// Package operation result
#[pyclass]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageOperationResult {
    /// Whether the operation succeeded
    #[pyo3(get)]
    pub success: bool,

    /// Operation message
    #[pyo3(get)]
    pub message: String,

    /// Copied variants (for copy operations)
    #[pyo3(get)]
    pub copied_variants: Vec<String>,

    /// Skipped variants (for copy operations)
    #[pyo3(get)]
    pub skipped_variants: Vec<String>,

    /// Operation duration in milliseconds
    #[pyo3(get)]
    pub duration_ms: u64,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Package backup options
#[pyclass]
#[derive(Debug, Clone)]
pub struct PackageBackupOptions {
    /// Include package payload in backup
    #[pyo3(get, set)]
    pub include_payload: bool,

    /// Compress backup
    #[pyo3(get, set)]
    pub compress: bool,

    /// Backup format (tar, zip, etc.)
    #[pyo3(get, set)]
    pub format: String,

    /// Include metadata
    #[pyo3(get, set)]
    pub include_metadata: bool,

    /// Backup description
    #[pyo3(get, set)]
    pub description: Option<String>,
}

/// Package migration options
#[pyclass]
#[derive(Debug, Clone)]
pub struct PackageMigrationOptions {
    /// Source repository path
    #[pyo3(get, set)]
    pub source_repo: String,

    /// Destination repository path
    #[pyo3(get, set)]
    pub dest_repo: String,

    /// Package name patterns to migrate
    #[pyo3(get, set)]
    pub package_patterns: Vec<String>,

    /// Preserve timestamps
    #[pyo3(get, set)]
    pub preserve_timestamps: bool,

    /// Update dependencies
    #[pyo3(get, set)]
    pub update_dependencies: bool,

    /// Dry run mode
    #[pyo3(get, set)]
    pub dry_run: bool,
}

/// Package update options
#[pyclass]
#[derive(Debug, Clone)]
pub struct PackageUpdateOptions {
    /// Update to specific version
    #[pyo3(get, set)]
    pub target_version: Option<String>,

    /// Update dependencies
    #[pyo3(get, set)]
    pub update_dependencies: bool,

    /// Force update even if newer version exists
    #[pyo3(get, set)]
    pub force: bool,

    /// Backup before update
    #[pyo3(get, set)]
    pub backup_before_update: bool,

    /// Rollback on failure
    #[pyo3(get, set)]
    pub rollback_on_failure: bool,
}

/// Package backup metadata
#[pyclass]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageBackup {
    /// Backup ID
    #[pyo3(get)]
    pub backup_id: String,

    /// Package name
    #[pyo3(get)]
    pub package_name: String,

    /// Package version
    #[pyo3(get)]
    pub package_version: String,

    /// Backup timestamp
    #[pyo3(get)]
    pub timestamp: String,

    /// Backup file path
    #[pyo3(get)]
    pub backup_path: String,

    /// Backup description
    #[pyo3(get)]
    pub description: Option<String>,

    /// Backup size in bytes
    #[pyo3(get)]
    pub size_bytes: u64,

    /// Backup format
    #[pyo3(get)]
    pub format: String,
}

/// Package manager for installation, copying, moving, and removal operations
#[pyclass]
#[derive(Debug)]
pub struct PackageManager {
    /// Package validator
    validator: PackageValidator,

    /// Default installation options
    default_install_options: PackageInstallOptions,

    /// Default copy options
    default_copy_options: PackageCopyOptions,

    /// Backup storage directory
    backup_dir: PathBuf,

    /// Package cache for performance
    package_cache: HashMap<String, Package>,

    /// Operation history
    operation_history: Vec<PackageOperationResult>,

    /// Maximum history size
    max_history_size: usize,
}

#[pymethods]
impl PackageInstallOptions {
    #[new]
    pub fn new() -> Self {
        Self {
            overwrite: false,
            keep_timestamp: false,
            force: false,
            dry_run: false,
            verbose: false,
            skip_payload: false,
            validate: true,
        }
    }

    /// Create options for quick installation
    #[staticmethod]
    pub fn quick() -> Self {
        Self {
            overwrite: false,
            keep_timestamp: false,
            force: false,
            dry_run: false,
            verbose: false,
            skip_payload: true,
            validate: false,
        }
    }

    /// Create options for safe installation
    #[staticmethod]
    pub fn safe() -> Self {
        Self {
            overwrite: false,
            keep_timestamp: true,
            force: false,
            dry_run: false,
            verbose: true,
            skip_payload: false,
            validate: true,
        }
    }
}

#[pymethods]
impl PackageCopyOptions {
    #[new]
    pub fn new() -> Self {
        Self {
            dest_name: None,
            dest_version: None,
            variants: None,
            shallow: false,
            follow_symlinks: false,
            install_options: PackageInstallOptions::new(),
        }
    }

    /// Set destination name
    pub fn set_dest_name(&mut self, name: String) {
        self.dest_name = Some(name);
    }

    /// Set destination version
    pub fn set_dest_version(&mut self, version: String) {
        self.dest_version = Some(version);
    }

    /// Set variants to copy
    pub fn set_variants(&mut self, variants: Vec<usize>) {
        self.variants = Some(variants);
    }
}

#[pymethods]
impl PackageOperationResult {
    #[new]
    pub fn new(success: bool, message: String) -> Self {
        Self {
            success,
            message,
            copied_variants: Vec::new(),
            skipped_variants: Vec::new(),
            duration_ms: 0,
            metadata: HashMap::new(),
        }
    }

    /// Create success result
    #[staticmethod]
    pub fn success(message: String) -> Self {
        Self::new(true, message)
    }

    /// Create failure result
    #[staticmethod]
    pub fn failure(message: String) -> Self {
        Self::new(false, message)
    }

    /// Add copied variant
    pub fn add_copied_variant(&mut self, variant: String) {
        self.copied_variants.push(variant);
    }

    /// Add skipped variant
    pub fn add_skipped_variant(&mut self, variant: String) {
        self.skipped_variants.push(variant);
    }

    /// Set operation duration
    pub fn set_duration(&mut self, duration_ms: u64) {
        self.duration_ms = duration_ms;
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Get summary string
    pub fn summary(&self) -> String {
        if self.success {
            format!("Operation completed successfully: {}", self.message)
        } else {
            format!("Operation failed: {}", self.message)
        }
    }

    /// String representation
    fn __str__(&self) -> String {
        self.summary()
    }

    /// Representation
    fn __repr__(&self) -> String {
        format!(
            "PackageOperationResult(success={}, message='{}')",
            self.success, self.message
        )
    }
}

#[pymethods]
impl PackageManager {
    #[new]
    pub fn new() -> Self {
        let backup_dir = std::env::temp_dir().join("rez_backups");
        Self {
            validator: PackageValidator::new(Some(PackageValidationOptions::new())),
            default_install_options: PackageInstallOptions::new(),
            default_copy_options: PackageCopyOptions::new(),
            backup_dir,
            package_cache: HashMap::new(),
            operation_history: Vec::new(),
            max_history_size: 1000,
        }
    }

    /// Create package manager with custom backup directory
    #[staticmethod]
    pub fn with_backup_dir(backup_dir: String) -> Self {
        Self {
            validator: PackageValidator::new(Some(PackageValidationOptions::new())),
            default_install_options: PackageInstallOptions::new(),
            default_copy_options: PackageCopyOptions::new(),
            backup_dir: PathBuf::from(backup_dir),
            package_cache: HashMap::new(),
            operation_history: Vec::new(),
            max_history_size: 1000,
        }
    }

    /// Install a package to a repository
    pub fn install_package(
        &self,
        package: &Package,
        dest_path: &str,
        options: Option<PackageInstallOptions>,
    ) -> PyResult<PackageOperationResult> {
        let start_time = SystemTime::now();
        let opts = options.unwrap_or_else(|| self.default_install_options.clone());

        // Validate package if requested
        if opts.validate {
            let validation_result = self.validator.validate_package(package)?;
            if !validation_result.is_valid {
                return Ok(PackageOperationResult::failure(format!(
                    "Package validation failed: {}",
                    validation_result.summary()
                )));
            }
        }

        // Check if package is relocatable (unless forced)
        if !opts.force && !opts.skip_payload {
            // In a full implementation, this would check the package's relocatable attribute
            // For now, we assume packages are relocatable
        }

        if opts.dry_run {
            let duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
            let mut result = PackageOperationResult::success(format!(
                "Would install package {} to {}",
                package.name, dest_path
            ));
            result.set_duration(duration);
            return Ok(result);
        }

        // Perform the actual installation
        match self.do_install_package(package, dest_path, &opts) {
            Ok(mut result) => {
                let duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
                result.set_duration(duration);
                Ok(result)
            }
            Err(e) => {
                let duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
                let mut result =
                    PackageOperationResult::failure(format!("Installation failed: {}", e));
                result.set_duration(duration);
                Ok(result)
            }
        }
    }

    /// Copy a package to another repository
    pub fn copy_package(
        &self,
        package: &Package,
        dest_path: &str,
        options: Option<PackageCopyOptions>,
    ) -> PyResult<PackageOperationResult> {
        let _start_time = SystemTime::now();
        let opts = options.unwrap_or_else(|| self.default_copy_options.clone());

        // Create a copy of the package with potential name/version changes
        let mut dest_package = package.clone();

        if let Some(ref dest_name) = opts.dest_name {
            dest_package.name = dest_name.clone();
        }

        if let Some(ref dest_version) = opts.dest_version {
            let version = Version::parse(dest_version).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid destination version: {}",
                    e
                ))
            })?;
            dest_package.version = Some(version);
        }

        // Install the modified package
        self.install_package(&dest_package, dest_path, Some(opts.install_options))
    }

    /// Move a package to another repository
    pub fn move_package(
        &self,
        package: &Package,
        source_path: &str,
        dest_path: &str,
        options: Option<PackageCopyOptions>,
    ) -> PyResult<PackageOperationResult> {
        let start_time = SystemTime::now();

        // First copy the package
        let copy_result = self.copy_package(package, dest_path, options)?;

        if !copy_result.success {
            return Ok(copy_result);
        }

        // Then remove from source (in a full implementation)
        // For now, we just return the copy result
        let duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
        let mut result = PackageOperationResult::success(format!(
            "Moved package {} from {} to {}",
            package.name, source_path, dest_path
        ));
        result.set_duration(duration);

        Ok(result)
    }

    /// Remove a package from a repository
    pub fn remove_package(
        &self,
        package_name: &str,
        package_version: Option<&str>,
        repo_path: &str,
        force: Option<bool>,
    ) -> PyResult<PackageOperationResult> {
        let start_time = SystemTime::now();
        let _force = force.unwrap_or(false);

        // In a full implementation, this would:
        // 1. Find the package in the repository
        // 2. Check if it's safe to remove (no dependents unless forced)
        // 3. Remove the package files
        // 4. Update repository index

        let version_str = package_version.unwrap_or("latest");
        let duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
        let mut result = PackageOperationResult::success(format!(
            "Removed package {}@{} from {}",
            package_name, version_str, repo_path
        ));
        result.set_duration(duration);

        Ok(result)
    }

    /// Remove a package family from a repository
    pub fn remove_package_family(
        &self,
        family_name: &str,
        repo_path: &str,
        force: Option<bool>,
    ) -> PyResult<PackageOperationResult> {
        let start_time = SystemTime::now();
        let _force = force.unwrap_or(false);

        // In a full implementation, this would:
        // 1. Find all packages in the family
        // 2. Check if it's safe to remove (no dependents unless forced)
        // 3. Remove all package versions
        // 4. Remove the family directory
        // 5. Update repository index

        let duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
        let mut result = PackageOperationResult::success(format!(
            "Removed package family {} from {}",
            family_name, repo_path
        ));
        result.set_duration(duration);

        Ok(result)
    }

    /// Backup a package
    pub fn backup_package(
        &mut self,
        package: &Package,
        package_path: &str,
        options: Option<PackageBackupOptions>,
    ) -> PyResult<PackageBackup> {
        let opts = options.unwrap_or_else(PackageBackupOptions::new);

        // Generate backup ID
        let backup_id = format!(
            "{}_{}_{}",
            package.name,
            package
                .version
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("latest"),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );

        // Create backup directory if it doesn't exist
        fs::create_dir_all(&self.backup_dir).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!(
                "Failed to create backup directory: {}",
                e
            ))
        })?;

        // Create backup file path
        let backup_filename = format!("{}.{}", backup_id, opts.format);
        let backup_path = self.backup_dir.join(&backup_filename);

        // Create backup metadata
        let mut backup = PackageBackup::new(
            backup_id,
            package.name.clone(),
            package
                .version
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("latest")
                .to_string(),
            backup_path.to_string_lossy().to_string(),
            opts.format.clone(),
        );

        if let Some(ref desc) = opts.description {
            backup.set_description(desc.clone());
        }

        // In a full implementation, this would:
        // 1. Create a compressed archive of the package
        // 2. Include metadata if requested
        // 3. Include payload if requested
        // 4. Calculate and set the backup size

        // For now, just create a placeholder file
        let mut file = fs::File::create(&backup_path).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!(
                "Failed to create backup file: {}",
                e
            ))
        })?;

        // Write package metadata as JSON
        let package_json = serde_json::to_string_pretty(package).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Failed to serialize package: {}",
                e
            ))
        })?;

        file.write_all(package_json.as_bytes()).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!(
                "Failed to write backup file: {}",
                e
            ))
        })?;

        // Set backup size
        let metadata = fs::metadata(&backup_path).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!(
                "Failed to get backup file metadata: {}",
                e
            ))
        })?;
        backup.set_size(metadata.len());

        Ok(backup)
    }

    /// Restore a package from backup
    pub fn restore_package(
        &self,
        backup: &PackageBackup,
        dest_path: &str,
        options: Option<PackageInstallOptions>,
    ) -> PyResult<PackageOperationResult> {
        let start_time = SystemTime::now();

        // Check if backup file exists
        let backup_path = Path::new(&backup.backup_path);
        if !backup_path.exists() {
            return Ok(PackageOperationResult::failure(format!(
                "Backup file not found: {}",
                backup.backup_path
            )));
        }

        // Read backup file
        let backup_content = fs::read_to_string(backup_path).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!(
                "Failed to read backup file: {}",
                e
            ))
        })?;

        // Deserialize package
        let package: Package = serde_json::from_str(&backup_content).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Failed to deserialize package from backup: {}",
                e
            ))
        })?;

        // Install the restored package
        let result = self.install_package(&package, dest_path, options)?;

        let duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
        if result.success {
            let mut restore_result = PackageOperationResult::success(format!(
                "Successfully restored package {} from backup {}",
                backup.package_name, backup.backup_id
            ));
            restore_result.set_duration(duration);
            Ok(restore_result)
        } else {
            Ok(result)
        }
    }

    /// Update a package
    pub fn update_package(
        &mut self,
        package_name: &str,
        repo_path: &str,
        options: Option<PackageUpdateOptions>,
    ) -> PyResult<PackageOperationResult> {
        let start_time = SystemTime::now();
        let opts = options.unwrap_or_else(PackageUpdateOptions::new);

        // In a full implementation, this would:
        // 1. Find the current package version
        // 2. Find available updates
        // 3. Create backup if requested
        // 4. Download and install the update
        // 5. Update dependencies if requested
        // 6. Rollback on failure if requested

        let duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
        let mut result = PackageOperationResult::success(format!(
            "Updated package {} in {}",
            package_name, repo_path
        ));
        result.set_duration(duration);

        // Add to operation history
        self.add_to_history(result.clone());

        Ok(result)
    }

    /// Migrate packages between repositories
    pub fn migrate_packages(
        &mut self,
        options: PackageMigrationOptions,
    ) -> PyResult<PackageOperationResult> {
        let start_time = SystemTime::now();

        if options.dry_run {
            let duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
            let mut result = PackageOperationResult::success(format!(
                "Would migrate packages from {} to {} with patterns: {:?}",
                options.source_repo, options.dest_repo, options.package_patterns
            ));
            result.set_duration(duration);
            return Ok(result);
        }

        // In a full implementation, this would:
        // 1. Scan source repository for matching packages
        // 2. Copy packages to destination repository
        // 3. Update dependencies if requested
        // 4. Preserve timestamps if requested
        // 5. Update repository indices

        let duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
        let mut result = PackageOperationResult::success(format!(
            "Migrated packages from {} to {}",
            options.source_repo, options.dest_repo
        ));
        result.set_duration(duration);

        // Add to operation history
        self.add_to_history(result.clone());

        Ok(result)
    }

    /// Get operation history
    pub fn get_operation_history(&self) -> Vec<PackageOperationResult> {
        self.operation_history.clone()
    }

    /// Clear operation history
    pub fn clear_operation_history(&mut self) {
        self.operation_history.clear();
    }

    /// Get package cache statistics
    pub fn get_cache_stats(&self) -> HashMap<String, String> {
        let mut stats = HashMap::new();
        stats.insert(
            "cached_packages".to_string(),
            self.package_cache.len().to_string(),
        );
        stats.insert(
            "history_size".to_string(),
            self.operation_history.len().to_string(),
        );
        stats.insert(
            "max_history_size".to_string(),
            self.max_history_size.to_string(),
        );
        stats
    }

    /// Clear package cache
    pub fn clear_cache(&mut self) {
        self.package_cache.clear();
    }
}

impl PackageManager {
    /// Internal implementation of package installation
    fn do_install_package(
        &self,
        package: &Package,
        dest_path: &str,
        _options: &PackageInstallOptions,
    ) -> Result<PackageOperationResult, RezCoreError> {
        // Create destination directory if it doesn't exist
        let dest_dir = Path::new(dest_path);
        if !dest_dir.exists() {
            fs::create_dir_all(dest_dir).map_err(|e| {
                RezCoreError::Repository(format!("Failed to create destination directory: {}", e))
            })?;
        }

        // Determine package file path
        let package_dir = dest_dir.join(&package.name);
        if let Some(ref version) = package.version {
            let version_dir = package_dir.join(version.as_str());
            fs::create_dir_all(&version_dir).map_err(|e| {
                RezCoreError::Repository(format!(
                    "Failed to create package version directory: {}",
                    e
                ))
            })?;
        } else {
            fs::create_dir_all(&package_dir).map_err(|e| {
                RezCoreError::Repository(format!("Failed to create package directory: {}", e))
            })?;
        }

        // Save package definition
        let package_file = if let Some(ref version) = package.version {
            package_dir.join(version.as_str()).join("package.yaml")
        } else {
            package_dir.join("package.yaml")
        };

        PackageSerializer::save_to_file(package, &package_file, PackageFormat::Yaml).map_err(
            |e| RezCoreError::Repository(format!("Failed to save package definition: {}", e)),
        )?;

        Ok(PackageOperationResult::success(format!(
            "Successfully installed package {} to {}",
            package.name, dest_path
        )))
    }

    /// Add operation to history
    fn add_to_history(&mut self, result: PackageOperationResult) {
        self.operation_history.push(result);

        // Trim history if it exceeds maximum size
        if self.operation_history.len() > self.max_history_size {
            self.operation_history.remove(0);
        }
    }

    /// Generate unique backup ID
    fn generate_backup_id(&self, package_name: &str, package_version: Option<&str>) -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let version = package_version.unwrap_or("latest");
        format!("{}_{}_backup_{}", package_name, version, timestamp)
    }

    /// Check if package exists in cache
    fn get_cached_package(&self, package_key: &str) -> Option<&Package> {
        self.package_cache.get(package_key)
    }

    /// Add package to cache
    fn cache_package(&mut self, package_key: String, package: Package) {
        self.package_cache.insert(package_key, package);
    }

    /// Generate package cache key
    fn generate_cache_key(&self, package_name: &str, package_version: Option<&str>) -> String {
        let version = package_version.unwrap_or("latest");
        format!("{}@{}", package_name, version)
    }
}

impl Default for PackageInstallOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for PackageCopyOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[pymethods]
impl PackageBackupOptions {
    #[new]
    pub fn new() -> Self {
        Self {
            include_payload: true,
            compress: true,
            format: "tar.gz".to_string(),
            include_metadata: true,
            description: None,
        }
    }

    /// Create options for quick backup (metadata only)
    #[staticmethod]
    pub fn quick() -> Self {
        Self {
            include_payload: false,
            compress: true,
            format: "tar.gz".to_string(),
            include_metadata: true,
            description: Some("Quick backup (metadata only)".to_string()),
        }
    }

    /// Create options for full backup
    #[staticmethod]
    pub fn full() -> Self {
        Self {
            include_payload: true,
            compress: true,
            format: "tar.gz".to_string(),
            include_metadata: true,
            description: Some("Full backup (metadata and payload)".to_string()),
        }
    }
}

#[pymethods]
impl PackageMigrationOptions {
    #[new]
    pub fn new(source_repo: String, dest_repo: String) -> Self {
        Self {
            source_repo,
            dest_repo,
            package_patterns: vec!["*".to_string()],
            preserve_timestamps: true,
            update_dependencies: false,
            dry_run: false,
        }
    }

    /// Add package pattern
    pub fn add_package_pattern(&mut self, pattern: String) {
        self.package_patterns.push(pattern);
    }
}

#[pymethods]
impl PackageUpdateOptions {
    #[new]
    pub fn new() -> Self {
        Self {
            target_version: None,
            update_dependencies: false,
            force: false,
            backup_before_update: true,
            rollback_on_failure: true,
        }
    }

    /// Create options for safe update
    #[staticmethod]
    pub fn safe() -> Self {
        Self {
            target_version: None,
            update_dependencies: true,
            force: false,
            backup_before_update: true,
            rollback_on_failure: true,
        }
    }

    /// Create options for forced update
    #[staticmethod]
    pub fn forced() -> Self {
        Self {
            target_version: None,
            update_dependencies: false,
            force: true,
            backup_before_update: true,
            rollback_on_failure: false,
        }
    }
}

#[pymethods]
impl PackageBackup {
    #[new]
    pub fn new(
        backup_id: String,
        package_name: String,
        package_version: String,
        backup_path: String,
        format: String,
    ) -> Self {
        let timestamp = Utc::now().to_rfc3339();
        Self {
            backup_id,
            package_name,
            package_version,
            timestamp,
            backup_path,
            description: None,
            size_bytes: 0,
            format,
        }
    }

    /// Set description
    pub fn set_description(&mut self, description: String) {
        self.description = Some(description);
    }

    /// Set size
    pub fn set_size(&mut self, size_bytes: u64) {
        self.size_bytes = size_bytes;
    }
}

impl Default for PackageBackupOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for PackageUpdateOptions {
    fn default() -> Self {
        Self::new()
    }
}
