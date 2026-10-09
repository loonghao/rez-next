//! Package deserialization (load) logic.

use std::fs;
use std::path::Path;

use rez_next_common::RezCoreError;
use rez_next_version::Version;

use crate::{Package, PythonAstParser};

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

        // Special handling for binary format: read JSON bytes from file and deserialize
        if format == PackageFormat::Binary {
            let binary_data = fs::read(path).map_err(|e| {
                RezCoreError::PackageParse(format!(
                    "Failed to read binary file {}: {}",
                    path.display(),
                    e
                ))
            })?;
            let json_str = String::from_utf8_lossy(&binary_data);
            let package: Package = serde_json::from_str(&json_str).map_err(|e| {
                RezCoreError::PackageParse(format!(
                    "Failed to deserialize from binary (JSON): {}",
                    e
                ))
            })?;
            let mut metadata = PackageMetadata::new(format.default_filename().to_string());
            metadata.set_original_path(path.to_string_lossy().to_string());
            return Ok(PackageContainer::with_metadata(package, metadata));
        }

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
        // Preserve scalar types instead of coercing numeric requirements to strings.
        let document: serde_yaml::Value = serde_yaml::from_str(content)
            .map_err(|e| RezCoreError::PackageParse(format!("Failed to parse YAML: {}", e)))?;
        let package: Package = serde_yaml::from_value(document)
            .map_err(|e| RezCoreError::PackageParse(format!("Failed to parse YAML: {}", e)))?;
        package.validate()?;
        Ok(package)
    }

    /// Load a package from JSON content
    pub fn load_from_json(content: &str) -> Result<Package, RezCoreError> {
        let package: Package = serde_json::from_str(content)
            .map_err(|e| RezCoreError::PackageParse(format!("Failed to parse JSON: {}", e)))?;
        package.validate()?;
        Ok(package)
    }

    /// Load a package from Python content using advanced AST parsing
    pub fn load_from_python(content: &str) -> Result<Package, RezCoreError> {
        PythonAstParser::parse_package_py(content)
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
