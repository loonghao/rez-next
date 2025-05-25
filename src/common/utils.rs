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
        && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
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
    fn test_package_name_validation() {
        assert!(is_valid_package_name("valid_package"));
        assert!(is_valid_package_name("valid-package"));
        assert!(is_valid_package_name("package123"));
        
        assert!(!is_valid_package_name(""));
        assert!(!is_valid_package_name("-invalid"));
        assert!(!is_valid_package_name("invalid-"));
        assert!(!is_valid_package_name("invalid package"));
    }
}
