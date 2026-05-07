//! Platform detection utilities

/// Check if running on Windows
#[inline]
pub fn is_windows() -> bool {
    cfg!(target_os = "windows")
}

/// Check if running on Linux
#[inline]
pub fn is_linux() -> bool {
    cfg!(target_os = "linux")
}

/// Check if running on macOS
#[inline]
pub fn is_macos() -> bool {
    cfg!(target_os = "macos")
}

/// Get the current platform name
pub fn get_platform() -> &'static str {
    if is_windows() {
        "windows"
    } else if is_linux() {
        "linux"
    } else if is_macos() {
        "macos"
    } else {
        "unknown"
    }
}

/// Check if running on a Unix-like system (Linux or macOS)
#[inline]
pub fn is_unix() -> bool {
    is_linux() || is_macos()
}

/// Get the platform architecture
pub fn get_architecture() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "x86") {
        "x86"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else if cfg!(target_arch = "arm") {
        "arm"
    } else {
        "unknown"
    }
}

/// Get a string representation of the current platform and architecture
pub fn get_platform_id() -> String {
    format!("{}-{}", get_platform(), get_architecture())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        // At least one should be true
        assert!(is_windows() || is_linux() || is_macos());
    }

    #[test]
    fn test_get_platform() {
        let platform = get_platform();
        assert!(!platform.is_empty());
        assert!(platform == "windows" || platform == "linux" || platform == "macos" || platform == "unknown");
    }

    #[test]
    fn test_get_architecture() {
        let arch = get_architecture();
        assert!(!arch.is_empty());
    }

    #[test]
    fn test_get_platform_id() {
        let id = get_platform_id();
        assert!(!id.is_empty());
        assert!(id.contains('-'));
    }

    #[test]
    fn test_is_unix() {
        if is_linux() || is_macos() {
            assert!(is_unix());
        }
        // Note: we can't assert is_unix() == false on Windows because
        // this test might run on Unix
    }
}
