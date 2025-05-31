//! GIL optimization tests
//!
//! This module contains tests to verify that GIL release optimization is working correctly.

#[cfg(test)]
mod tests {
    use super::*;
    use rez_core_version::Version;
    use pyo3::prelude::*;
    use std::time::Instant;

    #[test]
    fn test_gil_release_parsing() {
        // Test that GIL release parsing works correctly
        let version_str = "1.2.3-alpha.1";
        
        // Test optimized parsing
        let optimized_result = Version::parse_with_gil_release(version_str);
        assert!(optimized_result.is_ok());
        
        // Test regular parsing
        let regular_result = Version::parse(version_str);
        assert!(regular_result.is_ok());
        
        // Results should be equivalent
        let optimized = optimized_result.unwrap();
        let regular = regular_result.unwrap();
        assert_eq!(optimized.as_str(), regular.as_str());
    }

    #[test]
    fn test_gil_release_comparison() {
        let version1 = Version::parse("1.2.3").unwrap();
        let version2 = Version::parse("1.2.4").unwrap();
        
        // Test optimized comparison
        let optimized_cmp = version1.cmp_with_gil_release(&version2);
        
        // Test regular comparison
        let regular_cmp = version1.cmp(&version2);
        
        // Results should be equivalent
        assert_eq!(optimized_cmp, regular_cmp);
        assert_eq!(optimized_cmp, std::cmp::Ordering::Less);
    }

    #[test]
    fn test_gil_release_special_versions() {
        // Test special versions with GIL release
        let empty = Version::parse_with_gil_release("").unwrap();
        let inf = Version::parse_with_gil_release("inf").unwrap();
        let epsilon = Version::parse_with_gil_release("epsilon").unwrap();
        
        assert!(empty.is_empty());
        assert!(inf.is_inf());
        assert!(epsilon.is_epsilon());
        
        // Test comparison of special versions
        assert_eq!(empty.cmp_with_gil_release(&epsilon), std::cmp::Ordering::Equal);
        assert_eq!(empty.cmp_with_gil_release(&inf), std::cmp::Ordering::Less);
        assert_eq!(inf.cmp_with_gil_release(&empty), std::cmp::Ordering::Greater);
    }

    #[test]
    fn test_gil_release_error_handling() {
        // Test that errors are properly handled in GIL release mode
        let invalid_versions = vec![
            "v1.2.3",  // Version prefix not supported
            "1..2",    // Invalid syntax
            ".1.2",    // Invalid syntax
            "1.2.",    // Invalid syntax
        ];
        
        for invalid in invalid_versions {
            let optimized_result = Version::parse_with_gil_release(invalid);
            let regular_result = Version::parse(invalid);
            
            // Both should fail
            assert!(optimized_result.is_err());
            assert!(regular_result.is_err());
            
            // Error messages should be similar
            let optimized_err = optimized_result.unwrap_err().to_string();
            let regular_err = regular_result.unwrap_err().to_string();
            assert_eq!(optimized_err, regular_err);
        }
    }

    #[test]
    fn test_performance_comparison() {
        // Simple performance comparison test
        let test_versions = vec![
            "1.2.3", "2.0.0-alpha.1", "3.1.4-beta.2", "1.0.0-rc.1",
            "4.5.6", "0.1.0-dev.123", "10.20.30", "1.2.3-snapshot.1"
        ];
        
        // Warm up
        for version_str in &test_versions {
            let _ = Version::parse(version_str);
            let _ = Version::parse_with_gil_release(version_str);
        }
        
        // Test regular parsing
        let start = Instant::now();
        for _ in 0..1000 {
            for version_str in &test_versions {
                let _ = Version::parse(version_str).unwrap();
            }
        }
        let regular_duration = start.elapsed();
        
        // Test optimized parsing
        let start = Instant::now();
        for _ in 0..1000 {
            for version_str in &test_versions {
                let _ = Version::parse_with_gil_release(version_str).unwrap();
            }
        }
        let optimized_duration = start.elapsed();
        
        println!("Regular parsing: {:?}", regular_duration);
        println!("Optimized parsing: {:?}", optimized_duration);
        
        // The optimized version should not be significantly slower
        // (In a real multi-threaded environment, it should be faster)
        assert!(optimized_duration.as_millis() <= regular_duration.as_millis() * 2);
    }
}
