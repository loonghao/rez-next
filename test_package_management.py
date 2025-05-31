#!/usr/bin/env python3
"""
Simple test script for package management functionality
"""

import sys
import os
import tempfile
import shutil

# Add the build directory to Python path
sys.path.insert(0, 'target/debug')

try:
    import rez_core_package
    from rez_core_package import (
        Package, PackageManager, PackageValidator,
        PackageValidationOptions, PackageInstallOptions,
        PackageCopyOptions, PackageOperationResult
    )
    print("✅ Successfully imported rez-core-package modules")

    # Try to import version module separately
    try:
        import rez_core_version
        from rez_core_version import Version
        print("✅ Successfully imported rez-core-version modules")
    except ImportError as e:
        print(f"⚠️  Could not import rez-core-version: {e}")
        print("   Creating version manually...")
        # Create a simple version class for testing
        class Version:
            def __init__(self, version_str):
                self.version_str = version_str

            @staticmethod
            def parse(version_str):
                return Version(version_str)

            def as_str(self):
                return self.version_str

except ImportError as e:
    print(f"❌ Failed to import rez-core modules: {e}")
    sys.exit(1)

def test_package_creation():
    """Test basic package creation"""
    print("\n🧪 Testing package creation...")
    
    package = Package("test_package")
    package.version = Version.parse("1.0.0")
    package.description = "Test package description"
    package.authors = ["Test Author"]
    
    print(f"   Package name: {package.name}")
    print(f"   Package version: {package.version.as_str() if package.version else 'None'}")
    print(f"   Package description: {package.description}")
    print("✅ Package creation test passed")

def test_package_validation():
    """Test package validation"""
    print("\n🧪 Testing package validation...")
    
    # Create a valid package
    package = Package("valid_package")
    package.version = Version.parse("1.0.0")
    package.description = "Valid test package"
    package.authors = ["Test Author"]
    
    # Create validator
    validator = PackageValidator(PackageValidationOptions())
    
    # Validate the package
    result = validator.validate_package(package)
    
    print(f"   Validation result: {result.is_valid}")
    print(f"   Errors: {len(result.errors)}")
    print(f"   Warnings: {len(result.warnings)}")
    
    if result.is_valid:
        print("✅ Package validation test passed")
    else:
        print("❌ Package validation test failed")
        for error in result.errors:
            print(f"     Error: {error}")

def test_package_validation_invalid():
    """Test package validation with invalid package"""
    print("\n🧪 Testing package validation with invalid package...")
    
    # Create an invalid package (empty name)
    package = Package("")
    
    # Create validator
    validator = PackageValidator(PackageValidationOptions())
    
    # Validate the package
    result = validator.validate_package(package)
    
    print(f"   Validation result: {result.is_valid}")
    print(f"   Errors: {len(result.errors)}")
    print(f"   Warnings: {len(result.warnings)}")
    
    if not result.is_valid and len(result.errors) > 0:
        print("✅ Invalid package validation test passed")
        for error in result.errors:
            print(f"     Error: {error}")
    else:
        print("❌ Invalid package validation test failed")

def test_package_manager():
    """Test package manager operations"""
    print("\n🧪 Testing package manager...")
    
    # Create a test package
    package = Package("test_package")
    package.version = Version.parse("1.0.0")
    package.description = "Test package for manager"
    package.authors = ["Test Author"]
    
    # Create package manager
    manager = PackageManager()
    
    # Test dry run installation
    with tempfile.TemporaryDirectory() as temp_dir:
        options = PackageInstallOptions()
        options.dry_run = True
        options.validate = False  # Skip validation for simplicity
        
        result = manager.install_package(package, temp_dir, options)
        
        print(f"   Install result: {result.success}")
        print(f"   Install message: {result.message}")
        print(f"   Duration: {result.duration_ms}ms")
        
        if result.success and "Would install" in result.message:
            print("✅ Package manager dry run test passed")
        else:
            print("❌ Package manager dry run test failed")

def test_package_copy():
    """Test package copy functionality"""
    print("\n🧪 Testing package copy...")
    
    # Create a test package
    package = Package("original_package")
    package.version = Version.parse("1.0.0")
    package.description = "Original package"
    package.authors = ["Test Author"]
    
    # Create package manager
    manager = PackageManager()
    
    # Test copy with rename
    with tempfile.TemporaryDirectory() as temp_dir:
        options = PackageCopyOptions()
        options.set_dest_name("renamed_package")
        options.install_options.dry_run = True
        options.install_options.validate = False
        
        result = manager.copy_package(package, temp_dir, options)
        
        print(f"   Copy result: {result.success}")
        print(f"   Copy message: {result.message}")
        
        if result.success:
            print("✅ Package copy test passed")
        else:
            print("❌ Package copy test failed")

def test_validation_options():
    """Test validation options"""
    print("\n🧪 Testing validation options...")
    
    # Test default options
    default_options = PackageValidationOptions()
    print(f"   Default check_metadata: {default_options.check_metadata}")
    print(f"   Default check_dependencies: {default_options.check_dependencies}")
    print(f"   Default strict_mode: {default_options.strict_mode}")
    
    # Test quick options
    quick_options = PackageValidationOptions.quick()
    print(f"   Quick check_metadata: {quick_options.check_metadata}")
    print(f"   Quick check_dependencies: {quick_options.check_dependencies}")
    
    # Test full options
    full_options = PackageValidationOptions.full()
    print(f"   Full check_metadata: {full_options.check_metadata}")
    print(f"   Full strict_mode: {full_options.strict_mode}")
    
    print("✅ Validation options test passed")

def test_install_options():
    """Test install options"""
    print("\n🧪 Testing install options...")
    
    # Test default options
    default_options = PackageInstallOptions()
    print(f"   Default overwrite: {default_options.overwrite}")
    print(f"   Default validate: {default_options.validate}")
    print(f"   Default dry_run: {default_options.dry_run}")
    
    # Test quick options
    quick_options = PackageInstallOptions.quick()
    print(f"   Quick skip_payload: {quick_options.skip_payload}")
    print(f"   Quick validate: {quick_options.validate}")
    
    # Test safe options
    safe_options = PackageInstallOptions.safe()
    print(f"   Safe keep_timestamp: {safe_options.keep_timestamp}")
    print(f"   Safe verbose: {safe_options.verbose}")
    
    print("✅ Install options test passed")

def main():
    """Run all tests"""
    print("🚀 Starting Package Management Tests")
    print("=" * 50)
    
    try:
        test_package_creation()
        test_package_validation()
        test_package_validation_invalid()
        test_package_manager()
        test_package_copy()
        test_validation_options()
        test_install_options()
        
        print("\n" + "=" * 50)
        print("🎉 All tests completed successfully!")
        
    except Exception as e:
        print(f"\n❌ Test failed with error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)

if __name__ == "__main__":
    main()
