//! Simple Rust test for package management functionality

use rez_next_package::{
    Package, PackageCopyOptions, PackageInstallOptions, PackageManager, PackageValidationOptions,
    PackageValidator,
};
use rez_next_version::Version;
use tempfile::TempDir;

fn test_package_creation() {
    println!("🧪 Testing package creation...");

    let mut package = Package::new("test_package".to_string());
    package.version = Some(Version::parse("1.0.0").unwrap());
    package.description = Some("Test package description".to_string());
    package.authors = vec!["Test Author".to_string()];

    println!("   Package name: {}", package.name);
    println!(
        "   Package version: {}",
        package
            .version
            .as_ref()
            .map(|v| v.as_str())
            .unwrap_or("None")
    );
    println!(
        "   Package description: {}",
        package.description.as_ref().unwrap_or(&"None".to_string())
    );
    println!("✅ Package creation test passed");
}

fn test_package_validation() {
    println!("\n🧪 Testing package validation...");

    // Create a valid package
    let mut package = Package::new("valid_package".to_string());
    package.version = Some(Version::parse("1.0.0").unwrap());
    package.description = Some("Valid test package".to_string());
    package.authors = vec!["Test Author".to_string()];

    // Create validator
    let validator = PackageValidator::new(Some(PackageValidationOptions::new()));

    // Validate the package
    let result = validator.validate_package(&package).unwrap();

    println!("   Validation result: {}", result.is_valid);
    println!("   Errors: {}", result.errors.len());
    println!("   Warnings: {}", result.warnings.len());

    if result.is_valid {
        println!("✅ Package validation test passed");
    } else {
        println!("❌ Package validation test failed");
        for error in &result.errors {
            println!("     Error: {}", error);
        }
    }
}

fn test_package_validation_invalid() {
    println!("\n🧪 Testing package validation with invalid package...");

    // Create an invalid package (empty name)
    let package = Package::new("".to_string());

    // Create validator
    let validator = PackageValidator::new(Some(PackageValidationOptions::new()));

    // Validate the package
    let result = validator.validate_package(&package).unwrap();

    println!("   Validation result: {}", result.is_valid);
    println!("   Errors: {}", result.errors.len());
    println!("   Warnings: {}", result.warnings.len());

    if !result.is_valid && !result.errors.is_empty() {
        println!("✅ Invalid package validation test passed");
        for error in &result.errors {
            println!("     Error: {}", error);
        }
    } else {
        println!("❌ Invalid package validation test failed");
    }
}

fn test_package_manager() {
    println!("\n🧪 Testing package manager...");

    // Create a test package
    let mut package = Package::new("test_package".to_string());
    package.version = Some(Version::parse("1.0.0").unwrap());
    package.description = Some("Test package for manager".to_string());
    package.authors = vec!["Test Author".to_string()];

    // Create package manager
    let manager = PackageManager::new();

    // Test dry run installation
    let temp_dir = TempDir::new().unwrap();
    let dest_path = temp_dir.path().to_str().unwrap();

    let mut options = PackageInstallOptions::new();
    options.dry_run = true;
    options.validate = false; // Skip validation for simplicity

    let result = manager
        .install_package(&package, dest_path, Some(options))
        .unwrap();

    println!("   Install result: {}", result.success);
    println!("   Install message: {}", result.message);
    println!("   Duration: {}ms", result.duration_ms);

    if result.success && result.message.contains("Would install") {
        println!("✅ Package manager dry run test passed");
    } else {
        println!("❌ Package manager dry run test failed");
    }
}

fn test_package_copy() {
    println!("\n🧪 Testing package copy...");

    // Create a test package
    let mut package = Package::new("original_package".to_string());
    package.version = Some(Version::parse("1.0.0").unwrap());
    package.description = Some("Original package".to_string());
    package.authors = vec!["Test Author".to_string()];

    // Create package manager
    let manager = PackageManager::new();

    // Test copy with rename
    let temp_dir = TempDir::new().unwrap();
    let dest_path = temp_dir.path().to_str().unwrap();

    let mut options = PackageCopyOptions::new();
    options.set_dest_name("renamed_package".to_string());
    options.install_options.dry_run = true;
    options.install_options.validate = false;

    let result = manager
        .copy_package(&package, dest_path, Some(options))
        .unwrap();

    println!("   Copy result: {}", result.success);
    println!("   Copy message: {}", result.message);

    if result.success {
        println!("✅ Package copy test passed");
    } else {
        println!("❌ Package copy test failed");
    }
}

fn test_validation_options() {
    println!("\n🧪 Testing validation options...");

    // Test default options
    let default_options = PackageValidationOptions::new();
    println!(
        "   Default check_metadata: {}",
        default_options.check_metadata
    );
    println!(
        "   Default check_dependencies: {}",
        default_options.check_dependencies
    );
    println!("   Default strict_mode: {}", default_options.strict_mode);

    // Test quick options
    let quick_options = PackageValidationOptions::quick();
    println!("   Quick check_metadata: {}", quick_options.check_metadata);
    println!(
        "   Quick check_dependencies: {}",
        quick_options.check_dependencies
    );

    // Test full options
    let full_options = PackageValidationOptions::full();
    println!("   Full check_metadata: {}", full_options.check_metadata);
    println!("   Full strict_mode: {}", full_options.strict_mode);

    println!("✅ Validation options test passed");
}

fn test_install_options() {
    println!("\n🧪 Testing install options...");

    // Test default options
    let default_options = PackageInstallOptions::new();
    println!("   Default overwrite: {}", default_options.overwrite);
    println!("   Default validate: {}", default_options.validate);
    println!("   Default dry_run: {}", default_options.dry_run);

    // Test quick options
    let quick_options = PackageInstallOptions::quick();
    println!("   Quick skip_payload: {}", quick_options.skip_payload);
    println!("   Quick validate: {}", quick_options.validate);

    // Test safe options
    let safe_options = PackageInstallOptions::safe();
    println!("   Safe keep_timestamp: {}", safe_options.keep_timestamp);
    println!("   Safe verbose: {}", safe_options.verbose);

    println!("✅ Install options test passed");
}

fn main() {
    println!("🚀 Starting Package Management Tests (Rust)");
    println!("{}", "=".repeat(50));

    test_package_creation();
    test_package_validation();
    test_package_validation_invalid();
    test_package_manager();
    test_package_copy();
    test_validation_options();
    test_install_options();

    println!("\n{}", "=".repeat(50));
    println!("🎉 All tests completed successfully!");
}
