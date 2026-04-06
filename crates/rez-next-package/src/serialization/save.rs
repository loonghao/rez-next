//! Package serialization (save) logic.

use crate::Package;
use chrono::Utc;
use flate2::write::GzEncoder;
use flate2::Compression;
use rez_next_common::RezCoreError;
use std::fs;
use std::io::Write;
use std::path::Path;

use super::types::{PackageContainer, PackageFormat, PackageMetadata, SerializationOptions};

pub struct PackageSaver;

impl PackageSaver {
    /// Save a package to a file with options
    pub fn save_to_file_with_options(
        package: &Package,
        path: &Path,
        format: PackageFormat,
        options: Option<SerializationOptions>,
    ) -> Result<(), RezCoreError> {
        let opts = options.unwrap_or_default();

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
        Self::save_to_python_with_options(package, &SerializationOptions::minimal())
    }

    /// Save container to string with options
    pub fn save_container_to_string(
        container: &PackageContainer,
        format: PackageFormat,
        options: &SerializationOptions,
    ) -> Result<String, RezCoreError> {
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

        if let Some(ref exclude_fields) = options.exclude_fields {
            for field in exclude_fields {
                match field.as_str() {
                    "description" => filtered.description = None,
                    "help" => filtered.help = None,
                    "authors" => filtered.authors.clear(),
                    "tools" => filtered.tools.clear(),
                    _ => {}
                }
            }
        }

        if let Some(ref include_only) = options.include_only {
            let mut new_package = Package::new(filtered.name.clone());

            for field in include_only {
                match field.as_str() {
                    "name" => {}
                    "version" => new_package.version = filtered.version.clone(),
                    "description" => new_package.description = filtered.description.clone(),
                    "authors" => new_package.authors = filtered.authors.clone(),
                    "requires" => new_package.requires = filtered.requires.clone(),
                    "build_requires" => {
                        new_package.build_requires = filtered.build_requires.clone()
                    }
                    "variants" => new_package.variants = filtered.variants.clone(),
                    "tools" => new_package.tools = filtered.tools.clone(),
                    _ => {}
                }
            }

            filtered = new_package;
        }

        Ok(filtered)
    }

    fn save_container_to_yaml(
        container: &PackageContainer,
        _options: &SerializationOptions,
    ) -> Result<String, RezCoreError> {
        serde_yaml::to_string(container).map_err(|e| {
            RezCoreError::PackageParse(format!("Failed to serialize container to YAML: {}", e))
        })
    }

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

    fn save_to_yaml_with_options(
        package: &Package,
        options: &SerializationOptions,
    ) -> Result<String, RezCoreError> {
        let raw = serde_yaml::to_string(package).map_err(|e| {
            RezCoreError::PackageParse(format!("Failed to serialize to YAML: {}", e))
        })?;

        if !options.pretty_print {
            return Ok(raw);
        }

        let mut output = String::with_capacity(raw.len() + 64);
        output.push_str("# ---\n");
        for line in raw.lines() {
            if line.starts_with("- ") {
                output.push_str("  ");
            }
            output.push_str(line);
            output.push('\n');
        }
        Ok(output)
    }

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

    pub(super) fn save_to_python_with_options(
        package: &Package,
        options: &SerializationOptions,
    ) -> Result<String, RezCoreError> {
        let mut content = String::new();

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

    /// Save a package as a base64-wrapped bincode payload.
    fn save_to_binary(package: &Package) -> Result<String, RezCoreError> {
        use base64::Engine as _;

        let binary_data = bincode::serde::encode_to_vec(package, bincode::config::standard())
            .map_err(|e| {
                RezCoreError::PackageParse(format!("Failed to serialize to binary: {}", e))
            })?;

        Ok(base64::engine::general_purpose::STANDARD.encode(binary_data))
    }

    fn save_to_toml(
        package: &Package,
        _options: &SerializationOptions,
    ) -> Result<String, RezCoreError> {
        toml::to_string(package)
            .map_err(|e| RezCoreError::PackageParse(format!("Failed to serialize to TOML: {}", e)))
    }

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

    /// Write compressed file
    pub(super) fn write_compressed_file(
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

/// Convert YAML value to JSON value
pub fn yaml_to_json_value(yaml_value: serde_yaml::Value) -> serde_json::Value {
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
        serde_yaml::Value::Tagged(_) => serde_json::Value::Null,
    }
}
