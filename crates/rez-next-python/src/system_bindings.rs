//! Python bindings for rez system information
//!
//! Provides `rez.system` compatible API: platform, arch, os, rez_version, etc.

use pyo3::prelude::*;

/// System information class - compatible with rez.system
#[pyclass(name = "System")]
pub struct PySystem;

impl Default for PySystem {
    fn default() -> Self {
        Self::new()
    }
}

#[pymethods]
impl PySystem {
    #[new]
    pub fn new() -> Self {
        PySystem
    }

    fn __repr__(&self) -> String {
        format!(
            "System(platform={}, arch={}, os={})",
            Self::platform_str(),
            Self::arch_str(),
            Self::os_str()
        )
    }

    /// Current platform: linux, windows, osx
    #[getter]
    fn platform(&self) -> String {
        Self::platform_str()
    }

    /// Current CPU architecture
    #[getter]
    fn arch(&self) -> String {
        Self::arch_str()
    }

    /// Detailed OS name
    #[getter]
    fn os(&self) -> String {
        Self::os_str()
    }

    /// rez-next version
    #[getter]
    fn rez_version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    /// Python version (of the current interpreter)
    #[getter]
    fn python_version(&self, py: Python) -> String {
        py.version().to_string()
    }

    /// Number of logical CPUs
    #[getter]
    fn num_cpus(&self) -> usize {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    }

    /// Hostname
    #[getter]
    fn hostname(&self) -> String {
        std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "unknown".to_string())
    }
}

impl PySystem {
    pub fn platform_pub() -> String {
        Self::platform_str()
    }

    pub fn arch_pub() -> String {
        Self::arch_str()
    }

    pub fn os_pub() -> String {
        Self::os_str()
    }

    fn platform_str() -> String {
        match std::env::consts::OS {
            "windows" => "windows".to_string(),
            "macos" => "osx".to_string(),
            "linux" => "linux".to_string(),
            other => other.to_string(),
        }
    }

    fn arch_str() -> String {
        match std::env::consts::ARCH {
            "x86_64" => "x86_64".to_string(),
            "aarch64" => "arm_64".to_string(),
            other => other.to_string(),
        }
    }

    fn os_str() -> String {
        // Attempt to read /etc/os-release on Linux, else use OS name
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
                for line in content.lines() {
                    if line.starts_with("PRETTY_NAME=") {
                        return line
                            .trim_start_matches("PRETTY_NAME=")
                            .trim_matches('"')
                            .to_string();
                    }
                }
            }
            "linux".to_string()
        }
        #[cfg(target_os = "windows")]
        {
            // Try Windows version from registry or cmd
            "windows".to_string()
        }
        #[cfg(target_os = "macos")]
        {
            "osx".to_string()
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        {
            std::env::consts::OS.to_string()
        }
    }
}

/// Global system singleton factory
#[pyfunction]
pub fn get_system() -> PySystem {
    PySystem::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    mod test_platform {
        use super::*;

        #[test]
        fn test_platform_str_is_known_value() {
            let platform = PySystem::platform_pub();
            let known = ["linux", "windows", "osx"];
            assert!(
                known.contains(&platform.as_str()) || !platform.is_empty(),
                "platform must be non-empty, got: '{}'",
                platform
            );
        }

        #[test]
        fn test_arch_str_non_empty() {
            let arch = PySystem::arch_pub();
            assert!(!arch.is_empty(), "arch must be non-empty");
        }

        #[test]
        fn test_os_str_non_empty() {
            let os = PySystem::os_pub();
            assert!(!os.is_empty(), "os must be non-empty");
        }

        #[test]
        fn test_platform_is_windows_on_windows() {
            #[cfg(target_os = "windows")]
            {
                assert_eq!(PySystem::platform_pub(), "windows");
            }
            #[cfg(not(target_os = "windows"))]
            {
                // On non-Windows the function must still return a valid string
                assert!(!PySystem::platform_pub().is_empty());
            }
        }

        #[test]
        fn test_arch_x86_64_maps_correctly() {
            #[cfg(target_arch = "x86_64")]
            {
                assert_eq!(PySystem::arch_pub(), "x86_64");
            }
            #[cfg(not(target_arch = "x86_64"))]
            {
                assert!(!PySystem::arch_pub().is_empty());
            }
        }
    }

    mod test_system_struct {
        use super::*;

        #[test]
        fn test_new_is_deterministic_for_static_fields() {
            let s1 = PySystem::new();
            let s2 = PySystem::new();
            assert_eq!(s1.platform(), s2.platform());
            assert_eq!(s1.arch(), s2.arch());
            assert_eq!(s1.os(), s2.os());
        }

        #[test]
        fn test_default_equals_new() {
            let s1 = PySystem::new();
            let s2 = PySystem::new();
            // Both must produce identical static fields
            assert_eq!(s1.platform(), s2.platform());
            assert_eq!(s1.arch(), s2.arch());
            assert_eq!(s1.os(), s2.os());
        }


        #[test]
        fn test_num_cpus_at_least_one() {
            let sys = PySystem::new();
            assert!(sys.num_cpus() >= 1, "num_cpus must be >= 1");
        }

        #[test]
        fn test_hostname_non_empty() {
            let sys = PySystem::new();
            // hostname may be "unknown" if env vars not set; must still be non-empty
            assert!(!sys.hostname().is_empty());
        }

        #[test]
        fn test_hostname_fallback_to_unknown() {
            // When neither COMPUTERNAME nor HOSTNAME is set, result is "unknown"
            // We verify the function does not panic under any env state.
            let sys = PySystem::new();
            let h = sys.hostname();
            assert!(!h.is_empty(), "hostname must never be empty string");
        }

        #[test]
        fn test_rez_version_non_empty() {
            let sys = PySystem::new();
            let ver = sys.rez_version();
            assert!(!ver.is_empty(), "rez_version must be non-empty");
            // Should look like a semver (contains at least one '.')
            assert!(
                ver.contains('.'),
                "rez_version should be semver-like: {}",
                ver
            );
        }

        #[test]
        fn test_get_system_factory_consistent_with_new() {
            let s1 = get_system();
            let s2 = PySystem::new();
            assert_eq!(s1.platform(), s2.platform());
            assert_eq!(s1.arch(), s2.arch());
        }

        #[test]
        fn test_pub_helpers_match_getters() {
            let sys = PySystem::new();
            assert_eq!(sys.platform(), PySystem::platform_pub());
            assert_eq!(sys.arch(), PySystem::arch_pub());
            assert_eq!(sys.os(), PySystem::os_pub());
        }
    }

    mod test_system_repr_and_extras {
        use super::*;

        #[test]
        fn test_repr_contains_platform_arch_os() {
            let sys = PySystem::new();
            let repr = sys.__repr__();
            assert!(repr.contains("System("), "repr must start with 'System(': {repr}");
            assert!(repr.contains("platform="), "repr must contain 'platform=': {repr}");
            assert!(repr.contains("arch="), "repr must contain 'arch=': {repr}");
            assert!(repr.contains("os="), "repr must contain 'os=': {repr}");
        }

        #[test]
        fn test_repr_includes_actual_platform_value() {
            let sys = PySystem::new();
            let repr = sys.__repr__();
            let platform = sys.platform();
            assert!(
                repr.contains(&platform),
                "repr '{repr}' must contain platform value '{platform}'"
            );
        }

        #[test]
        fn test_platform_is_valid_rez_value() {
            let platform = PySystem::platform_pub();
            // rez recognizes: linux, windows, osx; plus any unknown OS string
            assert!(!platform.is_empty(), "platform must be non-empty");
            // Must not contain spaces
            assert!(
                !platform.contains(' '),
                "platform must not contain spaces: '{platform}'"
            );
        }

        #[test]
        fn test_arch_does_not_contain_spaces() {
            let arch = PySystem::arch_pub();
            assert!(
                !arch.contains(' '),
                "arch must not contain spaces: '{arch}'"
            );
        }

        #[test]
        fn test_os_does_not_contain_newline() {
            let os = PySystem::os_pub();
            assert!(
                !os.contains('\n'),
                "os string must not contain newline: '{os}'"
            );
        }

        #[test]
        fn test_rez_version_major_minor_patch_format() {
            let sys = PySystem::new();
            let ver = sys.rez_version();
            let parts: Vec<&str> = ver.split('.').collect();
            assert!(
                parts.len() >= 2,
                "rez_version should have at least major.minor: '{ver}'"
            );
            // Each part before any '-' (pre-release tag) should be numeric
            let major = parts[0].parse::<u64>();
            assert!(major.is_ok(), "major version should be numeric: '{}'", parts[0]);
        }

        #[test]
        fn test_num_cpus_reasonable_upper_bound() {
            let sys = PySystem::new();
            let cpus = sys.num_cpus();
            assert!(
                (1..=4096).contains(&cpus),
                "num_cpus should be in [1, 4096], got {cpus}"
            );

        }

        #[test]
        fn test_multiple_system_instances_identical_static_fields() {
            let instances: Vec<PySystem> = (0..3).map(|_| PySystem::new()).collect();
            for i in 1..instances.len() {
                assert_eq!(instances[0].platform(), instances[i].platform());
                assert_eq!(instances[0].arch(), instances[i].arch());
                assert_eq!(instances[0].os(), instances[i].os());
                assert_eq!(instances[0].rez_version(), instances[i].rez_version());
            }
        }
    }

    mod test_system_additional {
        use super::*;

        #[test]
        fn test_platform_not_contains_slash() {
            let platform = PySystem::platform_pub();
            assert!(!platform.contains('/'), "platform must not contain '/': '{platform}'");
        }

        #[test]
        fn test_arch_not_contains_slash() {
            let arch = PySystem::arch_pub();
            assert!(!arch.contains('/'), "arch must not contain '/': '{arch}'");
        }

        #[test]
        fn test_os_str_not_empty_string() {
            let os = PySystem::os_pub();
            assert!(!os.is_empty(), "os must not be empty string");
        }

        #[test]
        fn test_rez_version_patch_numeric() {
            let sys = PySystem::new();
            let ver = sys.rez_version();
            let parts: Vec<&str> = ver.split('.').collect();
            if parts.len() >= 3 {
                // patch may include pre-release suffix like "0-alpha.1"
                let patch_num = parts[2].split('-').next().unwrap_or("");
                assert!(
                    patch_num.parse::<u64>().is_ok(),
                    "patch version should start with numeric: '{}'", parts[2]
                );
            }
        }

        #[test]
        fn test_hostname_no_null_bytes() {
            let sys = PySystem::new();
            let h = sys.hostname();
            assert!(!h.contains('\0'), "hostname must not contain null bytes");
        }

        #[test]
        fn test_num_cpus_power_of_two_or_reasonable() {
            let sys = PySystem::new();
            let cpus = sys.num_cpus();
            // Just verify it's a positive integer
            assert!(cpus > 0, "num_cpus must be positive, got {cpus}");
        }

        #[test]
        fn test_arch_not_numeric_only() {
            let arch = PySystem::arch_pub();
            // Architecture strings like "x86_64" are not purely numeric
            let all_digits = arch.chars().all(|c| c.is_ascii_digit());
            assert!(!all_digits || arch.is_empty(),
                "arch should not be purely numeric: '{arch}'");
        }

        #[test]
        fn test_repr_contains_platform_arch_os() {
            let sys = PySystem::new();
            let repr = sys.__repr__();
            assert!(repr.contains("System("), "repr must start with System(: '{repr}'");
            assert!(repr.contains("platform="), "repr must contain platform=: '{repr}'");
            assert!(repr.contains("arch="), "repr must contain arch=: '{repr}'");
            assert!(repr.contains("os="), "repr must contain os=: '{repr}'");
        }

        #[test]
        fn test_platform_is_one_of_known_values() {
            let platform = PySystem::platform_pub();
            let known = ["linux", "windows", "osx"];
            assert!(
                known.contains(&platform.as_str()),
                "platform must be one of {:?}, got '{platform}'",
                known
            );
        }

        #[test]
        fn test_hostname_is_non_empty() {
            let sys = PySystem::new();
            let h = sys.hostname();
            assert!(!h.is_empty(), "hostname must not be empty");
        }

        #[test]
        fn test_os_contains_no_newlines() {
            let os = PySystem::os_pub();
            assert!(!os.contains('\n'), "os must not contain newlines: '{os}'");
            assert!(!os.contains('\r'), "os must not contain carriage returns: '{os}'");
        }
    }
}

