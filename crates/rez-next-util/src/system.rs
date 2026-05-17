// system.rs - System information module aligned with Rez's system.py interface
// Copyright Contributors to the Rez Project
// SPDX-License-Identifier: Apache-2.0

use std::env;
use std::path::PathBuf;

use dirs;

/// Get the current machine's hostname
pub fn get_hostname() -> String {
    // Try to get hostname from environment variables first (cross-platform)
    #[cfg(windows)]
    {
        if let Ok(computername) = env::var("COMPUTERNAME") {
            return computername;
        }
    }

    #[cfg(not(windows))]
    {
        if let Ok(hostname) = env::var("HOSTNAME") {
            return hostname;
        }
    }

    // Fallback to gethostname crate or similar
    // For now, return a default value
    "unknown".to_string()
}

/// Get the current username
pub fn get_username() -> String {
    // Try to get username from environment variables
    if let Ok(user) = env::var("USER") {
        return user;
    }
    if let Ok(username) = env::var("USERNAME") {
        return username;
    }
    if let Ok(logname) = env::var("LOGNAME") {
        return logname;
    }

    "unknown".to_string()
}

/// Get the current user's home directory
pub fn get_home_directory() -> Option<PathBuf> {
    dirs::home_dir()
}

/// Get the current machine's fully qualified domain name (FQDN)
pub fn get_fqdn() -> String {
    // Try to get FQDN using system APIs
    // For now, return hostname as fallback
    get_hostname()
}

/// Get the current machine's domain
pub fn get_domain() -> String {
    let fqdn = get_fqdn();
    if let Some(dot_pos) = fqdn.find('.') {
        fqdn[dot_pos + 1..].to_string()
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_hostname() {
        let hostname = get_hostname();
        assert!(!hostname.is_empty());
    }

    #[test]
    fn test_get_username() {
        let username = get_username();
        assert!(!username.is_empty());
    }

    #[test]
    fn test_get_home_directory() {
        let home = get_home_directory();
        assert!(home.is_some());
    }

    #[test]
    fn test_get_fqdn() {
        let fqdn = get_fqdn();
        assert!(!fqdn.is_empty());
    }

    #[test]
    fn test_get_domain() {
        let domain = get_domain();
        // Domain might be empty if FQDN doesn't contain a dot
        // Just ensure it doesn't panic
        let _ = domain;
    }
}
