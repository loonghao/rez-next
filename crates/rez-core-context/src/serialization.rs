//! Context serialization and deserialization

use crate::ResolvedContext;
use rez_core_common::RezCoreError;
use serde_json;
use std::path::Path;
use base64::{Engine as _, engine::general_purpose};

/// Context serialization format
#[derive(Debug, Clone, PartialEq)]
pub enum ContextFormat {
    /// JSON format (.rxt)
    Json,
    /// Binary format (future)
    Binary,
}

impl ContextFormat {
    /// Get the file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            ContextFormat::Json => "rxt",
            ContextFormat::Binary => "rxtb",
        }
    }

    /// Detect format from file extension
    pub fn from_extension(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "rxt" => Some(ContextFormat::Json),
            "rxtb" => Some(ContextFormat::Binary),
            _ => None,
        }
    }
}

/// Context serializer/deserializer
pub struct ContextSerializer;

impl ContextSerializer {
    /// Save a resolved context to a file
    pub async fn save_to_file(
        context: &ResolvedContext,
        path: &Path,
        format: ContextFormat,
    ) -> Result<(), RezCoreError> {
        let content = Self::serialize(context, format)?;
        
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| RezCoreError::ContextError(
                    format!("Failed to create directory {}: {}", parent.display(), e)
                ))?;
        }

        tokio::fs::write(path, content).await
            .map_err(|e| RezCoreError::ContextError(
                format!("Failed to write context file {}: {}", path.display(), e)
            ))
    }

    /// Load a resolved context from a file
    pub async fn load_from_file(path: &Path) -> Result<ResolvedContext, RezCoreError> {
        let format = ContextFormat::from_extension(path)
            .ok_or_else(|| RezCoreError::ContextError(
                format!("Unsupported context file format: {}", path.display())
            ))?;

        let content = tokio::fs::read(path).await
            .map_err(|e| RezCoreError::ContextError(
                format!("Failed to read context file {}: {}", path.display(), e)
            ))?;

        Self::deserialize(&content, format)
    }

    /// Serialize a context to bytes
    pub fn serialize(context: &ResolvedContext, format: ContextFormat) -> Result<Vec<u8>, RezCoreError> {
        match format {
            ContextFormat::Json => {
                let json_str = serde_json::to_string_pretty(context)
                    .map_err(|e| RezCoreError::ContextError(
                        format!("Failed to serialize context to JSON: {}", e)
                    ))?;
                Ok(json_str.into_bytes())
            }
            ContextFormat::Binary => {
                // For now, use JSON as binary format (could be replaced with bincode or similar)
                let json_str = serde_json::to_string(context)
                    .map_err(|e| RezCoreError::ContextError(
                        format!("Failed to serialize context to binary: {}", e)
                    ))?;
                Ok(json_str.into_bytes())
            }
        }
    }

    /// Deserialize a context from bytes
    pub fn deserialize(content: &[u8], format: ContextFormat) -> Result<ResolvedContext, RezCoreError> {
        match format {
            ContextFormat::Json | ContextFormat::Binary => {
                let json_str = String::from_utf8(content.to_vec())
                    .map_err(|e| RezCoreError::ContextError(
                        format!("Invalid UTF-8 in context file: {}", e)
                    ))?;

                serde_json::from_str(&json_str)
                    .map_err(|e| RezCoreError::ContextError(
                        format!("Failed to deserialize context: {}", e)
                    ))
            }
        }
    }

    /// Save context to string
    pub fn to_string(context: &ResolvedContext, format: ContextFormat) -> Result<String, RezCoreError> {
        match format {
            ContextFormat::Json => {
                serde_json::to_string_pretty(context)
                    .map_err(|e| RezCoreError::ContextError(
                        format!("Failed to serialize context to JSON string: {}", e)
                    ))
            }
            ContextFormat::Binary => {
                // For binary format, return base64 encoded string
                let bytes = Self::serialize(context, format)?;
                Ok(general_purpose::STANDARD.encode(bytes))
            }
        }
    }

    /// Load context from string
    pub fn from_string(content: &str, format: ContextFormat) -> Result<ResolvedContext, RezCoreError> {
        match format {
            ContextFormat::Json => {
                serde_json::from_str(content)
                    .map_err(|e| RezCoreError::ContextError(
                        format!("Failed to deserialize context from JSON string: {}", e)
                    ))
            }
            ContextFormat::Binary => {
                // For binary format, expect base64 encoded string
                let bytes = general_purpose::STANDARD.decode(content)
                    .map_err(|e| RezCoreError::ContextError(
                        format!("Failed to decode base64 context: {}", e)
                    ))?;
                Self::deserialize(&bytes, format)
            }
        }
    }

    /// Export context to various formats
    pub fn export_context(
        context: &ResolvedContext,
        export_format: ExportFormat,
    ) -> Result<String, RezCoreError> {
        match export_format {
            ExportFormat::Json => Self::to_string(context, ContextFormat::Json),
            ExportFormat::Yaml => Self::export_to_yaml(context),
            ExportFormat::Env => Self::export_to_env_file(context),
            ExportFormat::Shell(shell_type) => Self::export_to_shell_script(context, shell_type),
        }
    }

    /// Export context to YAML format
    fn export_to_yaml(context: &ResolvedContext) -> Result<String, RezCoreError> {
        serde_yaml::to_string(context)
            .map_err(|e| RezCoreError::ContextError(
                format!("Failed to export context to YAML: {}", e)
            ))
    }

    /// Export context to environment file format
    fn export_to_env_file(context: &ResolvedContext) -> Result<String, RezCoreError> {
        let mut env_content = String::new();
        env_content.push_str("# Generated by rez-core\n");
        env_content.push_str(&format!("# Context: {}\n", context.id));
        if let Some(ref name) = context.name {
            env_content.push_str(&format!("# Name: {}\n", name));
        }
        env_content.push_str("\n");

        for (name, value) in &context.environment_vars {
            env_content.push_str(&format!("{}={}\n", name, value));
        }

        Ok(env_content)
    }

    /// Export context to shell script format
    fn export_to_shell_script(
        context: &ResolvedContext,
        shell_type: crate::ShellType,
    ) -> Result<String, RezCoreError> {
        let env_manager = crate::EnvironmentManager::new(crate::ContextConfig {
            shell_type,
            ..Default::default()
        });

        env_manager.generate_shell_script(&context.environment_vars)
    }

    /// Validate a context file
    pub async fn validate_file(path: &Path) -> Result<ContextValidation, RezCoreError> {
        let format = ContextFormat::from_extension(path)
            .ok_or_else(|| RezCoreError::ContextError(
                format!("Unsupported context file format: {}", path.display())
            ))?;

        let content = tokio::fs::read(path).await
            .map_err(|e| RezCoreError::ContextError(
                format!("Failed to read context file {}: {}", path.display(), e)
            ))?;

        let validation_start = std::time::Instant::now();
        
        match Self::deserialize(&content, format) {
            Ok(context) => {
                let validation_time = validation_start.elapsed().as_millis() as u64;
                
                // Additional validation
                match context.validate() {
                    Ok(_) => Ok(ContextValidation {
                        is_valid: true,
                        errors: Vec::new(),
                        warnings: Vec::new(),
                        context_id: Some(context.id),
                        package_count: context.resolved_packages.len(),
                        validation_time_ms: validation_time,
                    }),
                    Err(e) => Ok(ContextValidation {
                        is_valid: false,
                        errors: vec![e.to_string()],
                        warnings: Vec::new(),
                        context_id: Some(context.id),
                        package_count: context.resolved_packages.len(),
                        validation_time_ms: validation_time,
                    }),
                }
            }
            Err(e) => {
                let validation_time = validation_start.elapsed().as_millis() as u64;
                Ok(ContextValidation {
                    is_valid: false,
                    errors: vec![e.to_string()],
                    warnings: Vec::new(),
                    context_id: None,
                    package_count: 0,
                    validation_time_ms: validation_time,
                })
            }
        }
    }
}

/// Export format options
#[derive(Debug, Clone, PartialEq)]
pub enum ExportFormat {
    /// JSON format
    Json,
    /// YAML format
    Yaml,
    /// Environment file format
    Env,
    /// Shell script format
    Shell(crate::ShellType),
}

/// Context validation result
#[derive(Debug, Clone)]
pub struct ContextValidation {
    /// Whether the context is valid
    pub is_valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
    /// Context ID (if parseable)
    pub context_id: Option<String>,
    /// Number of packages (if parseable)
    pub package_count: usize,
    /// Validation time in milliseconds
    pub validation_time_ms: u64,
}

impl ContextValidation {
    /// Check if there are any issues
    pub fn has_issues(&self) -> bool {
        !self.errors.is_empty() || !self.warnings.is_empty()
    }

    /// Get total issue count
    pub fn issue_count(&self) -> usize {
        self.errors.len() + self.warnings.len()
    }
}

/// Context file utilities
pub struct ContextFileUtils;

impl ContextFileUtils {
    /// Get the default context file name for a context
    pub fn get_default_filename(context: &ResolvedContext) -> String {
        match &context.name {
            Some(name) => format!("{}.rxt", name.replace(" ", "_").to_lowercase()),
            None => format!("context_{}.rxt", &context.id[..8]),
        }
    }

    /// Check if a path is a valid context file
    pub fn is_context_file(path: &Path) -> bool {
        ContextFormat::from_extension(path).is_some()
    }

    /// Get context file metadata
    pub async fn get_file_metadata(path: &Path) -> Result<ContextFileMetadata, RezCoreError> {
        let metadata = tokio::fs::metadata(path).await
            .map_err(|e| RezCoreError::ContextError(
                format!("Failed to get file metadata: {}", e)
            ))?;

        let format = ContextFormat::from_extension(path);
        
        Ok(ContextFileMetadata {
            path: path.to_path_buf(),
            format,
            size_bytes: metadata.len(),
            modified_time: metadata.modified()
                .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())
                .unwrap_or(0),
        })
    }

    /// Find all context files in a directory
    pub async fn find_context_files(dir: &Path) -> Result<Vec<std::path::PathBuf>, RezCoreError> {
        let mut context_files = Vec::new();
        let mut entries = tokio::fs::read_dir(dir).await
            .map_err(|e| RezCoreError::ContextError(
                format!("Failed to read directory {}: {}", dir.display(), e)
            ))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| RezCoreError::ContextError(
                format!("Failed to read directory entry: {}", e)
            ))? {
            
            let path = entry.path();
            if path.is_file() && Self::is_context_file(&path) {
                context_files.push(path);
            }
        }

        context_files.sort();
        Ok(context_files)
    }
}

/// Context file metadata
#[derive(Debug, Clone)]
pub struct ContextFileMetadata {
    /// File path
    pub path: std::path::PathBuf,
    /// Context format
    pub format: Option<ContextFormat>,
    /// File size in bytes
    pub size_bytes: u64,
    /// Last modified time (Unix timestamp)
    pub modified_time: u64,
}
