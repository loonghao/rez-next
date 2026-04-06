//! Package deserialization (load) logic.

use crate::{Package, PythonAstParser};
use rez_next_common::RezCoreError;
use rez_next_version::Version;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::types::{PackageContainer, PackageFormat, PackageMetadata, SerializationOptions};

pub struct PackageLoader;

impl PackageLoader {
    /// Load a package from a file with options
    pub fn load_from_file_with_options(
        path: &Path,
        _options: Option<SerializationOptions>,
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
        PythonAstParser::parse_package_py(content)
    }

    /// Load a package from generic data
    fn load_from_data<T>(data: HashMap<String, T>) -> Result<Package, RezCoreError>
    where
        T: Into<serde_json::Value>,
    {
        let json_data: HashMap<String, serde_json::Value> =
            data.into_iter().map(|(k, v)| (k, v.into())).collect();
        Self::load_from_json_data(json_data)
    }

    /// Load a package from YAML data
    pub(super) fn load_from_yaml_data(
        data: HashMap<String, serde_yaml::Value>,
    ) -> Result<Package, RezCoreError> {
        let json_data: HashMap<String, serde_json::Value> = data
            .into_iter()
            .map(|(k, v)| (k, super::save::yaml_to_json_value(v)))
            .collect();
        Self::load_from_json_data(json_data)
    }

    /// Shared implementation: build a Package from a JSON-compatible HashMap
    fn load_from_json_data(
        json_data: HashMap<String, serde_json::Value>,
    ) -> Result<Package, RezCoreError> {
        let name = json_data
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                RezCoreError::PackageParse("Missing or invalid 'name' field".to_string())
            })?
            .to_string();

        let mut package = Package::new(name);

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

        package.validate()?;

        Ok(package)
    }

    /// Load a package from base64-wrapped bincode content.
    pub fn load_from_binary(content: &str) -> Result<Package, RezCoreError> {
        use base64::Engine as _;

        let binary_data = base64::engine::general_purpose::STANDARD
            .decode(content)
            .map_err(|e| RezCoreError::PackageParse(format!("Failed to decode base64: {}", e)))?;

        let (package, _) =
            bincode::serde::decode_from_slice(&binary_data, bincode::config::standard()).map_err(
                |e| RezCoreError::PackageParse(format!("Failed to deserialize from binary: {}", e)),
            )?;
        Ok(package)
    }

    /// Load a package from TOML content
    pub fn load_from_toml(content: &str) -> Result<Package, RezCoreError> {
        toml::from_str(content)
            .map_err(|e| RezCoreError::PackageParse(format!("Failed to parse TOML: {}", e)))
    }

    /// Load a package from XML content (simplified)
    pub fn load_from_xml(content: &str) -> Result<Package, RezCoreError> {
        let name_start = content
            .find("<name>")
            .ok_or_else(|| RezCoreError::PackageParse("Missing <name> tag in XML".to_string()))?;
        let name_end = content
            .find("</name>")
            .ok_or_else(|| RezCoreError::PackageParse("Missing </name> tag in XML".to_string()))?;

        let name = content[name_start + 6..name_end].to_string();
        let mut package = Package::new(name);

        if let (Some(version_start), Some(version_end)) =
            (content.find("<version>"), content.find("</version>"))
        {
            let version_str = &content[version_start + 9..version_end];
            if let Ok(version) = Version::parse(version_str) {
                package.version = Some(version);
            }
        }

        if let (Some(desc_start), Some(desc_end)) = (
            content.find("<description>"),
            content.find("</description>"),
        ) {
            let description = content[desc_start + 13..desc_end].to_string();
            package.description = Some(description);
        }

        Ok(package)
    }

    /// Read compressed file
    pub(super) fn read_compressed_file(path: &Path) -> Result<String, RezCoreError> {
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
}
