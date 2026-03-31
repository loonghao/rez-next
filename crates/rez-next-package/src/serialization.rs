//! Package serialization and deserialization

use crate::{Package, PythonAstParser};
use chrono::{DateTime, Utc};
use flate2::write::GzEncoder;
use flate2::Compression;
use rez_next_common::RezCoreError;
use rez_next_version::Version;
use serde::{Deserialize, Serialize};
use serde_json;
use serde_yaml;
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

// PackageRequirement and PackageVariant imports removed as they're not used in this module
// PyO3 imports removed as they're not used in this module

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
#[cfg_attr(feature = "python-bindings", pyclass)]
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
#[cfg_attr(feature = "python-bindings", pyclass)]
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

/// Enhanced package serializer/deserializer
pub struct PackageSerializer;

impl PackageSerializer {
    /// Load a package from a file with options
    pub fn load_from_file_with_options(
        path: &Path,
        options: Option<SerializationOptions>,
    ) -> Result<PackageContainer, RezCoreError> {
        let format = PackageFormat::from_extension(path).ok_or_else(|| {
            RezCoreError::PackageParse(format!("Unsupported file format: {}", path.display()))
        })?;

        let content = if format.supports_compression() {
            Self::read_compressed_file(path)?
        } else {
            fs::read_to_string(path).map_err(|e| {
                RezCoreError::PackageParse(format!("Failed to read file {}: {}", path.display(), e))
            })?
        };

        let package = Self::load_from_string(&content, format)?;
        let mut metadata = PackageMetadata::new(format.default_filename().to_string());
        metadata.set_original_path(path.to_string_lossy().to_string());

        Ok(PackageContainer::with_metadata(package, metadata))
    }

    /// Load a package from a file (legacy method)
    pub fn load_from_file(path: &Path) -> Result<Package, RezCoreError> {
        let container = Self::load_from_file_with_options(path, None)?;
        Ok(container.package)
    }

    /// Load a package from a string
    pub fn load_from_string(content: &str, format: PackageFormat) -> Result<Package, RezCoreError> {
        match format {
            PackageFormat::Yaml | PackageFormat::YamlCompressed => Self::load_from_yaml(content),
            PackageFormat::Json | PackageFormat::JsonCompressed => Self::load_from_json(content),
            PackageFormat::Python => Self::load_from_python(content),
            PackageFormat::Binary => Self::load_from_binary(content),
            PackageFormat::Toml => Self::load_from_toml(content),
            PackageFormat::Xml => Self::load_from_xml(content),
        }
    }

    /// Load a package from YAML content
    pub fn load_from_yaml(content: &str) -> Result<Package, RezCoreError> {
        let data: HashMap<String, serde_yaml::Value> = serde_yaml::from_str(content)
            .map_err(|e| RezCoreError::PackageParse(format!("Failed to parse YAML: {}", e)))?;

        Self::load_from_yaml_data(data)
    }

    /// Load a package from JSON content
    pub fn load_from_json(content: &str) -> Result<Package, RezCoreError> {
        let data: HashMap<String, serde_json::Value> = serde_json::from_str(content)
            .map_err(|e| RezCoreError::PackageParse(format!("Failed to parse JSON: {}", e)))?;

        Self::load_from_data(data)
    }

    /// Load a package from Python content using advanced AST parsing
    pub fn load_from_python(content: &str) -> Result<Package, RezCoreError> {
        // Use the advanced Python AST parser for complete Python support
        PythonAstParser::parse_package_py(content)
    }

    /// Load a package from generic data
    fn load_from_data<T>(data: HashMap<String, T>) -> Result<Package, RezCoreError>
    where
        T: Into<serde_json::Value>,
    {
        // Convert to JSON values for easier processing
        let json_data: HashMap<String, serde_json::Value> =
            data.into_iter().map(|(k, v)| (k, v.into())).collect();

        // Extract required name field
        let name = json_data
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                RezCoreError::PackageParse("Missing or invalid 'name' field".to_string())
            })?
            .to_string();

        let mut package = Package::new(name);

        // Extract optional fields
        if let Some(version_value) = json_data.get("version") {
            if let Some(version_str) = version_value.as_str() {
                let version = Version::parse(version_str)
                    .map_err(|e| RezCoreError::PackageParse(format!("Invalid version: {}", e)))?;
                package.version = Some(version);
            }
        }

        if let Some(description_value) = json_data.get("description") {
            if let Some(description) = description_value.as_str() {
                package.description = Some(description.to_string());
            }
        }

        if let Some(authors_value) = json_data.get("authors") {
            if let Some(authors_array) = authors_value.as_array() {
                package.authors = authors_array
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect();
            }
        }

        if let Some(requires_value) = json_data.get("requires") {
            if let Some(requires_array) = requires_value.as_array() {
                package.requires = requires_array
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect();
            }
        }

        if let Some(build_requires_value) = json_data.get("build_requires") {
            if let Some(build_requires_array) = build_requires_value.as_array() {
                package.build_requires = build_requires_array
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect();
            }
        }

        if let Some(private_build_requires_value) = json_data.get("private_build_requires") {
            if let Some(arr) = private_build_requires_value.as_array() {
                package.private_build_requires = arr
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect();
            }
        }

        if let Some(variants_value) = json_data.get("variants") {
            if let Some(variants_array) = variants_value.as_array() {
                package.variants = variants_array
                    .iter()
                    .filter_map(|v| v.as_array())
                    .map(|variant_array| {
                        variant_array
                            .iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| s.to_string())
                            .collect()
                    })
                    .collect();
            }
        }

        if let Some(tools_value) = json_data.get("tools") {
            if let Some(tools_array) = tools_value.as_array() {
                package.tools = tools_array
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect();
            }
        }

        // Validate the package
        package.validate()?;

        Ok(package)
    }

    /// Load a package from YAML data
    fn load_from_yaml_data(
        data: HashMap<String, serde_yaml::Value>,
    ) -> Result<Package, RezCoreError> {
        // Convert YAML values to JSON values for easier processing
        let json_data: HashMap<String, serde_json::Value> = data
            .into_iter()
            .map(|(k, v)| (k, yaml_to_json_value(v)))
            .collect();

        // Extract required name field
        let name = json_data
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                RezCoreError::PackageParse("Missing or invalid 'name' field".to_string())
            })?
            .to_string();

        let mut package = Package::new(name);

        // Extract optional fields (same logic as load_from_data)
        if let Some(version_value) = json_data.get("version") {
            if let Some(version_str) = version_value.as_str() {
                let version = Version::parse(version_str)
                    .map_err(|e| RezCoreError::PackageParse(format!("Invalid version: {}", e)))?;
                package.version = Some(version);
            }
        }

        if let Some(description_value) = json_data.get("description") {
            if let Some(description) = description_value.as_str() {
                package.description = Some(description.to_string());
            }
        }

        if let Some(authors_value) = json_data.get("authors") {
            if let Some(authors_array) = authors_value.as_array() {
                package.authors = authors_array
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect();
            }
        }

        if let Some(requires_value) = json_data.get("requires") {
            if let Some(requires_array) = requires_value.as_array() {
                package.requires = requires_array
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect();
            }
        }

        if let Some(build_requires_value) = json_data.get("build_requires") {
            if let Some(build_requires_array) = build_requires_value.as_array() {
                package.build_requires = build_requires_array
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect();
            }
        }

        if let Some(variants_value) = json_data.get("variants") {
            if let Some(variants_array) = variants_value.as_array() {
                package.variants = variants_array
                    .iter()
                    .filter_map(|v| v.as_array())
                    .map(|variant_array| {
                        variant_array
                            .iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| s.to_string())
                            .collect()
                    })
                    .collect();
            }
        }

        if let Some(tools_value) = json_data.get("tools") {
            if let Some(tools_array) = tools_value.as_array() {
                package.tools = tools_array
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect();
            }
        }

        // Validate the package
        package.validate()?;

        Ok(package)
    }

    /// Save a package to a file with options
    pub fn save_to_file_with_options(
        package: &Package,
        path: &Path,
        format: PackageFormat,
        options: Option<SerializationOptions>,
    ) -> Result<(), RezCoreError> {
        let opts = options.unwrap_or_else(SerializationOptions::new);

        // Create container with metadata if requested
        let container = if opts.include_metadata {
            let mut metadata = PackageMetadata::new(format.default_filename().to_string());
            metadata.set_original_path(path.to_string_lossy().to_string());
            PackageContainer::with_metadata(package.clone(), metadata)
        } else {
            PackageContainer::new(package.clone(), format.default_filename().to_string())
        };

        let content = Self::save_container_to_string(&container, format, &opts)?;

        if format.supports_compression() {
            Self::write_compressed_file(path, &content, opts.compression_level)?;
        } else {
            fs::write(path, content).map_err(|e| {
                RezCoreError::PackageParse(format!(
                    "Failed to write file {}: {}",
                    path.display(),
                    e
                ))
            })?;
        }

        Ok(())
    }

    /// Save a package to a file (legacy method)
    pub fn save_to_file(
        package: &Package,
        path: &Path,
        format: PackageFormat,
    ) -> Result<(), RezCoreError> {
        Self::save_to_file_with_options(package, path, format, None)
    }

    /// Save a package to a string
    pub fn save_to_string(
        package: &Package,
        format: PackageFormat,
    ) -> Result<String, RezCoreError> {
        match format {
            PackageFormat::Yaml | PackageFormat::YamlCompressed => Self::save_to_yaml(package),
            PackageFormat::Json | PackageFormat::JsonCompressed => Self::save_to_json(package),
            PackageFormat::Python => Self::save_to_python(package),
            PackageFormat::Binary => Self::save_to_binary(package),
            PackageFormat::Toml => Self::save_to_toml(package, &SerializationOptions::new()),
            PackageFormat::Xml => Self::save_to_xml(package, &SerializationOptions::new()),
        }
    }

    /// Save a package to YAML format
    pub fn save_to_yaml(package: &Package) -> Result<String, RezCoreError> {
        serde_yaml::to_string(package)
            .map_err(|e| RezCoreError::PackageParse(format!("Failed to serialize to YAML: {}", e)))
    }

    /// Save a package to JSON format
    pub fn save_to_json(package: &Package) -> Result<String, RezCoreError> {
        serde_json::to_string_pretty(package)
            .map_err(|e| RezCoreError::PackageParse(format!("Failed to serialize to JSON: {}", e)))
    }

    /// Save a package to Python format (simplified)
    pub fn save_to_python(package: &Package) -> Result<String, RezCoreError> {
        let mut content = String::new();

        content.push_str(&format!("name = \"{}\"\n", package.name));

        if let Some(ref version) = package.version {
            content.push_str(&format!("version = \"{}\"\n", version.as_str()));
        }

        if let Some(ref description) = package.description {
            content.push_str(&format!("description = \"{}\"\n", description));
        }

        if !package.authors.is_empty() {
            content.push_str("authors = [\n");
            for author in &package.authors {
                content.push_str(&format!("    \"{}\",\n", author));
            }
            content.push_str("]\n");
        }

        if !package.requires.is_empty() {
            content.push_str("requires = [\n");
            for req in &package.requires {
                content.push_str(&format!("    \"{}\",\n", req));
            }
            content.push_str("]\n");
        }

        if !package.build_requires.is_empty() {
            content.push_str("build_requires = [\n");
            for req in &package.build_requires {
                content.push_str(&format!("    \"{}\",\n", req));
            }
            content.push_str("]\n");
        }

        if !package.variants.is_empty() {
            content.push_str("variants = [\n");
            for variant in &package.variants {
                content.push_str("    [");
                for (i, req) in variant.iter().enumerate() {
                    if i > 0 {
                        content.push_str(", ");
                    }
                    content.push_str(&format!("\"{}\"", req));
                }
                content.push_str("],\n");
            }
            content.push_str("]\n");
        }

        if !package.tools.is_empty() {
            content.push_str("tools = [\n");
            for tool in &package.tools {
                content.push_str(&format!("    \"{}\",\n", tool));
            }
            content.push_str("]\n");
        }

        Ok(content)
    }

    /// Load a package from binary content
    pub fn load_from_binary(content: &str) -> Result<Package, RezCoreError> {
        // Decode from base64
        let binary_data = base64::decode(content)
            .map_err(|e| RezCoreError::PackageParse(format!("Failed to decode base64: {}", e)))?;

        // Deserialize from binary
        bincode::deserialize(&binary_data).map_err(|e| {
            RezCoreError::PackageParse(format!("Failed to deserialize from binary: {}", e))
        })
    }

    /// Load a package from TOML content
    pub fn load_from_toml(content: &str) -> Result<Package, RezCoreError> {
        toml::from_str(content)
            .map_err(|e| RezCoreError::PackageParse(format!("Failed to parse TOML: {}", e)))
    }

    /// Load a package from XML content (simplified)
    pub fn load_from_xml(content: &str) -> Result<Package, RezCoreError> {
        // This is a very simplified XML parser
        // In a real implementation, you'd use a proper XML parser like quick-xml

        let name_start = content
            .find("<name>")
            .ok_or_else(|| RezCoreError::PackageParse("Missing <name> tag in XML".to_string()))?;
        let name_end = content
            .find("</name>")
            .ok_or_else(|| RezCoreError::PackageParse("Missing </name> tag in XML".to_string()))?;

        let name = content[name_start + 6..name_end].to_string();
        let mut package = Package::new(name);

        // Extract version if present
        if let (Some(version_start), Some(version_end)) =
            (content.find("<version>"), content.find("</version>"))
        {
            let version_str = &content[version_start + 9..version_end];
            if let Ok(version) = Version::parse(version_str) {
                package.version = Some(version);
            }
        }

        // Extract description if present
        if let (Some(desc_start), Some(desc_end)) = (
            content.find("<description>"),
            content.find("</description>"),
        ) {
            let description = content[desc_start + 13..desc_end].to_string();
            package.description = Some(description);
        }

        Ok(package)
    }
}

/// Convert YAML value to JSON value
fn yaml_to_json_value(yaml_value: serde_yaml::Value) -> serde_json::Value {
    match yaml_value {
        serde_yaml::Value::Null => serde_json::Value::Null,
        serde_yaml::Value::Bool(b) => serde_json::Value::Bool(b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_json::Value::Number(serde_json::Number::from(i))
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            }
        }
        serde_yaml::Value::String(s) => serde_json::Value::String(s),
        serde_yaml::Value::Sequence(seq) => {
            let json_array: Vec<serde_json::Value> =
                seq.into_iter().map(yaml_to_json_value).collect();
            serde_json::Value::Array(json_array)
        }
        serde_yaml::Value::Mapping(map) => {
            let mut json_object = serde_json::Map::new();
            for (k, v) in map {
                if let serde_yaml::Value::String(key) = k {
                    json_object.insert(key, yaml_to_json_value(v));
                }
            }
            serde_json::Value::Object(json_object)
        }
        serde_yaml::Value::Tagged(_) => serde_json::Value::Null, // Ignore tagged values
    }
}

impl PackageSerializer {
    /// Save container to string with options
    pub fn save_container_to_string(
        container: &PackageContainer,
        format: PackageFormat,
        options: &SerializationOptions,
    ) -> Result<String, RezCoreError> {
        // Apply field filters if specified
        let filtered_package = if options.include_only.is_some() || options.exclude_fields.is_some()
        {
            Self::apply_field_filters(&container.package, options)?
        } else {
            container.package.clone()
        };

        match format {
            PackageFormat::Yaml | PackageFormat::YamlCompressed => {
                if options.include_metadata {
                    let container_with_filtered = PackageContainer {
                        package: filtered_package,
                        metadata: container.metadata.clone(),
                        schema_version: container.schema_version.clone(),
                    };
                    Self::save_container_to_yaml(&container_with_filtered, options)
                } else {
                    Self::save_to_yaml_with_options(&filtered_package, options)
                }
            }
            PackageFormat::Json | PackageFormat::JsonCompressed => {
                if options.include_metadata {
                    let container_with_filtered = PackageContainer {
                        package: filtered_package,
                        metadata: container.metadata.clone(),
                        schema_version: container.schema_version.clone(),
                    };
                    Self::save_container_to_json(&container_with_filtered, options)
                } else {
                    Self::save_to_json_with_options(&filtered_package, options)
                }
            }
            PackageFormat::Python => Self::save_to_python_with_options(&filtered_package, options),
            PackageFormat::Binary => Self::save_to_binary(&filtered_package),
            PackageFormat::Toml => Self::save_to_toml(&filtered_package, options),
            PackageFormat::Xml => Self::save_to_xml(&filtered_package, options),
        }
    }

    /// Apply field filters to package
    fn apply_field_filters(
        package: &Package,
        options: &SerializationOptions,
    ) -> Result<Package, RezCoreError> {
        let mut filtered = package.clone();

        // Apply exclude filters
        if let Some(ref exclude_fields) = options.exclude_fields {
            for field in exclude_fields {
                match field.as_str() {
                    "description" => filtered.description = None,
                    "help" => filtered.help = None,
                    "authors" => filtered.authors.clear(),
                    "tools" => filtered.tools.clear(),
                    _ => {} // Ignore unknown fields
                }
            }
        }

        // Apply include only filters
        if let Some(ref include_only) = options.include_only {
            let mut new_package = Package::new(filtered.name.clone());

            for field in include_only {
                match field.as_str() {
                    "name" => {} // Always included
                    "version" => new_package.version = filtered.version.clone(),
                    "description" => new_package.description = filtered.description.clone(),
                    "authors" => new_package.authors = filtered.authors.clone(),
                    "requires" => new_package.requires = filtered.requires.clone(),
                    "build_requires" => {
                        new_package.build_requires = filtered.build_requires.clone()
                    }
                    "variants" => new_package.variants = filtered.variants.clone(),
                    "tools" => new_package.tools = filtered.tools.clone(),
                    _ => {} // Ignore unknown fields
                }
            }

            filtered = new_package;
        }

        Ok(filtered)
    }

    /// Save container to YAML with options
    fn save_container_to_yaml(
        container: &PackageContainer,
        options: &SerializationOptions,
    ) -> Result<String, RezCoreError> {
        if options.pretty_print {
            serde_yaml::to_string(container)
        } else {
            // YAML doesn't have a compact mode, so use regular serialization
            serde_yaml::to_string(container)
        }
        .map_err(|e| {
            RezCoreError::PackageParse(format!("Failed to serialize container to YAML: {}", e))
        })
    }

    /// Save container to JSON with options
    fn save_container_to_json(
        container: &PackageContainer,
        options: &SerializationOptions,
    ) -> Result<String, RezCoreError> {
        if options.pretty_print {
            serde_json::to_string_pretty(container)
        } else {
            serde_json::to_string(container)
        }
        .map_err(|e| {
            RezCoreError::PackageParse(format!("Failed to serialize container to JSON: {}", e))
        })
    }

    /// Save package to YAML with options
    fn save_to_yaml_with_options(
        package: &Package,
        options: &SerializationOptions,
    ) -> Result<String, RezCoreError> {
        if options.pretty_print {
            serde_yaml::to_string(package)
        } else {
            serde_yaml::to_string(package)
        }
        .map_err(|e| RezCoreError::PackageParse(format!("Failed to serialize to YAML: {}", e)))
    }

    /// Save package to JSON with options
    fn save_to_json_with_options(
        package: &Package,
        options: &SerializationOptions,
    ) -> Result<String, RezCoreError> {
        if options.pretty_print {
            serde_json::to_string_pretty(package)
        } else {
            serde_json::to_string(package)
        }
        .map_err(|e| RezCoreError::PackageParse(format!("Failed to serialize to JSON: {}", e)))
    }

    /// Save package to Python with options
    fn save_to_python_with_options(
        package: &Package,
        options: &SerializationOptions,
    ) -> Result<String, RezCoreError> {
        let mut content = String::new();

        // Add header comment if metadata is included
        if options.include_metadata && options.include_timestamps {
            content.push_str(&format!(
                "# Generated by rez-next serializer at {}\n",
                Utc::now().to_rfc3339()
            ));
            content.push_str("# Do not edit manually\n\n");
        }

        content.push_str(&format!("name = \"{}\"\n", package.name));

        if let Some(ref version) = package.version {
            content.push_str(&format!("version = \"{}\"\n", version.as_str()));
        }

        if let Some(ref description) = package.description {
            content.push_str(&format!("description = \"{}\"\n", description));
        }

        if !package.authors.is_empty() {
            content.push_str("authors = [\n");
            for author in &package.authors {
                content.push_str(&format!("    \"{}\",\n", author));
            }
            content.push_str("]\n");
        }

        if !package.requires.is_empty() {
            content.push_str("requires = [\n");
            for req in &package.requires {
                content.push_str(&format!("    \"{}\",\n", req));
            }
            content.push_str("]\n");
        }

        if !package.build_requires.is_empty() {
            content.push_str("build_requires = [\n");
            for req in &package.build_requires {
                content.push_str(&format!("    \"{}\",\n", req));
            }
            content.push_str("]\n");
        }

        if !package.variants.is_empty() {
            content.push_str("variants = [\n");
            for variant in &package.variants {
                content.push_str("    [");
                for (i, req) in variant.iter().enumerate() {
                    if i > 0 {
                        content.push_str(", ");
                    }
                    content.push_str(&format!("\"{}\"", req));
                }
                content.push_str("],\n");
            }
            content.push_str("]\n");
        }

        if !package.tools.is_empty() {
            content.push_str("tools = [\n");
            for tool in &package.tools {
                content.push_str(&format!("    \"{}\",\n", tool));
            }
            content.push_str("]\n");
        }

        Ok(content)
    }

    /// Save package to binary format
    fn save_to_binary(package: &Package) -> Result<String, RezCoreError> {
        // For binary format, we'll use bincode serialization
        let binary_data = bincode::serialize(package).map_err(|e| {
            RezCoreError::PackageParse(format!("Failed to serialize to binary: {}", e))
        })?;

        // Convert to base64 for text representation
        Ok(base64::encode(binary_data))
    }

    /// Save package to TOML format
    fn save_to_toml(
        package: &Package,
        _options: &SerializationOptions,
    ) -> Result<String, RezCoreError> {
        toml::to_string(package)
            .map_err(|e| RezCoreError::PackageParse(format!("Failed to serialize to TOML: {}", e)))
    }

    /// Save package to XML format
    fn save_to_xml(
        package: &Package,
        options: &SerializationOptions,
    ) -> Result<String, RezCoreError> {
        let mut content = String::new();

        if options.pretty_print {
            content.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        }

        content.push_str("<package>\n");
        content.push_str(&format!("  <name>{}</name>\n", package.name));

        if let Some(ref version) = package.version {
            content.push_str(&format!("  <version>{}</version>\n", version.as_str()));
        }

        if let Some(ref description) = package.description {
            content.push_str(&format!("  <description>{}</description>\n", description));
        }

        if !package.authors.is_empty() {
            content.push_str("  <authors>\n");
            for author in &package.authors {
                content.push_str(&format!("    <author>{}</author>\n", author));
            }
            content.push_str("  </authors>\n");
        }

        if !package.requires.is_empty() {
            content.push_str("  <requires>\n");
            for req in &package.requires {
                content.push_str(&format!("    <requirement>{}</requirement>\n", req));
            }
            content.push_str("  </requires>\n");
        }

        content.push_str("</package>\n");

        Ok(content)
    }

    /// Read compressed file
    fn read_compressed_file(path: &Path) -> Result<String, RezCoreError> {
        use flate2::read::GzDecoder;
        use std::io::Read;

        let file = fs::File::open(path).map_err(|e| {
            RezCoreError::PackageParse(format!(
                "Failed to open compressed file {}: {}",
                path.display(),
                e
            ))
        })?;

        let mut decoder = GzDecoder::new(file);
        let mut content = String::new();
        decoder.read_to_string(&mut content).map_err(|e| {
            RezCoreError::PackageParse(format!(
                "Failed to decompress file {}: {}",
                path.display(),
                e
            ))
        })?;

        Ok(content)
    }

    /// Write compressed file
    fn write_compressed_file(
        path: &Path,
        content: &str,
        compression_level: u32,
    ) -> Result<(), RezCoreError> {
        let file = fs::File::create(path).map_err(|e| {
            RezCoreError::PackageParse(format!(
                "Failed to create compressed file {}: {}",
                path.display(),
                e
            ))
        })?;

        let compression = Compression::new(compression_level);
        let mut encoder = GzEncoder::new(file, compression);
        encoder.write_all(content.as_bytes()).map_err(|e| {
            RezCoreError::PackageParse(format!(
                "Failed to write compressed file {}: {}",
                path.display(),
                e
            ))
        })?;

        encoder.finish().map_err(|e| {
            RezCoreError::PackageParse(format!(
                "Failed to finish compressed file {}: {}",
                path.display(),
                e
            ))
        })?;

        Ok(())
    }
}

impl Default for SerializationOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Package;
    use rez_next_version::Version;

    fn make_test_package() -> Package {
        let mut pkg = Package::new("test_pkg".to_string());
        pkg.version = Some(Version::parse("1.2.3").unwrap());
        pkg.description = Some("A test package for serialization".to_string());
        pkg.authors = vec!["Alice".to_string(), "Bob".to_string()];
        pkg.requires = vec!["python>=3.8".to_string(), "maya>=2022".to_string()];
        pkg.tools = vec!["my_tool".to_string()];
        pkg
    }

    #[test]
    fn test_package_format_from_extension_yaml() {
        let p = std::path::Path::new("package.yaml");
        assert_eq!(PackageFormat::from_extension(p), Some(PackageFormat::Yaml));
    }

    #[test]
    fn test_package_format_from_extension_json() {
        let p = std::path::Path::new("package.json");
        assert_eq!(PackageFormat::from_extension(p), Some(PackageFormat::Json));
    }

    #[test]
    fn test_package_format_from_extension_py() {
        let p = std::path::Path::new("package.py");
        assert_eq!(
            PackageFormat::from_extension(p),
            Some(PackageFormat::Python)
        );
    }

    #[test]
    fn test_package_format_default_filename() {
        assert_eq!(PackageFormat::Yaml.default_filename(), "package.yaml");
        assert_eq!(PackageFormat::Json.default_filename(), "package.json");
        assert_eq!(PackageFormat::Python.default_filename(), "package.py");
    }

    #[test]
    fn test_serialization_options_default() {
        let opts = SerializationOptions::default();
        assert!(opts.pretty_print);
        assert!(opts.include_metadata);
    }

    #[test]
    fn test_serialization_options_minimal() {
        let opts = SerializationOptions::minimal();
        assert!(!opts.pretty_print);
        assert!(!opts.include_metadata);
    }

    #[test]
    fn test_serialize_to_yaml_string() {
        let pkg = make_test_package();
        let yaml = PackageSerializer::save_to_yaml(&pkg).unwrap();
        assert!(!yaml.is_empty());
    }

    #[test]
    fn test_serialize_to_json_string() {
        let pkg = make_test_package();
        let json = PackageSerializer::save_to_json(&pkg).unwrap();
        assert!(!json.is_empty());
    }

    #[test]
    fn test_write_yaml_and_read_back() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let yaml_path = tmp.path().join("package.yaml");

        let pkg = make_test_package();
        PackageSerializer::save_to_file(&pkg, &yaml_path, PackageFormat::Yaml).unwrap();

        assert!(yaml_path.exists(), "package.yaml should be written");
        let content = std::fs::read_to_string(&yaml_path).unwrap();
        assert!(!content.is_empty(), "yaml content should not be empty");
    }

    #[test]
    fn test_write_python_package_py() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let py_path = tmp.path().join("package.py");

        let pkg = make_test_package();
        PackageSerializer::save_to_file(&pkg, &py_path, PackageFormat::Python).unwrap();

        assert!(py_path.exists(), "package.py should be written");
        let content = std::fs::read_to_string(&py_path).unwrap();
        assert!(content.contains("name"), "package.py should contain name");
        assert!(
            content.contains("test_pkg"),
            "package.py should contain package name"
        );
    }

    #[test]
    fn test_load_from_yaml_string() {
        let yaml = r#"
name: my_package
version: "2.0.0"
description: My test package
authors:
  - Alice
requires:
  - python>=3.8
"#;
        let pkg = PackageSerializer::load_from_yaml(yaml).unwrap();
        assert_eq!(pkg.name, "my_package");
        assert!(pkg.version.is_some());
        assert_eq!(pkg.version.as_ref().map(|v| v.as_str()), Some("2.0.0"));
        assert_eq!(pkg.description, Some("My test package".to_string()));
    }

    #[test]
    fn test_load_from_python_string() {
        let python = r#"
name = "pytools"
version = "1.0.0"
description = "Python tools package"
requires = ["python>=3.7"]
"#;
        let pkg = PackageSerializer::load_from_python(python).unwrap();
        assert_eq!(pkg.name, "pytools");
        assert_eq!(pkg.version.as_ref().map(|v| v.as_str()), Some("1.0.0"));
        assert_eq!(pkg.description, Some("Python tools package".to_string()));
        assert_eq!(pkg.requires, vec!["python>=3.7"]);
    }

    #[test]
    fn test_yaml_roundtrip() {
        let pkg = make_test_package();
        let yaml_str = PackageSerializer::save_to_yaml(&pkg).unwrap();
        let pkg2 = PackageSerializer::load_from_yaml(&yaml_str).unwrap();
        assert_eq!(pkg.name, pkg2.name);
        assert_eq!(
            pkg.version.as_ref().map(|v| v.as_str()),
            pkg2.version.as_ref().map(|v| v.as_str())
        );
    }

    #[test]
    fn test_package_metadata_creation() {
        let meta = PackageMetadata::new("yaml".to_string());
        assert_eq!(meta.format, "yaml");
        assert!(!meta.serialized_at.is_empty());
    }

    #[test]
    fn test_package_format_mime_type() {
        assert_eq!(PackageFormat::Yaml.mime_type(), "application/x-yaml");
        assert_eq!(PackageFormat::Json.mime_type(), "application/json");
        assert_eq!(PackageFormat::Python.mime_type(), "text/x-python");
    }

    // ── Phase 106: Package YAML save_to_file roundtrip tests ─────────────────

    fn minimal_opts() -> SerializationOptions {
        let mut opts = SerializationOptions::new();
        opts.include_metadata = false;
        opts
    }

    /// Full YAML file roundtrip: write package.yaml, load back, verify all fields
    #[test]
    fn test_yaml_file_roundtrip_all_fields() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let yaml_path = tmp.path().join("package.yaml");

        let mut pkg = Package::new("full_pkg".to_string());
        pkg.version = Some(Version::parse("2.5.0").unwrap());
        pkg.description = Some("Full field test package".to_string());
        pkg.authors = vec!["Dev1".to_string(), "Dev2".to_string()];
        pkg.requires = vec!["python-3.9".to_string(), "maya-2023".to_string()];
        pkg.tools = vec!["tool_a".to_string(), "tool_b".to_string()];

        PackageSerializer::save_to_file_with_options(
            &pkg,
            &yaml_path,
            PackageFormat::Yaml,
            Some(minimal_opts()),
        )
        .unwrap();

        let loaded = PackageSerializer::load_from_file(&yaml_path).unwrap();
        assert_eq!(loaded.name, "full_pkg");
        assert_eq!(loaded.version.as_ref().map(|v| v.as_str()), Some("2.5.0"));
        assert_eq!(
            loaded.description,
            Some("Full field test package".to_string())
        );
        assert!(loaded.authors.contains(&"Dev1".to_string()));
        assert!(loaded.authors.contains(&"Dev2".to_string()));
    }

    /// Package with requires field roundtrip via YAML
    #[test]
    fn test_yaml_file_roundtrip_requires() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let yaml_path = tmp.path().join("package.yaml");

        let mut pkg = Package::new("dep_pkg".to_string());
        pkg.version = Some(Version::parse("1.0.0").unwrap());
        pkg.requires = vec![
            "python-3.9".to_string(),
            "numpy-1.20".to_string(),
            "scipy-1.7".to_string(),
        ];

        PackageSerializer::save_to_file_with_options(
            &pkg,
            &yaml_path,
            PackageFormat::Yaml,
            Some(minimal_opts()),
        )
        .unwrap();
        let loaded = PackageSerializer::load_from_file(&yaml_path).unwrap();

        assert_eq!(loaded.requires.len(), 3);
        assert!(loaded.requires.contains(&"python-3.9".to_string()));
        assert!(loaded.requires.contains(&"numpy-1.20".to_string()));
        assert!(loaded.requires.contains(&"scipy-1.7".to_string()));
    }

    /// JSON file roundtrip preserves name and version
    #[test]
    fn test_json_file_roundtrip_name_version() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let json_path = tmp.path().join("package.json");

        let mut pkg = Package::new("json_pkg".to_string());
        pkg.version = Some(Version::parse("3.1.2").unwrap());
        pkg.description = Some("JSON test".to_string());

        PackageSerializer::save_to_file_with_options(
            &pkg,
            &json_path,
            PackageFormat::Json,
            Some(minimal_opts()),
        )
        .unwrap();
        let loaded = PackageSerializer::load_from_file(&json_path).unwrap();

        assert_eq!(loaded.name, "json_pkg");
        assert_eq!(loaded.version.as_ref().map(|v| v.as_str()), Some("3.1.2"));
    }

    /// Save YAML with load_from_yaml_string equivalence check
    #[test]
    fn test_save_yaml_string_matches_file() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let yaml_path = tmp.path().join("package.yaml");

        let pkg = make_test_package();

        // Save to file (no metadata)
        PackageSerializer::save_to_file_with_options(
            &pkg,
            &yaml_path,
            PackageFormat::Yaml,
            Some(minimal_opts()),
        )
        .unwrap();

        // Save to string
        let yaml_string = PackageSerializer::save_to_yaml(&pkg).unwrap();

        // Both should load to same package
        let from_file = PackageSerializer::load_from_file(&yaml_path).unwrap();
        let from_string = PackageSerializer::load_from_yaml(&yaml_string).unwrap();

        assert_eq!(from_file.name, from_string.name);
        assert_eq!(
            from_file.version.as_ref().map(|v| v.as_str()),
            from_string.version.as_ref().map(|v| v.as_str()),
        );
    }

    /// Package with no optional fields serializes to valid YAML
    #[test]
    fn test_minimal_package_yaml_roundtrip() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let yaml_path = tmp.path().join("package.yaml");

        let pkg = Package::new("minimal_pkg".to_string());
        PackageSerializer::save_to_file_with_options(
            &pkg,
            &yaml_path,
            PackageFormat::Yaml,
            Some(minimal_opts()),
        )
        .unwrap();

        let loaded = PackageSerializer::load_from_file(&yaml_path).unwrap();
        assert_eq!(loaded.name, "minimal_pkg");
        assert!(loaded.version.is_none());
        assert!(loaded.description.is_none());
        assert!(loaded.requires.is_empty());
    }

    /// Format detection from extension is consistent for all formats
    #[test]
    fn test_format_detection_all_extensions() {
        use std::path::Path;
        assert_eq!(
            PackageFormat::from_extension(Path::new("pkg.yaml")),
            Some(PackageFormat::Yaml)
        );
        assert_eq!(
            PackageFormat::from_extension(Path::new("pkg.yml")),
            Some(PackageFormat::Yaml)
        );
        assert_eq!(
            PackageFormat::from_extension(Path::new("pkg.json")),
            Some(PackageFormat::Json)
        );
        assert_eq!(
            PackageFormat::from_extension(Path::new("pkg.py")),
            Some(PackageFormat::Python)
        );
        assert_eq!(
            PackageFormat::from_extension(Path::new("pkg.bin")),
            Some(PackageFormat::Binary)
        );
        assert_eq!(
            PackageFormat::from_extension(Path::new("pkg.toml")),
            Some(PackageFormat::Toml)
        );
        assert_eq!(PackageFormat::from_extension(Path::new("pkg.xyz")), None);
    }

    // ── Phase 112: build_requires / private_build_requires / variants tests ──

    /// build_requires field is serialized and deserialized correctly via JSON
    #[test]
    fn test_build_requires_json_roundtrip() {
        let mut pkg = Package::new("build_pkg".to_string());
        pkg.version = Some(Version::parse("1.0.0").unwrap());
        pkg.build_requires = vec!["cmake-3.20".to_string(), "ninja-1.10".to_string()];

        let json = PackageSerializer::save_to_json(&pkg).unwrap();
        assert!(
            json.contains("cmake-3.20"),
            "JSON should have cmake in build_requires"
        );
        let loaded = PackageSerializer::load_from_json(&json).unwrap();
        assert_eq!(loaded.build_requires.len(), 2);
        assert!(loaded.build_requires.contains(&"cmake-3.20".to_string()));
        assert!(loaded.build_requires.contains(&"ninja-1.10".to_string()));
    }

    /// build_requires field is serialized and deserialized correctly via YAML
    #[test]
    fn test_build_requires_yaml_roundtrip() {
        let mut pkg = Package::new("yaml_build_pkg".to_string());
        pkg.version = Some(Version::parse("2.0.0").unwrap());
        pkg.build_requires = vec!["gcc-11".to_string(), "python-3.9".to_string()];

        let yaml = PackageSerializer::save_to_yaml(&pkg).unwrap();
        assert!(yaml.contains("gcc-11"), "YAML should have build_requires");
        let loaded = PackageSerializer::load_from_yaml(&yaml).unwrap();
        assert_eq!(loaded.build_requires.len(), 2);
    }

    /// private_build_requires field preserved in JSON roundtrip
    #[test]
    fn test_private_build_requires_json_roundtrip() {
        let mut pkg = Package::new("private_build_pkg".to_string());
        pkg.private_build_requires = vec!["internal_lib-1.0".to_string()];

        let json = PackageSerializer::save_to_json(&pkg).unwrap();
        let loaded = PackageSerializer::load_from_json(&json).unwrap();
        assert_eq!(loaded.private_build_requires.len(), 1);
        assert!(loaded
            .private_build_requires
            .contains(&"internal_lib-1.0".to_string()));
    }

    /// build_requires empty by default
    #[test]
    fn test_build_requires_empty_by_default() {
        let pkg = Package::new("default_pkg".to_string());
        assert!(pkg.build_requires.is_empty());
        assert!(pkg.private_build_requires.is_empty());
    }

    /// add_build_requirement appends to build_requires
    #[test]
    fn test_add_build_requirement() {
        let mut pkg = Package::new("add_req_pkg".to_string());
        pkg.add_build_requirement("cmake-3.25".to_string());
        pkg.add_build_requirement("make-4.3".to_string());
        assert_eq!(pkg.build_requires.len(), 2);
        assert!(pkg.build_requires.contains(&"cmake-3.25".to_string()));
    }

    /// save_to_python includes build_requires in output
    #[test]
    fn test_save_to_python_includes_build_requires() {
        let mut pkg = Package::new("py_build_pkg".to_string());
        pkg.version = Some(Version::parse("1.0.0").unwrap());
        pkg.build_requires = vec!["cmake-3".to_string()];

        let py = PackageSerializer::save_to_python(&pkg).unwrap();
        assert!(
            py.contains("build_requires"),
            "Python output should have build_requires"
        );
        assert!(py.contains("cmake-3"), "Python output should list cmake-3");
    }

    /// YAML file roundtrip preserves build_requires
    #[test]
    fn test_yaml_file_roundtrip_build_requires() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let yaml_path = tmp.path().join("build_pkg.yaml");

        let mut pkg = Package::new("file_build_pkg".to_string());
        pkg.version = Some(Version::parse("1.5.0").unwrap());
        pkg.build_requires = vec!["cmake-3.20".to_string(), "boost-1.80".to_string()];

        PackageSerializer::save_to_file_with_options(
            &pkg,
            &yaml_path,
            PackageFormat::Yaml,
            Some(minimal_opts()),
        )
        .unwrap();
        let loaded = PackageSerializer::load_from_file(&yaml_path).unwrap();

        assert_eq!(
            loaded.build_requires.len(),
            2,
            "build_requires should be preserved in YAML file"
        );
        assert!(loaded.build_requires.contains(&"cmake-3.20".to_string()));
        assert!(loaded.build_requires.contains(&"boost-1.80".to_string()));
    }

    /// Package with both requires and build_requires: both preserved
    #[test]
    fn test_both_requires_and_build_requires() {
        let mut pkg = Package::new("combo_pkg".to_string());
        pkg.requires = vec!["python-3.9".to_string()];
        pkg.build_requires = vec!["cmake-3.20".to_string()];

        let json = PackageSerializer::save_to_json(&pkg).unwrap();
        let loaded = PackageSerializer::load_from_json(&json).unwrap();

        assert_eq!(loaded.requires, vec!["python-3.9".to_string()]);
        assert_eq!(loaded.build_requires, vec!["cmake-3.20".to_string()]);
    }
}
