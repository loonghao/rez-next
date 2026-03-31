//! Utility functions for rez-core

/// Get the number of threads to use for parallel operations
pub fn get_thread_count(config_threads: Option<usize>) -> usize {
    config_threads.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    })
}

/// Validate a package name
pub fn is_valid_package_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        && !name.starts_with('-')
        && !name.ends_with('-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_count() {
        assert!(get_thread_count(None) >= 1);
        assert_eq!(get_thread_count(Some(8)), 8);
    }

    #[test]
    fn test_thread_count_custom_zero_is_accepted() {
        // Custom value of 0 should be returned as-is (caller responsibility)
        assert_eq!(get_thread_count(Some(0)), 0);
    }

    #[test]
    fn test_thread_count_large_value() {
        assert_eq!(get_thread_count(Some(256)), 256);
    }

    #[test]
    fn test_package_name_validation() {
        assert!(is_valid_package_name("valid_package"));
        assert!(is_valid_package_name("valid-package"));
        assert!(is_valid_package_name("package123"));

        assert!(!is_valid_package_name(""));
        assert!(!is_valid_package_name("-invalid"));
        assert!(!is_valid_package_name("invalid-"));
        assert!(!is_valid_package_name("invalid package"));
    }

    #[test]
    fn test_package_name_single_char() {
        assert!(is_valid_package_name("a"));
        assert!(is_valid_package_name("1"));
        assert!(!is_valid_package_name("-"));
    }

    #[test]
    fn test_package_name_numbers_only() {
        assert!(is_valid_package_name("123"));
    }

    #[test]
    fn test_package_name_special_chars_rejected() {
        assert!(!is_valid_package_name("pkg@1"));
        assert!(!is_valid_package_name("pkg.name"));
        assert!(!is_valid_package_name("pkg/name"));
        assert!(!is_valid_package_name("pkg:name"));
    }

    #[test]
    fn test_package_name_mixed_valid() {
        assert!(is_valid_package_name("my_package-1"));
        assert!(is_valid_package_name("PkgName"));
        assert!(is_valid_package_name("pkg_2024"));
    }

    #[test]
    fn test_package_name_consecutive_hyphens() {
        // Consecutive hyphens are valid (only start/end restriction)
        assert!(is_valid_package_name("pkg--name"));
    }
}
