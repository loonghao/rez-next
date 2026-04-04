//! Package serialization types: PackageFormat, SerializationOptions, PackageMetadata, PackageContainer

use crate::Package;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Package serialization format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageFormat {
    /// YAML format (package.yaml)
    Yaml,
    /// JSON format (package.json)
    Json,
    /// Python format (package.py)
    Python,
    /// Compressed YAML format (package.yaml.gz)
    YamlCompressed,
    /// Compressed JSON format (package.json.gz)
    JsonCompressed,
    /// Binary format (package.bin)
    Binary,
    /// TOML format (package.toml)
    Toml,
    /// XML format (package.xml)
    Xml,
}

/// Serialization options
#[derive(Debug, Clone)]
pub struct SerializationOptions {
    /// Pretty print output
    pub pretty_print: bool,
    /// Include metadata
    pub include_metadata: bool,
    /// Include timestamps
    pub include_timestamps: bool,
    /// Compression level (0-9, only for compressed formats)
    pub compression_level: u32,
    /// Custom field filters
    pub field_filters: Vec<String>,
    /// Include only specified fields
    pub include_only: Option<Vec<String>>,
    /// Exclude specified fields
    pub exclude_fields: Option<Vec<String>>,
    /// Custom serialization rules
    pub custom_rules: HashMap<String, String>,
}

/// Package metadata for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    /// Serialization timestamp
    pub serialized_at: String,
    /// Serialization format
    pub format: String,
    /// Serializer version
    pub serializer_version: String,
    /// Original file path
    pub original_path: Option<String>,
    /// Checksum
    pub checksum: Option<String>,
    /// Custom metadata
    pub custom: HashMap<String, String>,
}

/// Enhanced package container with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageContainer {
    /// Package data
    pub package: Package,
    /// Metadata
    pub metadata: PackageMetadata,
    /// Schema version
    pub schema_version: String,
}

// ── PackageFormat impl ────────────────────────────────────────────────────────

impl PackageFormat {
    /// Detect format from file extension
    pub fn from_extension(path: &Path) -> Option<Self> {
        let path_str = path.to_string_lossy();

        if path_str.ends_with(".yaml.gz") || path_str.ends_with(".yml.gz") {
            return Some(Self::YamlCompressed);
        }
        if path_str.ends_with(".json.gz") {
            return Some(Self::JsonCompressed);
        }

        match path.extension()?.to_str()? {
            "yaml" | "yml" => Some(Self::Yaml),
            "json" => Some(Self::Json),
            "py" => Some(Self::Python),
            "bin" => Some(Self::Binary),
            "toml" => Some(Self::Toml),
            "xml" => Some(Self::Xml),
            _ => None,
        }
    }

    /// Get the default file name for this format
    pub fn default_filename(&self) -> &'static str {
        match self {
            Self::Yaml => "package.yaml",
            Self::Json => "package.json",
            Self::Python => "package.py",
            Self::YamlCompressed => "package.yaml.gz",
            Self::JsonCompressed => "package.json.gz",
            Self::Binary => "package.bin",
            Self::Toml => "package.toml",
            Self::Xml => "package.xml",
        }
    }

    /// Check if format supports compression
    pub fn supports_compression(&self) -> bool {
        matches!(self, Self::YamlCompressed | Self::JsonCompressed)
    }

    /// Check if format is text-based
    pub fn is_text_format(&self) -> bool {
        !matches!(self, Self::Binary)
    }

    /// Get MIME type for the format
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Yaml | Self::YamlCompressed => "application/x-yaml",
            Self::Json | Self::JsonCompressed => "application/json",
            Self::Python => "text/x-python",
            Self::Binary => "application/octet-stream",
            Self::Toml => "application/toml",
            Self::Xml => "application/xml",
        }
    }
}

// ── SerializationOptions impl ─────────────────────────────────────────────────

impl SerializationOptions {
    /// Create default serialization options
    pub fn new() -> Self {
        Self {
            pretty_print: true,
            include_metadata: true,
            include_timestamps: true,
            compression_level: 6,
            field_filters: Vec::new(),
            include_only: None,
            exclude_fields: None,
            custom_rules: HashMap::new(),
        }
    }

    /// Create minimal serialization options
    pub fn minimal() -> Self {
        Self {
            pretty_print: false,
            include_metadata: false,
            include_timestamps: false,
            compression_level: 1,
            field_filters: Vec::new(),
            include_only: None,
            exclude_fields: None,
            custom_rules: HashMap::new(),
        }
    }

    /// Create compact serialization options
    pub fn compact() -> Self {
        Self {
            pretty_print: false,
            include_metadata: true,
            include_timestamps: false,
            compression_level: 9,
            field_filters: Vec::new(),
            include_only: None,
            exclude_fields: Some(vec!["description".to_string(), "help".to_string()]),
            custom_rules: HashMap::new(),
        }
    }

    /// Add field filter
    pub fn add_field_filter(&mut self, filter: String) {
        self.field_filters.push(filter);
    }

    /// Set include only fields
    pub fn set_include_only(&mut self, fields: Vec<String>) {
        self.include_only = Some(fields);
    }

    /// Set exclude fields
    pub fn set_exclude_fields(&mut self, fields: Vec<String>) {
        self.exclude_fields = Some(fields);
    }

    /// Add custom rule
    pub fn add_custom_rule(&mut self, field: String, rule: String) {
        self.custom_rules.insert(field, rule);
    }
}

impl Default for SerializationOptions {
    fn default() -> Self {
        Self::new()
    }
}

// ── PackageMetadata impl ──────────────────────────────────────────────────────

impl PackageMetadata {
    /// Create new metadata
    pub fn new(format: String) -> Self {
        Self {
            serialized_at: Utc::now().to_rfc3339(),
            format,
            serializer_version: env!("CARGO_PKG_VERSION").to_string(),
            original_path: None,
            checksum: None,
            custom: HashMap::new(),
        }
    }

    /// Set original path
    pub fn set_original_path(&mut self, path: String) {
        self.original_path = Some(path);
    }

    /// Set checksum
    pub fn set_checksum(&mut self, checksum: String) {
        self.checksum = Some(checksum);
    }

    /// Add custom metadata
    pub fn add_custom(&mut self, key: String, value: String) {
        self.custom.insert(key, value);
    }
}

// ── PackageContainer impl ─────────────────────────────────────────────────────

impl PackageContainer {
    /// Create new container
    pub fn new(package: Package, format: String) -> Self {
        Self {
            package,
            metadata: PackageMetadata::new(format),
            schema_version: "1.0".to_string(),
        }
    }

    /// Create container with metadata
    pub fn with_metadata(package: Package, metadata: PackageMetadata) -> Self {
        Self {
            package,
            metadata,
            schema_version: "1.0".to_string(),
        }
    }
}
