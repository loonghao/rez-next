//! Package management tests

use rez_core_package::{
    Package, PackageManager, PackageValidator, PackageValidationOptions,
    PackageInstallOptions, PackageCopyOptions, PackageOperationResult
};
use rez_core_version::Version;
use std::collections::HashMap;
use tempfile::TempDir;

#[test]
fn test_package_validation() {
    // Create a test package
    let mut package = Package::new("test_package".to_string());
    package.version = Some(Version::parse("1.0.0").unwrap());
    package.description = Some("Test package description".to_string());
    package.authors = vec!["Test Author".to_string()];
    
    // Create validator
    let validator = PackageValidator::new(Some(PackageValidationOptions::new()));
    
    // Validate the package
    let result = validator.validate_package(&package).unwrap();
    
    assert!(result.is_valid);
    assert_eq!(result.errors.len(), 0);
    assert!(result.metadata_valid);
    assert!(result.dependencies_valid);
    assert!(result.variants_valid);
}

#[test]
fn test_package_validation_invalid_name() {
    // Create a package with invalid name
    let mut package = Package::new("".to_string()); // Empty name
    package.version = Some(Version::parse("1.0.0").unwrap());
    
    let validator = PackageValidator::new(Some(PackageValidationOptions::new()));
    let result = validator.validate_package(&package).unwrap();
    
    assert!(!result.is_valid);
    assert!(result.errors.len() > 0);
    assert!(!result.metadata_valid);
}

#[test]
fn test_package_validation_with_requirements() {
    let mut package = Package::new("test_package".to_string());
    package.version = Some(Version::parse("1.0.0").unwrap());
    package.requires = vec!["python".to_string(), "numpy>=1.0".to_string()];
    package.build_requires = vec!["cmake".to_string()];
    
    let validator = PackageValidator::new(Some(PackageValidationOptions::new()));
    let result = validator.validate_package(&package).unwrap();
    
    assert!(result.is_valid);
    assert!(result.dependencies_valid);
}

#[test]
fn test_package_validation_with_variants() {
    let mut package = Package::new("test_package".to_string());
    package.version = Some(Version::parse("1.0.0").unwrap());
    package.variants = vec![
        vec!["python-3.8".to_string()],
        vec!["python-3.9".to_string()],
        vec!["python-3.8".to_string(), "numpy".to_string()],
    ];
    
    let validator = PackageValidator::new(Some(PackageValidationOptions::new()));
    let result = validator.validate_package(&package).unwrap();
    
    assert!(result.is_valid);
    assert!(result.variants_valid);
}

#[test]
fn test_package_validation_duplicate_variants() {
    let mut package = Package::new("test_package".to_string());
    package.version = Some(Version::parse("1.0.0").unwrap());
    package.variants = vec![
        vec!["python-3.8".to_string()],
        vec!["python-3.8".to_string()], // Duplicate
    ];
    
    let validator = PackageValidator::new(Some(PackageValidationOptions::new()));
    let result = validator.validate_package(&package).unwrap();
    
    assert!(!result.is_valid);
    assert!(!result.variants_valid);
    assert!(result.errors.iter().any(|e| e.contains("Duplicate variant")));
}

#[test]
fn test_package_manager_creation() {
    let manager = PackageManager::new();
    // Just test that we can create a manager without errors
    assert!(true);
}

#[test]
fn test_package_install_dry_run() {
    let mut package = Package::new("test_package".to_string());
    package.version = Some(Version::parse("1.0.0").unwrap());
    package.description = Some("Test package".to_string());
    
    let manager = PackageManager::new();
    let mut options = PackageInstallOptions::new();
    options.dry_run = true;
    
    let temp_dir = TempDir::new().unwrap();
    let dest_path = temp_dir.path().to_str().unwrap();
    
    let result = manager.install_package(&package, dest_path, Some(options)).unwrap();
    
    assert!(result.success);
    assert!(result.message.contains("Would install"));
}

#[test]
fn test_package_install_validation_failure() {
    let package = Package::new("".to_string()); // Invalid name
    
    let manager = PackageManager::new();
    let options = PackageInstallOptions::new(); // validation enabled by default
    
    let temp_dir = TempDir::new().unwrap();
    let dest_path = temp_dir.path().to_str().unwrap();
    
    let result = manager.install_package(&package, dest_path, Some(options)).unwrap();
    
    assert!(!result.success);
    assert!(result.message.contains("validation failed"));
}

#[test]
fn test_package_install_skip_validation() {
    let package = Package::new("".to_string()); // Invalid name
    
    let manager = PackageManager::new();
    let mut options = PackageInstallOptions::new();
    options.validate = false; // Skip validation
    options.dry_run = true; // Don't actually install
    
    let temp_dir = TempDir::new().unwrap();
    let dest_path = temp_dir.path().to_str().unwrap();
    
    let result = manager.install_package(&package, dest_path, Some(options)).unwrap();
    
    assert!(result.success); // Should succeed because validation is skipped
}

#[test]
fn test_package_copy_with_rename() {
    let mut package = Package::new("original_package".to_string());
    package.version = Some(Version::parse("1.0.0").unwrap());
    package.description = Some("Original package".to_string());
    
    let manager = PackageManager::new();
    let mut options = PackageCopyOptions::new();
    options.dest_name = Some("renamed_package".to_string());
    options.install_options.dry_run = true;
    options.install_options.validate = false;
    
    let temp_dir = TempDir::new().unwrap();
    let dest_path = temp_dir.path().to_str().unwrap();
    
    let result = manager.copy_package(&package, dest_path, Some(options)).unwrap();
    
    assert!(result.success);
}

#[test]
fn test_package_copy_with_reversion() {
    let mut package = Package::new("test_package".to_string());
    package.version = Some(Version::parse("1.0.0").unwrap());
    package.description = Some("Test package".to_string());
    
    let manager = PackageManager::new();
    let mut options = PackageCopyOptions::new();
    options.dest_version = Some("2.0.0".to_string());
    options.install_options.dry_run = true;
    options.install_options.validate = false;
    
    let temp_dir = TempDir::new().unwrap();
    let dest_path = temp_dir.path().to_str().unwrap();
    
    let result = manager.copy_package(&package, dest_path, Some(options)).unwrap();
    
    assert!(result.success);
}

#[test]
fn test_package_copy_invalid_version() {
    let mut package = Package::new("test_package".to_string());
    package.version = Some(Version::parse("1.0.0").unwrap());
    
    let manager = PackageManager::new();
    let mut options = PackageCopyOptions::new();
    options.dest_version = Some("invalid.version".to_string());
    
    let temp_dir = TempDir::new().unwrap();
    let dest_path = temp_dir.path().to_str().unwrap();
    
    let result = manager.copy_package(&package, dest_path, Some(options));
    
    assert!(result.is_err()); // Should fail due to invalid version
}

#[test]
fn test_package_move() {
    let mut package = Package::new("test_package".to_string());
    package.version = Some(Version::parse("1.0.0").unwrap());
    package.description = Some("Test package".to_string());
    
    let manager = PackageManager::new();
    let mut options = PackageCopyOptions::new();
    options.install_options.dry_run = true;
    options.install_options.validate = false;
    
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("source").to_str().unwrap().to_string();
    let dest_path = temp_dir.path().join("dest").to_str().unwrap().to_string();
    
    let result = manager.move_package(&package, &source_path, &dest_path, Some(options)).unwrap();
    
    assert!(result.success);
    assert!(result.message.contains("Moved package"));
}

#[test]
fn test_package_remove() {
    let manager = PackageManager::new();
    
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().to_str().unwrap();
    
    let result = manager.remove_package("test_package", Some("1.0.0"), repo_path, Some(false)).unwrap();
    
    assert!(result.success);
    assert!(result.message.contains("Removed package"));
}

#[test]
fn test_package_remove_family() {
    let manager = PackageManager::new();
    
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().to_str().unwrap();
    
    let result = manager.remove_package_family("test_family", repo_path, Some(false)).unwrap();
    
    assert!(result.success);
    assert!(result.message.contains("Removed package family"));
}

#[test]
fn test_validation_options() {
    let options = PackageValidationOptions::new();
    assert!(options.check_metadata);
    assert!(options.check_dependencies);
    assert!(options.check_variants);
    assert!(options.check_structure);
    assert!(options.check_circular_deps);
    assert!(!options.strict_mode);
    
    let quick_options = PackageValidationOptions::quick();
    assert!(quick_options.check_metadata);
    assert!(!quick_options.check_dependencies);
    
    let full_options = PackageValidationOptions::full();
    assert!(full_options.check_metadata);
    assert!(full_options.check_dependencies);
    assert!(full_options.strict_mode);
}

#[test]
fn test_install_options() {
    let options = PackageInstallOptions::new();
    assert!(!options.overwrite);
    assert!(!options.keep_timestamp);
    assert!(!options.force);
    assert!(!options.dry_run);
    assert!(!options.verbose);
    assert!(!options.skip_payload);
    assert!(options.validate);
    
    let quick_options = PackageInstallOptions::quick();
    assert!(quick_options.skip_payload);
    assert!(!quick_options.validate);
    
    let safe_options = PackageInstallOptions::safe();
    assert!(safe_options.keep_timestamp);
    assert!(safe_options.verbose);
    assert!(safe_options.validate);
}

#[test]
fn test_operation_result() {
    let mut result = PackageOperationResult::new(true, "Test operation".to_string());
    
    result.add_copied_variant("variant1".to_string());
    result.add_skipped_variant("variant2".to_string());
    result.set_duration(1000);
    result.add_metadata("key".to_string(), "value".to_string());
    
    assert!(result.success);
    assert_eq!(result.copied_variants.len(), 1);
    assert_eq!(result.skipped_variants.len(), 1);
    assert_eq!(result.duration_ms, 1000);
    assert_eq!(result.metadata.get("key"), Some(&"value".to_string()));
    
    let success_result = PackageOperationResult::success("Success".to_string());
    assert!(success_result.success);
    
    let failure_result = PackageOperationResult::failure("Failure".to_string());
    assert!(!failure_result.success);
}
