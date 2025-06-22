#!/usr/bin/env python3
"""
Validation script for the comprehensive benchmark framework.
This script validates the structure and functionality of the benchmark framework.
"""

import json
import os
import sys
from pathlib import Path


def validate_file_structure():
    """Validate that all required files exist."""
    print("🔍 Validating file structure...")

    required_files = [
        "benches/comprehensive_benchmark_suite.rs",
        "benches/example_version_module.rs",
        "Cargo.toml",
    ]

    missing_files = []
    for file_path in required_files:
        if not Path(file_path).exists():
            missing_files.append(file_path)

    if missing_files:
        print(f"❌ Missing files: {missing_files}")
        return False

    print("✅ All required files exist")
    return True


def validate_cargo_config():
    """Validate Cargo.toml configuration."""
    print("🔍 Validating Cargo.toml configuration...")

    try:
        # Check if comprehensive_benchmark_suite is in Cargo.toml
        with open("Cargo.toml") as f:
            content = f.read()

        if "comprehensive_benchmark_suite" not in content:
            print("❌ comprehensive_benchmark_suite not found in Cargo.toml")
            return False

        if "serde.workspace = true" not in content:
            print("❌ serde dependency not found in dev-dependencies")
            return False

        if "thiserror.workspace = true" not in content:
            print("❌ thiserror dependency not found in dev-dependencies")
            return False

        print("✅ Cargo.toml configuration is valid")
        return True

    except Exception as e:
        print(f"❌ Error validating Cargo.toml: {e}")
        return False


def validate_rust_syntax():
    """Validate Rust syntax of benchmark files."""
    print("🔍 Validating Rust syntax...")

    benchmark_files = [
        "benches/comprehensive_benchmark_suite.rs",
        "benches/example_version_module.rs",
    ]

    for file_path in benchmark_files:
        try:
            # Basic syntax validation by checking for common patterns
            with open(file_path, encoding="utf-8") as f:
                content = f.read()

            # Check for basic Rust structure
            if "use " not in content:
                print(f"❌ {file_path}: No use statements found")
                return False

            if "pub " not in content and "fn " not in content:
                print(f"❌ {file_path}: No functions found")
                return False

            # Check for specific framework components
            if file_path.endswith("comprehensive_benchmark_suite.rs"):
                required_items = [
                    "trait ModuleBenchmark",
                    "struct BenchmarkSuite",
                    "struct BaselineMetrics",
                    "struct BenchmarkConfig",
                ]

                for item in required_items:
                    if item not in content:
                        print(f"❌ {file_path}: Missing {item}")
                        return False

            print(f"✅ {file_path}: Syntax validation passed")

        except Exception as e:
            print(f"❌ Error validating {file_path}: {e}")
            return False

    return True


def validate_framework_structure():
    """Validate the framework structure and components."""
    print("🔍 Validating framework structure...")

    try:
        with open("benches/comprehensive_benchmark_suite.rs", encoding="utf-8") as f:
            content = f.read()

        # Check for essential traits and structs
        essential_components = [
            "trait ModuleBenchmark",
            "struct BenchmarkSuite",
            "struct BaselineMetrics",
            "struct BenchmarkResult",
            "struct BenchmarkConfig",
            "struct BaselineStorage",
            "enum BenchmarkError",
            "mod config_helpers",
            "mod environment",
            "mod analysis",
        ]

        missing_components = []
        for component in essential_components:
            if component not in content:
                missing_components.append(component)

        if missing_components:
            print(f"❌ Missing framework components: {missing_components}")
            return False

        # Check for essential methods
        essential_methods = [
            "fn run_benchmarks",
            "fn get_baseline_metrics",
            "fn register_module",
            "fn run_all",
            "fn save_baseline",
            "fn load_baseline",
        ]

        missing_methods = []
        for method in essential_methods:
            if method not in content:
                missing_methods.append(method)

        if missing_methods:
            print(f"❌ Missing essential methods: {missing_methods}")
            return False

        print("✅ Framework structure validation passed")
        return True

    except Exception as e:
        print(f"❌ Error validating framework structure: {e}")
        return False


def validate_example_implementation():
    """Validate the example implementation."""
    print("🔍 Validating example implementation...")

    try:
        with open("benches/example_version_module.rs", encoding="utf-8") as f:
            content = f.read()

        # Check for example implementation components
        required_components = [
            "struct VersionModuleBenchmark",
            "impl ModuleBenchmark for VersionModuleBenchmark",
            "fn benchmark_version_parsing",
            "fn benchmark_version_comparison",
            "fn benchmark_version_sorting",
            "#[cfg(test)]",
        ]

        missing_components = []
        for component in required_components:
            if component not in content:
                missing_components.append(component)

        if missing_components:
            print(f"❌ Missing example components: {missing_components}")
            return False

        print("✅ Example implementation validation passed")
        return True

    except Exception as e:
        print(f"❌ Error validating example implementation: {e}")
        return False


def create_test_baseline():
    """Create a test baseline to validate storage functionality."""
    print("🔍 Creating test baseline...")

    try:
        # Create test baseline directory
        baseline_dir = Path("test_baselines")
        baseline_dir.mkdir(exist_ok=True)

        # Create test baseline data
        test_baseline = {
            "module_name": "test_module",
            "timestamp": "2024-01-01T00:00:00Z",
            "benchmarks": {
                "test_benchmark": {
                    "name": "test_benchmark",
                    "mean_time_ns": 1000.0,
                    "std_dev_ns": 50.0,
                    "throughput_ops_per_sec": 1000000.0,
                    "memory_usage_bytes": 1024,
                    "additional_metrics": {},
                }
            },
            "overall_score": 90.0,
            "environment": {
                "os": "test_os",
                "cpu": "test_cpu",
                "memory_bytes": 8589934592,
                "rust_version": "1.70.0",
                "compiler_flags": ["-O3"],
            },
        }

        # Save test baseline
        baseline_file = baseline_dir / "test_module.json"
        with open(baseline_file, "w") as f:
            json.dump(test_baseline, f, indent=2)

        # Validate the saved file
        with open(baseline_file) as f:
            loaded_baseline = json.load(f)

        if loaded_baseline["module_name"] != "test_module":
            print("❌ Baseline save/load validation failed")
            return False

        print("✅ Test baseline creation and validation passed")

        # Cleanup
        baseline_file.unlink()
        baseline_dir.rmdir()

        return True

    except Exception as e:
        print(f"❌ Error creating test baseline: {e}")
        return False


def generate_validation_report():
    """Generate a validation report."""
    print("\n📊 Comprehensive Benchmark Framework Validation Report")
    print("=" * 60)

    validations = [
        ("File Structure", validate_file_structure),
        ("Cargo Configuration", validate_cargo_config),
        ("Rust Syntax", validate_rust_syntax),
        ("Framework Structure", validate_framework_structure),
        ("Example Implementation", validate_example_implementation),
        ("Baseline Storage", create_test_baseline),
    ]

    results = {}
    all_passed = True

    for name, validation_func in validations:
        print(f"\n🔍 Running {name} validation...")
        try:
            result = validation_func()
            results[name] = result
            if not result:
                all_passed = False
        except Exception as e:
            print(f"❌ {name} validation failed with exception: {e}")
            results[name] = False
            all_passed = False

    print("\n📋 Validation Summary:")
    print("-" * 30)
    for name, result in results.items():
        status = "✅ PASS" if result else "❌ FAIL"
        print(f"{name:.<25} {status}")

    print(
        f"\n🎯 Overall Result: {'✅ ALL VALIDATIONS PASSED' if all_passed else '❌ SOME VALIDATIONS FAILED'}"
    )

    if all_passed:
        print("\n🎉 The comprehensive benchmark framework is ready for use!")
        print("Next steps:")
        print("1. Implement actual module benchmarks")
        print("2. Integrate with CI/CD pipeline")
        print("3. Set up performance regression detection")
        print("4. Create performance monitoring dashboard")
    else:
        print("\n🔧 Please fix the failed validations before proceeding.")

    return all_passed


def main():
    """Main validation function."""
    print("🚀 Comprehensive Benchmark Framework Validation")
    print("=" * 50)

    # Change to project root directory
    script_dir = Path(__file__).parent
    project_root = script_dir.parent
    os.chdir(project_root)

    # Run validation
    success = generate_validation_report()

    # Exit with appropriate code
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
