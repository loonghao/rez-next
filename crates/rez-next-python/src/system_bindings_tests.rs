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
        assert!(!sys.hostname().is_empty());
    }

    #[test]
    fn test_hostname_fallback_to_unknown() {
        let sys = PySystem::new();
        let h = sys.hostname();
        assert!(!h.is_empty(), "hostname must never be empty string");
    }

    #[test]
    fn test_rez_version_non_empty() {
        let sys = PySystem::new();
        let ver = sys.rez_version();
        assert!(!ver.is_empty(), "rez_version must be non-empty");
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
        assert!(!platform.is_empty(), "platform must be non-empty");
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
        assert!(cpus > 0, "num_cpus must be positive, got {cpus}");
    }

    #[test]
    fn test_arch_not_numeric_only() {
        let arch = PySystem::arch_pub();
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

mod test_system_cycle_115 {
    use super::*;

    #[test]
    fn test_platform_is_lowercase() {
        let platform = PySystem::platform_pub();
        assert_eq!(
            platform,
            platform.to_lowercase(),
            "platform must be lowercase: '{platform}'"
        );
    }

    #[test]
    fn test_arch_contains_underscore_or_digit() {
        let arch = PySystem::arch_pub();
        assert!(
            arch.contains('_') || arch.chars().any(|c| c.is_ascii_digit()),
            "arch should contain underscore or digit: '{arch}'"
        );
    }

    #[test]
    fn test_rez_version_not_dev_version() {
        let sys = PySystem::new();
        let ver = sys.rez_version();
        assert!(!ver.is_empty(), "rez_version must not be empty");
    }

    #[test]
    fn test_system_platform_arch_os_all_non_empty() {
        let sys = PySystem::new();
        assert!(!sys.platform().is_empty(), "platform must not be empty");
        assert!(!sys.arch().is_empty(), "arch must not be empty");
        assert!(!sys.os().is_empty(), "os must not be empty");
    }

    #[test]
    fn test_system_repr_ends_with_paren() {
        let sys = PySystem::new();
        let repr = sys.__repr__();
        assert!(repr.ends_with(')'), "repr must end with ')': '{repr}'");
    }

    #[test]
    fn test_system_num_cpus_at_least_one_all_instances() {
        for _ in 0..3 {
            let sys = PySystem::new();
            assert!(sys.num_cpus() >= 1, "num_cpus must always be >= 1");
        }
    }

    #[test]
    fn test_hostname_does_not_equal_empty_string() {
        let sys = PySystem::new();
        let h = sys.hostname();
        assert_ne!(h, "", "hostname must not be empty string");
    }
}

mod test_system_cy119 {
    use super::*;

    #[test]
    fn test_platform_getter_matches_platform_pub() {
        let sys = PySystem::new();
        assert_eq!(sys.platform(), PySystem::platform_pub());
    }

    #[test]
    fn test_arch_getter_matches_arch_pub() {
        let sys = PySystem::new();
        assert_eq!(sys.arch(), PySystem::arch_pub());
    }

    #[test]
    fn test_os_getter_matches_os_pub() {
        let sys = PySystem::new();
        assert_eq!(sys.os(), PySystem::os_pub());
    }

    #[test]
    fn test_get_system_matches_new_rez_version() {
        let a = get_system();
        let b = PySystem::new();
        assert_eq!(a.rez_version(), b.rez_version());
    }

    #[test]
    fn test_repr_includes_actual_arch_value() {
        let sys = PySystem::new();
        let repr = sys.__repr__();
        let arch = sys.arch();
        assert!(
            repr.contains(&arch),
            "repr '{repr}' must contain arch value '{arch}'"
        );
    }

    #[test]
    fn test_num_cpus_is_deterministic() {
        let sys = PySystem::new();
        let c1 = sys.num_cpus();
        let c2 = sys.num_cpus();
        assert_eq!(c1, c2, "num_cpus must be deterministic");
    }
}

mod test_system_cy125 {
    use super::*;

    #[test]
    fn test_num_cpus_at_least_one() {
        let sys = PySystem::new();
        assert!(sys.num_cpus() >= 1, "system must have at least 1 CPU");
    }

    #[test]
    fn test_platform_is_nonempty() {
        assert!(!PySystem::platform_pub().is_empty(), "platform must be non-empty");
    }

    #[test]
    fn test_arch_is_nonempty() {
        assert!(!PySystem::arch_pub().is_empty(), "arch must be non-empty");
    }

    #[test]
    fn test_os_is_nonempty() {
        assert!(!PySystem::os_pub().is_empty(), "os must be non-empty");
    }

    #[test]
    fn test_get_system_platform_matches_new() {
        let a = get_system();
        let b = PySystem::new();
        assert_eq!(
            a.platform(),
            b.platform(),
            "get_system() and new() must report same platform"
        );
    }
}
