//! # Rez Core Package
//!
//! Package system implementation for Rez Core.
//!
//! This crate provides:
//! - Package definition and metadata
//! - Package variants and requirements
//! - Package serialization/deserialization
//! - Package validation
//! - Package management operations

pub mod package;
pub mod python_ast_parser;
pub mod serialization; // Always available for CLI usage // Advanced Python AST parser

#[cfg(feature = "python-bindings")]
pub mod management;
#[cfg(feature = "python-bindings")]
pub mod validation;
#[cfg(feature = "python-bindings")]
pub mod variant;

pub use package::*;
pub use python_ast_parser::*;
pub use serialization::*; // Always available for CLI usage // Advanced Python AST parser

// Always export requirement types for CLI usage
pub mod requirement;
pub use requirement::{Requirement, VersionConstraint};

#[cfg(feature = "python-bindings")]
pub use management::*;
#[cfg(feature = "python-bindings")]
pub use requirement::PackageRequirement as PyPackageRequirement;
#[cfg(feature = "python-bindings")]
pub use validation::*;
#[cfg(feature = "python-bindings")]
pub use variant::*;

#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;

/// Initialize the package module for Python
#[cfg(feature = "python-bindings")]
#[pymodule]
fn rez_core_package(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Package>()?;
    m.add_class::<PackageVariant>()?;
    m.add_class::<PyPackageRequirement>()?;
    m.add_class::<PackageValidator>()?;
    m.add_class::<PackageValidationResult>()?;
    m.add_class::<PackageValidationOptions>()?;
    m.add_class::<PackageManager>()?;
    m.add_class::<PackageInstallOptions>()?;
    m.add_class::<PackageCopyOptions>()?;
    m.add_class::<PackageOperationResult>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rez_core_version::Version;
    use tempfile::TempDir;

    #[test]
    fn test_package_creation() {
        let mut package = Package::new("test_package".to_string());
        package.version = Some(Version::parse("1.0.0").unwrap());
        package.description = Some("Test package description".to_string());
        package.authors = vec!["Test Author".to_string()];

        assert_eq!(package.name, "test_package");
        assert!(package.version.is_some());
        assert!(package.description.is_some());
        assert_eq!(package.authors.len(), 1);
    }

    #[test]
    #[cfg(feature = "python-bindings")]
    fn test_package_validation() {
        let mut package = Package::new("valid_package".to_string());
        package.version = Some(Version::parse("1.0.0").unwrap());
        package.description = Some("Valid test package".to_string());
        package.authors = vec!["Test Author".to_string()];

        let validator = PackageValidator::new(Some(PackageValidationOptions::new()));
        let result = validator.validate_package(&package).unwrap();

        assert!(result.is_valid);
        assert_eq!(result.errors.len(), 0);
        assert!(result.metadata_valid);
        assert!(result.dependencies_valid);
        assert!(result.variants_valid);
    }

    #[test]
    #[cfg(feature = "python-bindings")]
    fn test_package_validation_invalid() {
        let package = Package::new("".to_string()); // Invalid name

        let validator = PackageValidator::new(Some(PackageValidationOptions::new()));
        let result = validator.validate_package(&package).unwrap();

        assert!(!result.is_valid);
        assert!(result.errors.len() > 0);
        assert!(!result.metadata_valid);
    }

    #[test]
    #[cfg(feature = "python-bindings")]
    fn test_package_manager() {
        let mut package = Package::new("test_package".to_string());
        package.version = Some(Version::parse("1.0.0").unwrap());
        package.description = Some("Test package for manager".to_string());
        package.authors = vec!["Test Author".to_string()];

        let manager = PackageManager::new();

        let temp_dir = TempDir::new().unwrap();
        let dest_path = temp_dir.path().to_str().unwrap();

        let mut options = PackageInstallOptions::new();
        options.dry_run = true;
        options.validate = false;

        let result = manager
            .install_package(&package, dest_path, Some(options))
            .unwrap();

        assert!(result.success);
        assert!(result.message.contains("Would install"));
    }

    #[test]
    #[cfg(feature = "python-bindings")]
    fn test_package_copy() {
        let mut package = Package::new("original_package".to_string());
        package.version = Some(Version::parse("1.0.0").unwrap());
        package.description = Some("Original package".to_string());
        package.authors = vec!["Test Author".to_string()];

        let manager = PackageManager::new();

        let temp_dir = TempDir::new().unwrap();
        let dest_path = temp_dir.path().to_str().unwrap();

        let mut options = PackageCopyOptions::new();
        options.set_dest_name("renamed_package".to_string());
        options.install_options.dry_run = true;
        options.install_options.validate = false;

        let result = manager
            .copy_package(&package, dest_path, Some(options))
            .unwrap();

        assert!(result.success);
    }

    #[test]
    #[cfg(feature = "python-bindings")]
    fn test_validation_options() {
        let default_options = PackageValidationOptions::new();
        assert!(default_options.check_metadata);
        assert!(default_options.check_dependencies);
        assert!(!default_options.strict_mode);

        let quick_options = PackageValidationOptions::quick();
        assert!(quick_options.check_metadata);
        assert!(!quick_options.check_dependencies);

        let full_options = PackageValidationOptions::full();
        assert!(full_options.check_metadata);
        assert!(full_options.strict_mode);
    }

    #[test]
    #[cfg(feature = "python-bindings")]
    fn test_install_options() {
        let default_options = PackageInstallOptions::new();
        assert!(!default_options.overwrite);
        assert!(default_options.validate);
        assert!(!default_options.dry_run);

        let quick_options = PackageInstallOptions::quick();
        assert!(quick_options.skip_payload);
        assert!(!quick_options.validate);

        let safe_options = PackageInstallOptions::safe();
        assert!(safe_options.keep_timestamp);
        assert!(safe_options.verbose);
    }
}
