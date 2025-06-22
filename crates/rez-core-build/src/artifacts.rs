//! Build artifacts management

use rez_core_common::RezCoreError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

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
    async fn scan_directory(&mut self, dir: &PathBuf, relative_path: &PathBuf) -> Result<(), RezCoreError> {
        let mut entries = tokio::fs::read_dir(dir).await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to read directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to read directory entry: {}", e)))? {
            
            let path = entry.path();
            let file_name = entry.file_name();
            let relative_file_path = relative_path.join(&file_name);

            if path.is_dir() {
                // Recursively scan subdirectory
                Box::pin(self.scan_directory(&path, &relative_file_path)).await?;
            } else {
                // Add file to artifacts
                let metadata = tokio::fs::metadata(&path).await
                    .map_err(|e| RezCoreError::BuildError(format!("Failed to get file metadata: {}", e)))?;

                let file_type = Self::determine_file_type(&path);
                let size_bytes = metadata.len();
                let permissions = Self::get_file_permissions(&metadata);

                let artifact_file = ArtifactFile {
                    path: relative_file_path,
                    file_type,
                    size_bytes,
                    checksum: None, // TODO: Calculate checksum if needed
                    permissions,
                };

                self.files.push(artifact_file);
            }
        }

        Ok(())
    }

    /// Determine file type based on path and extension
    fn determine_file_type(path: &PathBuf) -> ArtifactFileType {
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
    fn is_executable(path: &PathBuf) -> bool {
        // On Unix systems, check execute permission
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(path) {
                let permissions = metadata.permissions();
                return permissions.mode() & 0o111 != 0;
            }
        }

        // On Windows, check file extension
        #[cfg(windows)]
        {
            if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
                matches!(extension.to_lowercase().as_str(), "exe" | "bat" | "cmd" | "com")
            } else {
                false
            }
        }

        #[cfg(not(any(unix, windows)))]
        false
    }

    /// Get file permissions
    fn get_file_permissions(metadata: &std::fs::Metadata) -> Option<u32> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            Some(metadata.mode())
        }

        #[cfg(not(unix))]
        None
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Get files by type
    pub fn get_files_by_type(&self, file_type: ArtifactFileType) -> Vec<&ArtifactFile> {
        self.files.iter().filter(|f| f.file_type == file_type).collect()
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
        let mut summary = ArtifactSummary::default();
        summary.total_files = self.files.len();
        summary.total_size_bytes = self.get_total_size();

        for file in &self.files {
            match file.file_type {
                ArtifactFileType::Executable => summary.executables += 1,
                ArtifactFileType::Library => summary.libraries += 1,
                ArtifactFileType::StaticLibrary => summary.static_libraries += 1,
                ArtifactFileType::Header => summary.headers += 1,
                ArtifactFileType::Documentation => summary.documentation += 1,
                ArtifactFileType::Configuration => summary.configuration += 1,
                ArtifactFileType::Data => summary.data_files += 1,
                ArtifactFileType::Other => summary.other_files += 1,
            }
        }

        summary
    }

    /// Validate artifacts
    pub async fn validate(&self) -> Result<ArtifactValidation, RezCoreError> {
        let mut validation = ArtifactValidation::default();

        for file in &self.files {
            let full_path = self.install_dir.join(&file.path);
            
            if !full_path.exists() {
                validation.errors.push(format!("File does not exist: {}", file.path.display()));
                continue;
            }

            // Check file size
            let metadata = tokio::fs::metadata(&full_path).await
                .map_err(|e| RezCoreError::BuildError(format!("Failed to get file metadata: {}", e)))?;

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
