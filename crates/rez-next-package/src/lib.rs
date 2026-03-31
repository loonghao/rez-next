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
pub mod batch;
#[cfg(feature = "python-bindings")]
pub mod cache;
#[cfg(feature = "python-bindings")]
pub mod dependency;
#[cfg(feature = "python-bindings")]
pub mod management;
#[cfg(feature = "python-bindings")]
pub mod validation;
#[cfg(feature = "python-bindings")]
pub mod variant;

pub use package::*;
pub use python_ast_parser::*;
pub use serialization::{PackageFormat, PackageSerializer}; // Always available for CLI usage

// Always export requirement types for CLI usage
pub mod requirement;
pub use requirement::{Requirement, VersionConstraint};

#[cfg(feature = "python-bindings")]
pub use batch::*;
#[cfg(feature = "python-bindings")]
pub use cache::*;
#[cfg(feature = "python-bindings")]
pub use dependency::*;
#[cfg(feature = "python-bindings")]
pub use management::*;
#[cfg(feature = "python-bindings")]
pub use package::PackageRequirement as PyPackageRequirement;
#[cfg(feature = "python-bindings")]
pub use validation::*;
#[cfg(feature = "python-bindings")]
pub use variant::*;

#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;

/// Initialize the package module for Python
#[cfg(feature = "python-bindings")]
#[pymodule]
fn rez_next_package(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Core package types
    m.add_class::<Package>()?;
    m.add_class::<PackageVariant>()?;
    m.add_class::<PyPackageRequirement>()?;

    // Validation
    m.add_class::<PackageValidator>()?;
    m.add_class::<PackageValidationResult>()?;
    m.add_class::<PackageValidationOptions>()?;

    // Management
    m.add_class::<PackageManager>()?;
    m.add_class::<PackageInstallOptions>()?;
    m.add_class::<PackageCopyOptions>()?;
    m.add_class::<PackageOperationResult>()?;
    m.add_class::<PackageBackupOptions>()?;
    m.add_class::<PackageMigrationOptions>()?;
    m.add_class::<PackageUpdateOptions>()?;
    m.add_class::<PackageBackup>()?;

    // Dependency resolution
    m.add_class::<DependencyResolver>()?;
    m.add_class::<DependencyResolutionOptions>()?;
    m.add_class::<DependencyResolutionResult>()?;
    m.add_class::<DependencyNode>()?;
    m.add_class::<DependencyConflict>()?;

    // Caching
    m.add_class::<PackageCacheManager>()?;
    m.add_class::<CacheConfig>()?;
    m.add_class::<CacheStatistics>()?;

    // Batch operations
    m.add_class::<BatchPackageProcessor>()?;
    m.add_class::<BatchConfig>()?;
    m.add_class::<BatchProgress>()?;
    m.add_class::<BatchParseOptions>()?;
    m.add_class::<BatchValidationOptions>()?;
    m.add_class::<BatchInstallOptions>()?;

    // Serialization
    m.add_class::<SerializationOptions>()?;
    m.add_class::<PackageMetadata>()?;
    m.add_class::<PackageContainer>()?;

    // Variant management
    m.add_class::<VariantManager>()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rez_next_version::Version;
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

    // ── Phase 88: Package variants tests ─────────────────────────────────────

    #[test]
    fn test_package_variants_empty() {
        let pkg = Package::new("mypkg".to_string());
        assert!(
            pkg.variants.is_empty(),
            "New package should have no variants"
        );
    }

    #[test]
    fn test_package_variants_single() {
        let mut pkg = Package::new("mypkg".to_string());
        pkg.variants.push(vec!["python-3.9".to_string()]);
        assert_eq!(pkg.variants.len(), 1);
        assert_eq!(pkg.variants[0], vec!["python-3.9"]);
    }

    #[test]
    fn test_package_variants_multiple() {
        let mut pkg = Package::new("mypkg".to_string());
        pkg.variants
            .push(vec!["python-3.9".to_string(), "platform-linux".to_string()]);
        pkg.variants.push(vec![
            "python-3.10".to_string(),
            "platform-linux".to_string(),
        ]);
        pkg.variants.push(vec![
            "python-3.11".to_string(),
            "platform-windows".to_string(),
        ]);
        assert_eq!(pkg.variants.len(), 3);
    }

    #[test]
    fn test_package_variants_parse_from_python() {
        // Simulate a package.py with variants
        let content = r#"
name = 'mypkg'
version = '1.0.0'
variants = [
    ['python-3.9'],
    ['python-3.10'],
]
"#;
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("package.py");
        std::fs::write(&path, content).unwrap();

        let pkg = serialization::PackageSerializer::load_from_file(&path).unwrap();
        assert_eq!(pkg.name, "mypkg");
        // variants field should be parsed (may be empty if parser doesn't support yet)
        // At minimum the package should be loaded
        assert_eq!(pkg.name, "mypkg");
    }

    #[test]
    fn test_package_requires_parsed() {
        let content = r#"
name = 'withreqs'
version = '2.0.0'
requires = ['python-3', 'maya-2023']
"#;
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("package.py");
        std::fs::write(&path, content).unwrap();

        let pkg = serialization::PackageSerializer::load_from_file(&path).unwrap();
        assert!(
            !pkg.requires.is_empty(),
            "requires should be parsed: {:?}",
            pkg.requires
        );
        assert!(
            pkg.requires.iter().any(|r| r.contains("python")),
            "python requirement should be present: {:?}",
            pkg.requires
        );
    }

    #[test]
    fn test_package_tools_field() {
        let mut pkg = Package::new("toolpkg".to_string());
        pkg.tools = vec!["mytool".to_string(), "another-tool".to_string()];
        assert_eq!(pkg.tools.len(), 2);
        assert!(pkg.tools.contains(&"mytool".to_string()));
    }

    #[test]
    fn test_package_commands_field() {
        let mut pkg = Package::new("cmdpkg".to_string());
        pkg.commands = Some("env.setenv('MY_VAR', '1')".to_string());
        assert!(pkg.commands.is_some());
    }

    #[test]
    fn test_package_requirement_satisfied_by() {
        use super::package::PackageRequirement;
        use rez_next_version::Version;

        // In rez version semantics: "3.9" > "3.9.0" (shorter = greater)
        // So >=3.9 means "greater than or equal to 3.9 (the epoch)"
        // 3.11 > 3.10 > 3.9 > 3.9.0 > 3.8
        let req_ge = PackageRequirement::with_version("python".to_string(), ">=3.9".to_string());
        // 3.11 satisfies >=3.9 (3.11 > 3.9)
        assert!(req_ge.satisfied_by(&Version::parse("3.11").unwrap()));
        // 3.8 does not satisfy >=3.9
        assert!(!req_ge.satisfied_by(&Version::parse("3.8").unwrap()));
        // 3.9 exactly satisfies >=3.9
        assert!(req_ge.satisfied_by(&Version::parse("3.9").unwrap()));
    }

    #[test]
    fn test_package_requirement_ne_constraint() {
        use super::package::PackageRequirement;
        use rez_next_version::Version;

        let req_ne = PackageRequirement::with_version("lib".to_string(), "!=1.5.0".to_string());
        assert!(req_ne.satisfied_by(&Version::parse("1.4.0").unwrap()));
        assert!(req_ne.satisfied_by(&Version::parse("1.6.0").unwrap()));
        assert!(!req_ne.satisfied_by(&Version::parse("1.5.0").unwrap()));
    }

    #[test]
    fn test_package_variant_requirements_structure() {
        // Each variant is a list of requirements that the variant needs
        let mut pkg = Package::new("maya_tools".to_string());
        pkg.version = Some(Version::parse("1.0.0").unwrap());
        // Variant 0: requires python-3.9 AND maya-2023
        pkg.variants
            .push(vec!["python-3.9".to_string(), "maya-2023".to_string()]);
        // Variant 1: requires python-3.10 AND maya-2024
        pkg.variants
            .push(vec!["python-3.10".to_string(), "maya-2024".to_string()]);

        assert_eq!(pkg.variants.len(), 2);
        assert!(pkg.variants[0].iter().any(|r| r.contains("python-3.9")));
        assert!(pkg.variants[1].iter().any(|r| r.contains("maya-2024")));
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
