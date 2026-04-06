//! Package serialization and deserialization
//!
//! Organized into focused sub-modules:
//! - `types`: data types (PackageFormat, SerializationOptions, PackageMetadata, PackageContainer)
//! - `load`: deserialization (load_from_*)
//! - `save`: serialization (save_to_*)

mod load;
mod save;
pub mod types;

#[cfg(test)]
mod tests;

pub use types::{PackageContainer, PackageFormat, PackageMetadata, SerializationOptions};

use crate::Package;
use rez_next_common::RezCoreError;
use std::path::Path;

use load::PackageLoader;
use save::PackageSaver;

/// Enhanced package serializer/deserializer (public facade).
///
/// Delegates to `PackageLoader` and `PackageSaver` internally.
pub struct PackageSerializer;

impl PackageSerializer {
    // ── Load (deserialize) ────────────────────────────────────────────────────

    /// Load a package from a file with options
    pub fn load_from_file_with_options(
        path: &Path,
        options: Option<SerializationOptions>,
    ) -> Result<PackageContainer, RezCoreError> {
        PackageLoader::load_from_file_with_options(path, options)
    }

    /// Load a package from a file (legacy method)
    pub fn load_from_file(path: &Path) -> Result<Package, RezCoreError> {
        PackageLoader::load_from_file(path)
    }

    /// Load a package from a string
    pub fn load_from_string(content: &str, format: PackageFormat) -> Result<Package, RezCoreError> {
        PackageLoader::load_from_string(content, format)
    }

    /// Load a package from YAML content
    pub fn load_from_yaml(content: &str) -> Result<Package, RezCoreError> {
        PackageLoader::load_from_yaml(content)
    }

    /// Load a package from JSON content
    pub fn load_from_json(content: &str) -> Result<Package, RezCoreError> {
        PackageLoader::load_from_json(content)
    }

    /// Load a package from Python content using advanced AST parsing
    pub fn load_from_python(content: &str) -> Result<Package, RezCoreError> {
        PackageLoader::load_from_python(content)
    }

    /// Load a package from binary content
    pub fn load_from_binary(content: &str) -> Result<Package, RezCoreError> {
        PackageLoader::load_from_binary(content)
    }

    /// Load a package from TOML content
    pub fn load_from_toml(content: &str) -> Result<Package, RezCoreError> {
        PackageLoader::load_from_toml(content)
    }

    /// Load a package from XML content (simplified)
    pub fn load_from_xml(content: &str) -> Result<Package, RezCoreError> {
        PackageLoader::load_from_xml(content)
    }

    // ── Save (serialize) ──────────────────────────────────────────────────────

    /// Save a package to a file with options
    pub fn save_to_file_with_options(
        package: &Package,
        path: &Path,
        format: PackageFormat,
        options: Option<SerializationOptions>,
    ) -> Result<(), RezCoreError> {
        PackageSaver::save_to_file_with_options(package, path, format, options)
    }

    /// Save a package to a file (legacy method)
    pub fn save_to_file(
        package: &Package,
        path: &Path,
        format: PackageFormat,
    ) -> Result<(), RezCoreError> {
        PackageSaver::save_to_file(package, path, format)
    }

    /// Save a package to a string
    pub fn save_to_string(
        package: &Package,
        format: PackageFormat,
    ) -> Result<String, RezCoreError> {
        PackageSaver::save_to_string(package, format)
    }

    /// Save a package to YAML format
    pub fn save_to_yaml(package: &Package) -> Result<String, RezCoreError> {
        PackageSaver::save_to_yaml(package)
    }

    /// Save a package to JSON format
    pub fn save_to_json(package: &Package) -> Result<String, RezCoreError> {
        PackageSaver::save_to_json(package)
    }

    /// Save a package to Python format
    pub fn save_to_python(package: &Package) -> Result<String, RezCoreError> {
        PackageSaver::save_to_python(package)
    }

    /// Save container to string with options
    pub fn save_container_to_string(
        container: &PackageContainer,
        format: PackageFormat,
        options: &SerializationOptions,
    ) -> Result<String, RezCoreError> {
        PackageSaver::save_container_to_string(container, format, options)
    }
}
