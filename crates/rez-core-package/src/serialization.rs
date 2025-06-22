//! Package serialization and deserialization

use crate::{Package, PythonAstParser};
use rez_core_common::RezCoreError;
use rez_core_version::Version;
use serde_json;
use serde_yaml;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[cfg(feature = "python-bindings")]
use crate::{PackageRequirement, PackageVariant};
#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;

/// Package serialization format
#[derive(Debug, Clone)]
pub enum PackageFormat {
    /// YAML format (package.yaml)
    Yaml,
    /// JSON format (package.json)
    Json,
    /// Python format (package.py)
    Python,
}

impl PackageFormat {
    /// Detect format from file extension
    pub fn from_extension(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "yaml" | "yml" => Some(Self::Yaml),
            "json" => Some(Self::Json),
            "py" => Some(Self::Python),
            _ => None,
        }
    }

    /// Get the default file name for this format
    pub fn default_filename(&self) -> &'static str {
        match self {
            Self::Yaml => "package.yaml",
            Self::Json => "package.json",
            Self::Python => "package.py",
        }
    }
}

/// Package serializer/deserializer
pub struct PackageSerializer;

impl PackageSerializer {
    /// Load a package from a file
    pub fn load_from_file(path: &Path) -> Result<Package, RezCoreError> {
        let format = PackageFormat::from_extension(path).ok_or_else(|| {
            RezCoreError::PackageParse(format!("Unsupported file format: {}", path.display()))
        })?;

        let content = fs::read_to_string(path).map_err(|e| {
            RezCoreError::PackageParse(format!("Failed to read file {}: {}", path.display(), e))
        })?;

        Self::load_from_string(&content, format)
    }

    /// Load a package from a string
    pub fn load_from_string(content: &str, format: PackageFormat) -> Result<Package, RezCoreError> {
        match format {
            PackageFormat::Yaml => Self::load_from_yaml(content),
            PackageFormat::Json => Self::load_from_json(content),
            PackageFormat::Python => Self::load_from_python(content),
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

    /// Save a package to a file
    pub fn save_to_file(
        package: &Package,
        path: &Path,
        format: PackageFormat,
    ) -> Result<(), RezCoreError> {
        let content = Self::save_to_string(package, format)?;

        fs::write(path, content).map_err(|e| {
            RezCoreError::PackageParse(format!("Failed to write file {}: {}", path.display(), e))
        })
    }

    /// Save a package to a string
    pub fn save_to_string(
        package: &Package,
        format: PackageFormat,
    ) -> Result<String, RezCoreError> {
        match format {
            PackageFormat::Yaml => Self::save_to_yaml(package),
            PackageFormat::Json => Self::save_to_json(package),
            PackageFormat::Python => Self::save_to_python(package),
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
