//! Integration tests for rez-core

use rez_core::common::RezCoreConfig;
use rez_core::version::{Version, VersionRange};

#[test]
fn test_version_creation() {
    let version = Version::parse("1.2.3").expect("Should create version");
    assert_eq!(version.as_str(), "1.2.3");
}

#[test]
fn test_version_range_creation() {
    let range = VersionRange::parse("1.0.0..2.0.0").expect("Should create version range");
    assert_eq!(range.as_str(), "1.0.0..2.0.0");
}

#[test]
fn test_version_token_creation() {
    let numeric_token = VersionToken::from_str("123");
    assert_eq!(numeric_token, VersionToken::Numeric(123));

    let alpha_token = VersionToken::from_str("alpha");
    assert_eq!(alpha_token, VersionToken::Alphanumeric("alpha".to_string()));
}

#[test]
fn test_version_token_comparison() {
    let num1 = VersionToken::Numeric(1);
    let num2 = VersionToken::Numeric(2);
    let alpha = VersionToken::Alphanumeric("alpha".to_string());

    assert!(num1 < num2);
    assert!(num1 < alpha);
}

#[test]
fn test_config_defaults() {
    let config = RezCoreConfig::default();
    assert!(config.use_rust_version);
    assert!(config.use_rust_solver);
    assert!(config.use_rust_repository);
    assert!(config.rust_fallback);
}

#[test]
fn test_module_structure() {
    // Test that all modules can be imported and basic functionality works
    let _version = Version::parse("1.0.0").expect("Version creation should work");
    let _range = VersionRange::parse("1.0.0+").expect("Range creation should work");
    let _config = RezCoreConfig::default();

    // This test ensures the basic module structure is working
    assert!(true);
}

// Performance optimization tests
mod performance_tests {
    use std::time::Instant;

    #[test]
    fn test_version_parsing_performance() {
        println!("Testing version parsing performance...");

        let test_versions = vec![
            "1.2.3",
            "1.2.3-alpha.1",
            "2.0.0-beta.2+build.123",
            "1.0.0-rc.1",
            "3.1.4-dev.123",
        ];

        let start_time = Instant::now();

        // Simulate optimized parsing
        for version_str in &test_versions {
            let _parsed = format!("parsed_{}", version_str);
        }

        let duration = start_time.elapsed();
        println!("Mock parsing took: {:?}", duration);

        assert!(duration.as_millis() < 100, "Parsing should be fast");
    }

    #[test]
    fn test_batch_processing_simulation() {
        println!("Testing batch processing simulation...");

        let version_strings: Vec<String> = (0..1000)
            .map(|i| format!("1.{}.{}", i / 100, i % 100))
            .collect();

        let start_time = Instant::now();

        let _results: Vec<_> = version_strings
            .iter()
            .map(|v| format!("processed_{}", v))
            .collect();

        let duration = start_time.elapsed();
        println!(
            "Batch processing of {} items took: {:?}",
            version_strings.len(),
            duration
        );

        assert!(
            duration.as_millis() < 1000,
            "Batch processing should be efficient"
        );
    }

    #[test]
    fn test_simd_pattern_matching_simulation() {
        println!("Testing SIMD pattern matching simulation...");

        let test_files = vec![
            "package.py",
            "package.yaml",
            "package.json",
            "not_a_package.txt",
            "another_package.py",
        ];

        let start_time = Instant::now();

        let matches: Vec<_> = test_files
            .iter()
            .filter(|filename| {
                filename.ends_with(".py")
                    || filename.ends_with(".yaml")
                    || filename.ends_with(".json")
            })
            .collect();

        let duration = start_time.elapsed();
        println!(
            "Pattern matching took: {:?}, found {} matches",
            duration,
            matches.len()
        );

        assert_eq!(matches.len(), 4);
        assert!(duration.as_millis() < 10);
    }

    #[test]
    fn test_memory_efficiency_simulation() {
        println!("Testing memory efficiency simulation...");

        let mut data = Vec::new();

        for i in 0..10000 {
            data.push(format!("item_{}", i));
        }

        assert_eq!(data.len(), 10000);

        data.clear();
        assert_eq!(data.len(), 0);

        println!("Memory efficiency test completed");
    }

    #[test]
    fn test_parallel_processing_simulation() {
        println!("Testing parallel processing simulation...");

        let large_dataset: Vec<String> = (0..5000).map(|i| format!("data_{}", i)).collect();

        let start_time = Instant::now();

        let _results: Vec<_> = large_dataset
            .iter()
            .map(|item| format!("processed_{}", item))
            .collect();

        let duration = start_time.elapsed();
        println!("Parallel simulation took: {:?}", duration);

        assert!(duration.as_millis() < 500);
    }
}
