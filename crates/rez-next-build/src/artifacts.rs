//! Build artifacts management

use rez_next_common::RezCoreError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Build artifacts container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildArtifacts {
    /// Install directory containing built artifacts
    pub install_dir: PathBuf,
    /// List of built files
    pub files: Vec<ArtifactFile>,
    /// Artifact metadata
    pub metadata: HashMap<String, String>,
}

impl Default for BuildArtifacts {
    fn default() -> Self {
        Self {
            install_dir: PathBuf::new(),
            files: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

/// Artifact file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactFile {
    /// Relative path from install directory
    pub path: PathBuf,
    /// File type
    pub file_type: ArtifactFileType,
    /// File size in bytes
    pub size_bytes: u64,
    /// File checksum (SHA256)
    pub checksum: Option<String>,
    /// File permissions (Unix-style)
    pub permissions: Option<u32>,
}

/// Artifact file types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArtifactFileType {
    /// Executable binary
    Executable,
    /// Shared library
    Library,
    /// Static library
    StaticLibrary,
    /// Header file
    Header,
    /// Documentation file
    Documentation,
    /// Configuration file
    Configuration,
    /// Data file
    Data,
    /// Other file type
    Other,
}

impl BuildArtifacts {
    /// Create new build artifacts
    pub fn new(install_dir: PathBuf) -> Self {
        Self {
            install_dir,
            files: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Scan install directory for artifacts
    pub async fn scan_install_dir(&mut self) -> Result<(), RezCoreError> {
        if !self.install_dir.exists() {
            return Ok(());
        }

        self.files.clear();
        let install_dir = self.install_dir.clone();
        self.scan_directory(&install_dir, &PathBuf::new()).await?;

        Ok(())
    }

    /// Recursively scan directory for files
    async fn scan_directory(
        &mut self,
        dir: &Path,
        relative_path: &Path,
    ) -> Result<(), RezCoreError> {
        let mut entries = tokio::fs::read_dir(dir)
            .await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to read directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            RezCoreError::BuildError(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();
            let file_name = entry.file_name();
            let relative_file_path = relative_path.join(&file_name);

            if path.is_dir() {
                // Recursively scan subdirectory
                Box::pin(self.scan_directory(&path, &relative_file_path)).await?;
            } else {
                // Add file to artifacts
                let metadata = tokio::fs::metadata(&path).await.map_err(|e| {
                    RezCoreError::BuildError(format!("Failed to get file metadata: {}", e))
                })?;

                let file_type = Self::determine_file_type(&path);
                let size_bytes = metadata.len();
                let permissions = Self::get_file_permissions(&metadata);
                let checksum = Self::compute_sha256(&path).await.ok();

                let artifact_file = ArtifactFile {
                    path: relative_file_path,
                    file_type,
                    size_bytes,
                    checksum,
                    permissions,
                };

                self.files.push(artifact_file);
            }
        }

        Ok(())
    }

    /// Determine file type based on path and extension
    fn determine_file_type(path: &Path) -> ArtifactFileType {
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            match extension.to_lowercase().as_str() {
                "exe" | "bin" => ArtifactFileType::Executable,
                "dll" | "so" | "dylib" => ArtifactFileType::Library,
                "lib" | "a" => ArtifactFileType::StaticLibrary,
                "h" | "hpp" | "hxx" => ArtifactFileType::Header,
                "md" | "txt" | "rst" | "html" => ArtifactFileType::Documentation,
                "conf" | "cfg" | "ini" | "yaml" | "yml" | "json" => ArtifactFileType::Configuration,
                _ => ArtifactFileType::Data,
            }
        } else {
            // Check if file is executable
            if Self::is_executable(path) {
                ArtifactFileType::Executable
            } else {
                ArtifactFileType::Other
            }
        }
    }

    /// Check if file is executable (simplified)
    fn is_executable(path: &Path) -> bool {
        // On Unix systems, check execute permission
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::metadata(path)
                .map(|m| m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
        }

        // On Windows, check file extension
        #[cfg(windows)]
        {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| matches!(ext.to_lowercase().as_str(), "exe" | "bat" | "cmd" | "com"))
                .unwrap_or(false)
        }

        #[cfg(not(any(unix, windows)))]
        {
            let _ = path;
            false
        }
    }

    /// Get file permissions
    fn get_file_permissions(_metadata: &std::fs::Metadata) -> Option<u32> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            Some(_metadata.mode())
        }

        #[cfg(not(unix))]
        None
    }

    /// Compute SHA-256 checksum of a file; returns lowercase hex string.
    async fn compute_sha256(path: &Path) -> Result<String, RezCoreError> {
        let data = tokio::fs::read(path)
            .await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to read file for checksum: {}", e)))?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        Ok(hex::encode(hasher.finalize()))
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Get files by type
    pub fn get_files_by_type(&self, file_type: ArtifactFileType) -> Vec<&ArtifactFile> {
        self.files
            .iter()
            .filter(|f| f.file_type == file_type)
            .collect()
    }

    /// Get total size of all artifacts
    pub fn get_total_size(&self) -> u64 {
        self.files.iter().map(|f| f.size_bytes).sum()
    }

    /// Get file count
    pub fn get_file_count(&self) -> usize {
        self.files.len()
    }

    /// Check if artifacts are empty
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Get artifact summary
    pub fn get_summary(&self) -> ArtifactSummary {
        let mut executables = 0;
        let mut libraries = 0;
        let mut static_libraries = 0;
        let mut headers = 0;
        let mut documentation = 0;
        let mut configuration = 0;
        let mut data_files = 0;
        let mut other_files = 0;

        for file in &self.files {
            match file.file_type {
                ArtifactFileType::Executable => executables += 1,
                ArtifactFileType::Library => libraries += 1,
                ArtifactFileType::StaticLibrary => static_libraries += 1,
                ArtifactFileType::Header => headers += 1,
                ArtifactFileType::Documentation => documentation += 1,
                ArtifactFileType::Configuration => configuration += 1,
                ArtifactFileType::Data => data_files += 1,
                ArtifactFileType::Other => other_files += 1,
            }
        }

        ArtifactSummary {
            total_files: self.files.len(),
            total_size_bytes: self.get_total_size(),
            executables,
            libraries,
            static_libraries,
            headers,
            documentation,
            configuration,
            data_files,
            other_files,
        }
    }

    /// Validate artifacts
    pub async fn validate(&self) -> Result<ArtifactValidation, RezCoreError> {
        let mut validation = ArtifactValidation::default();

        for file in &self.files {
            let full_path = self.install_dir.join(&file.path);

            if !full_path.exists() {
                validation
                    .errors
                    .push(format!("File does not exist: {}", file.path.display()));
                continue;
            }

            // Check file size
            let metadata = tokio::fs::metadata(&full_path).await.map_err(|e| {
                RezCoreError::BuildError(format!("Failed to get file metadata: {}", e))
            })?;

            if metadata.len() != file.size_bytes {
                validation.warnings.push(format!(
                    "File size mismatch for {}: expected {}, got {}",
                    file.path.display(),
                    file.size_bytes,
                    metadata.len()
                ));
            }

            validation.files_checked += 1;
        }

        validation.is_valid = validation.errors.is_empty();
        Ok(validation)
    }
}

/// Artifact summary
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactSummary {
    /// Total number of files
    pub total_files: usize,
    /// Total size in bytes
    pub total_size_bytes: u64,
    /// Number of executables
    pub executables: usize,
    /// Number of libraries
    pub libraries: usize,
    /// Number of static libraries
    pub static_libraries: usize,
    /// Number of headers
    pub headers: usize,
    /// Number of documentation files
    pub documentation: usize,
    /// Number of configuration files
    pub configuration: usize,
    /// Number of data files
    pub data_files: usize,
    /// Number of other files
    pub other_files: usize,
}

/// Artifact validation result
#[derive(Debug, Clone, Default)]
pub struct ArtifactValidation {
    /// Whether artifacts are valid
    pub is_valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
    /// Number of files checked
    pub files_checked: usize,
}
