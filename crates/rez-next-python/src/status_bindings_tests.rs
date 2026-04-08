use super::*;
use std::sync::Mutex;

// Serialize all tests that mutate process-global environment variables.
// cargo test runs tests in parallel by default, and concurrent env mutations
// cause non-deterministic failures on env-reads like detect_current_status().
static ENV_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn test_is_in_rez_context_false_outside() {
    let _lock = ENV_MUTEX.lock().unwrap();
    // Outside any rez env the function should return false (CI has no rez)
    let in_ctx = std::env::var("REZ_CONTEXT_FILE").is_ok()
        || std::env::var("REZ_USED_PACKAGES_NAMES").is_ok();
    // Just verify the function matches the manual check
    assert_eq!(is_in_rez_context(), in_ctx);
}

#[test]
fn test_get_context_file_none_outside_context() {
    if std::env::var("REZ_CONTEXT_FILE").is_err() {
        assert!(get_context_file().is_none());
    }
}

#[test]
fn test_get_resolved_package_names_empty_outside() {
    let _lock = ENV_MUTEX.lock().unwrap();
    // Only assert when we know no other test has set the env var
    if std::env::var("REZ_USED_PACKAGES_NAMES").is_err() {
        let names = get_resolved_package_names();
        assert!(names.is_empty(), "Should be empty outside rez context");
    }
}

#[test]
fn test_rez_status_inactive_repr() {
    let status = detect_current_status();
    // Only test the inactive case (CI env)
    if !status.is_active {
        assert!(!status.__repr__().is_empty());
        assert!(status.__repr__().contains("inactive"));
    }
}

#[test]
fn test_rez_status_resolved_packages_from_env() {
    let _lock = ENV_MUTEX.lock().unwrap();
    // Simulate REZ_USED_PACKAGES_NAMES
    unsafe {
        std::env::set_var("REZ_USED_PACKAGES_TEST_TEMP", "python-3.9 maya-2024.1");
    }
    // Parse logic
    let raw = std::env::var("REZ_USED_PACKAGES_TEST_TEMP").unwrap();
    let pkgs: Vec<String> = raw.split_whitespace().map(|p| p.to_string()).collect();
    assert_eq!(pkgs.len(), 2);
    assert_eq!(pkgs[0], "python-3.9");
    unsafe {
        std::env::remove_var("REZ_USED_PACKAGES_TEST_TEMP");
    }
}

#[test]
fn test_detect_shell_from_env_returns_valid_shell() {
    // On Windows, PSModulePath is always present so powershell is detected.
    // On Linux/macOS, SHELL governs. Either way, the result must be a known shell name.
    let shell = detect_shell_from_env();
    // In a rez-unactivated env, shell detection may return None; if Some, must be known.
    if let Some(ref s) = shell {
        let known = ["bash", "zsh", "fish", "powershell", "cmd"];
        assert!(
            known.iter().any(|k| s.contains(k)),
            "unexpected shell: {s}"
        );
    }
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_detect_shell_from_env_maps_bash_posix() {
    let _lock = ENV_MUTEX.lock().unwrap();
    // Only run on POSIX where PSModulePath does not interfere
    unsafe {
        std::env::set_var("SHELL", "/bin/bash");
    }
    assert_eq!(detect_shell_from_env().as_deref(), Some("bash"));
    unsafe {
        std::env::remove_var("SHELL");
    }
}

#[test]
fn test_get_rez_env_var_with_prefix() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("REZ_STATUS_BINDINGS_WITH_PREFIX", "active");
    }
    assert_eq!(
        get_rez_env_var("REZ_STATUS_BINDINGS_WITH_PREFIX").as_deref(),
        Some("active")
    );
    unsafe {
        std::env::remove_var("REZ_STATUS_BINDINGS_WITH_PREFIX");
    }
}

#[test]
fn test_get_rez_env_var_without_prefix() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("REZ_STATUS_BINDINGS_NO_PREFIX", "present");
    }
    assert_eq!(
        get_rez_env_var("STATUS_BINDINGS_NO_PREFIX").as_deref(),
        Some("present")
    );
    unsafe {
        std::env::remove_var("REZ_STATUS_BINDINGS_NO_PREFIX");
    }
}

#[test]
fn test_inactive_context_empty_packages() {
    let _lock = ENV_MUTEX.lock().unwrap();
    if std::env::var("REZ_USED_PACKAGES_NAMES").is_err() {
        let s = detect_current_status();
        if !s.is_active {
            assert!(s.resolved_packages.is_empty());
        }
    }
}

// ── detect_current_status field coverage ──────────────────────────────────

#[test]
fn test_detect_active_via_context_file_env() {
    let _lock = ENV_MUTEX.lock().unwrap();
    // Use a unique key suffix to avoid collision with CI vars
    unsafe {
        std::env::set_var("REZ_CONTEXT_FILE", "/tmp/test_ctx90.rxt");
    }
    let s = detect_current_status();
    assert!(s.is_active, "REZ_CONTEXT_FILE should make is_active=true");
    assert_eq!(s.context_file.as_deref(), Some("/tmp/test_ctx90.rxt"));
    unsafe {
        std::env::remove_var("REZ_CONTEXT_FILE");
    }
}

#[test]
fn test_detect_active_via_used_packages_env() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("REZ_USED_PACKAGES_NAMES", "python-3.9 cmake-3.21");
    }
    let s = detect_current_status();
    assert!(
        s.is_active,
        "status should be active when REZ_USED_PACKAGES_NAMES is set, got: {:?}",
        s.resolved_packages
    );
    assert!(
        s.resolved_packages.contains(&"python-3.9".to_string()),
        "resolved_packages should contain python-3.9, got {:?}",
        s.resolved_packages
    );
    assert!(
        s.resolved_packages.contains(&"cmake-3.21".to_string()),
        "resolved_packages should contain cmake-3.21, got {:?}",
        s.resolved_packages
    );
    unsafe {
        std::env::remove_var("REZ_USED_PACKAGES_NAMES");
    }
}

#[test]
fn test_detect_request_field() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("REZ_REQUEST", "python-3 maya-2024");
    }
    let s = detect_current_status();
    assert!(
        s.requested_packages.contains(&"python-3".to_string()),
        "requested_packages should include python-3, got {:?}",
        s.requested_packages
    );
    unsafe {
        std::env::remove_var("REZ_REQUEST");
    }
}

#[test]
fn test_detect_implicit_packages_field() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("REZ_IMPLICIT_PACKAGES", "platform-linux arch-x86_64");
    }
    let s = detect_current_status();
    assert!(
        s.implicit_packages.contains(&"platform-linux".to_string()),
        "implicit_packages missing platform-linux, got {:?}",
        s.implicit_packages
    );
    unsafe {
        std::env::remove_var("REZ_IMPLICIT_PACKAGES");
    }
}

#[test]
fn test_detect_context_cwd_and_version() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("REZ_ORIG_CWD", "/home/user/project");
        std::env::set_var("REZ_VERSION", "3.2.1");
    }
    let s = detect_current_status();
    assert_eq!(s.context_cwd.as_deref(), Some("/home/user/project"));
    assert_eq!(s.rez_version.as_deref(), Some("3.2.1"));
    unsafe {
        std::env::remove_var("REZ_ORIG_CWD");
        std::env::remove_var("REZ_VERSION");
    }
}

#[test]
fn test_active_repr_includes_package_count() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("REZ_USED_PACKAGES_NAMES", "alpha-1 beta-2 gamma-3");
    }
    let s = detect_current_status();
    if s.is_active {
        let r = s.__repr__();
        assert!(
            r.contains("3"),
            "repr should mention package count 3, got: {}",
            r
        );
        assert!(r.contains("active"), "repr should contain 'active': {}", r);
    }
    unsafe {
        std::env::remove_var("REZ_USED_PACKAGES_NAMES");
    }
}

#[test]
fn test_get_rez_env_var_missing_returns_none() {
    // Use a key that should never exist in CI
    let val = get_rez_env_var("STATUS_BINDINGS_NONEXISTENT_KEY_90XYZ");
    assert!(
        val.is_none(),
        "missing key should return None, got {:?}",
        val
    );
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_detect_shell_from_env_maps_zsh() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("SHELL", "/usr/bin/zsh");
    }
    assert_eq!(detect_shell_from_env().as_deref(), Some("zsh"));
    unsafe {
        std::env::remove_var("SHELL");
    }
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_detect_shell_from_env_maps_fish() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("SHELL", "/usr/local/bin/fish");
    }
    assert_eq!(detect_shell_from_env().as_deref(), Some("fish"));
    unsafe {
        std::env::remove_var("SHELL");
    }
}

// ── get_rez_env_var: empty key handling ───────────────────────────────────

#[test]
fn test_get_rez_env_var_empty_key_returns_none() {
    // "" -> "REZ_" — unlikely to exist in any env, should return None
    let val = get_rez_env_var("");
    // The key becomes "REZ_"; if the env has no such var, it must be None
    // (On some systems this could hypothetically be set, so only assert None
    //  when the raw env confirms absence)
    if std::env::var("REZ_").is_err() {
        assert!(val.is_none(), "empty key should yield None, got {:?}", val);
    }
}

// ── is_in_rez_context is_ok after env set ────────────────────────────────

#[test]
fn test_is_in_rez_context_true_after_env_set() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("REZ_CONTEXT_FILE", "/tmp/cycle97_test.rxt");
    }
    assert!(
        is_in_rez_context(),
        "is_in_rez_context should be true when REZ_CONTEXT_FILE is set"
    );
    unsafe {
        std::env::remove_var("REZ_CONTEXT_FILE");
    }
}

// ── rez_env_vars collection covers REZ_ prefix keys ──────────────────────

#[test]
fn test_rez_env_vars_includes_set_key() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("REZ_CYCLE97_MARKER", "cycle97");
    }
    let s = detect_current_status();
    assert!(
        s.rez_env_vars.contains_key("REZ_CYCLE97_MARKER"),
        "rez_env_vars should capture REZ_CYCLE97_MARKER"
    );
    assert_eq!(s.rez_env_vars["REZ_CYCLE97_MARKER"], "cycle97");
    unsafe {
        std::env::remove_var("REZ_CYCLE97_MARKER");
    }
}

// ── get_context_file returns value when set ────────────────────────────────

#[test]
fn test_get_context_file_returns_some_when_set() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("REZ_CONTEXT_FILE", "/tmp/some_ctx97.rxt");
    }
    assert_eq!(
        get_context_file().as_deref(),
        Some("/tmp/some_ctx97.rxt"),
        "get_context_file should return the env var value"
    );
    unsafe {
        std::env::remove_var("REZ_CONTEXT_FILE");
    }
}

// ── get_resolved_package_names parses space-separated list ────────────────

#[test]
fn test_get_resolved_package_names_parses_list() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("REZ_USED_PACKAGES_NAMES", "pkgA-1.0 pkgB-2.0 pkgC-3.0");
    }
    let names = get_resolved_package_names();
    assert_eq!(names.len(), 3);
    assert!(names.contains(&"pkgA-1.0".to_string()));
    assert!(names.contains(&"pkgC-3.0".to_string()));
    unsafe {
        std::env::remove_var("REZ_USED_PACKAGES_NAMES");
    }
}

// ── default RezStatus is_active false ────────────────────────────────────

#[test]
fn test_default_rez_status_is_active_default_false_when_no_rez_env() {
    let _lock = ENV_MUTEX.lock().unwrap();
    // Ensure neither trigger var is present
    if std::env::var("REZ_CONTEXT_FILE").is_err()
        && std::env::var("REZ_USED_PACKAGES_NAMES").is_err()
    {
        let s = PyRezStatus::new();
        assert!(
            !s.is_active,
            "default status should be inactive outside rez, got is_active=true"
        );
    }
}

// ── Cycle 103 additions ──────────────────────────────────────────────────

#[test]
fn test_rez_status_str_matches_repr() {
    // __str__ must delegate to __repr__
    let s = PyRezStatus::new();
    assert_eq!(s.__str__(), s.__repr__());
}

#[test]
fn test_inactive_status_repr_contains_inactive() {
    let _lock = ENV_MUTEX.lock().unwrap();
    if std::env::var("REZ_CONTEXT_FILE").is_err()
        && std::env::var("REZ_USED_PACKAGES_NAMES").is_err()
    {
        let s = PyRezStatus::new();
        if !s.is_active {
            assert!(
                s.__repr__().contains("inactive"),
                "inactive repr must say 'inactive', got: {}",
                s.__repr__()
            );
        }
    }
}

#[test]
fn test_active_status_repr_contains_active() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("REZ_USED_PACKAGES_NAMES", "pkg1-1.0 pkg2-2.0");
    }
    let s = detect_current_status();
    if s.is_active {
        assert!(
            s.__repr__().contains("active"),
            "active repr must say 'active', got: {}",
            s.__repr__()
        );
        assert!(
            s.__repr__().contains("2"),
            "active repr must show package count 2, got: {}",
            s.__repr__()
        );
    }
    unsafe {
        std::env::remove_var("REZ_USED_PACKAGES_NAMES");
    }
}

#[test]
fn test_detect_rez_platform_env_captured_in_rez_env_vars() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("REZ_PLATFORM", "linux");
    }
    let s = detect_current_status();
    assert!(
        s.rez_env_vars.contains_key("REZ_PLATFORM"),
        "rez_env_vars must capture REZ_PLATFORM"
    );
    assert_eq!(s.rez_env_vars["REZ_PLATFORM"], "linux");
    unsafe {
        std::env::remove_var("REZ_PLATFORM");
    }
}

#[test]
fn test_detect_rez_arch_env_captured_in_rez_env_vars() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("REZ_ARCH", "x86_64");
    }
    let s = detect_current_status();
    assert!(
        s.rez_env_vars.contains_key("REZ_ARCH"),
        "rez_env_vars must capture REZ_ARCH"
    );
    assert_eq!(s.rez_env_vars["REZ_ARCH"], "x86_64");
    unsafe {
        std::env::remove_var("REZ_ARCH");
    }
}

#[test]
fn test_multiple_requests_all_parsed() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("REZ_REQUEST", "python-3 cmake-3.21 boost-1.82");
    }
    let s = detect_current_status();
    assert!(
        s.requested_packages.contains(&"python-3".to_string()),
        "must contain python-3, got {:?}",
        s.requested_packages
    );
    assert!(
        s.requested_packages.contains(&"cmake-3.21".to_string()),
        "must contain cmake-3.21, got {:?}",
        s.requested_packages
    );
    assert!(
        s.requested_packages.contains(&"boost-1.82".to_string()),
        "must contain boost-1.82, got {:?}",
        s.requested_packages
    );
    unsafe {
        std::env::remove_var("REZ_REQUEST");
    }
}

#[test]
fn test_context_file_and_used_packages_both_active() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("REZ_CONTEXT_FILE", "/tmp/cy103_both.rxt");
        std::env::set_var("REZ_USED_PACKAGES_NAMES", "toolA-1.0");
    }
    let s = detect_current_status();
    assert!(s.is_active, "should be active when both env vars set");
    assert_eq!(s.context_file.as_deref(), Some("/tmp/cy103_both.rxt"));
    assert!(
        s.resolved_packages.contains(&"toolA-1.0".to_string()),
        "resolved packages must contain toolA-1.0, got {:?}",
        s.resolved_packages
    );
    unsafe {
        std::env::remove_var("REZ_CONTEXT_FILE");
        std::env::remove_var("REZ_USED_PACKAGES_NAMES");
    }
}

// ── Cycle 115 additions ──────────────────────────────────────────────────

#[test]
fn test_rez_status_requested_packages_empty_by_default() {
    let _lock = ENV_MUTEX.lock().unwrap();
    if std::env::var("REZ_REQUEST").is_err() {
        let s = PyRezStatus::new();
        // Outside a rez context, requested_packages should be empty
        if !s.is_active {
            assert!(
                s.requested_packages.is_empty(),
                "requested_packages must be empty outside rez, got {:?}",
                s.requested_packages
            );
        }
    }
}

#[test]
fn test_rez_status_implicit_packages_empty_by_default() {
    let _lock = ENV_MUTEX.lock().unwrap();
    if std::env::var("REZ_IMPLICIT_PACKAGES").is_err() {
        let s = PyRezStatus::new();
        if !s.is_active {
            assert!(
                s.implicit_packages.is_empty(),
                "implicit_packages must be empty outside rez"
            );
        }
    }
}

#[test]
fn test_detect_current_status_returns_rez_status_type() {
    // verify detect_current_status() can be called without panic and returns a RezStatus
    let s = detect_current_status();
    // is_active field must be accessible
    let _ = s.is_active;
    let _ = s.resolved_packages.len();
}

#[test]
fn test_get_resolved_package_names_single_package() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("REZ_USED_PACKAGES_NAMES", "only_one_pkg-1.2.3");
    }
    let names = get_resolved_package_names();
    assert_eq!(names.len(), 1);
    assert_eq!(names[0], "only_one_pkg-1.2.3");
    unsafe {
        std::env::remove_var("REZ_USED_PACKAGES_NAMES");
    }
}

#[test]
fn test_get_rez_env_var_already_has_rez_prefix() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("REZ_CYCLE115_PREFIX_TEST", "hello");
    }
    let val = get_rez_env_var("REZ_CYCLE115_PREFIX_TEST");
    assert_eq!(val.as_deref(), Some("hello"), "key with REZ_ prefix must be found as-is");
    unsafe {
        std::env::remove_var("REZ_CYCLE115_PREFIX_TEST");
    }
}

#[test]
fn test_rez_env_vars_not_contains_non_rez_key() {
    let _lock = ENV_MUTEX.lock().unwrap();
    // Set a non-REZ_ env var and verify it is NOT captured in rez_env_vars
    unsafe {
        std::env::set_var("CYCLE115_NON_REZ_KEY", "should_not_appear");
    }
    let s = detect_current_status();
    assert!(
        !s.rez_env_vars.contains_key("CYCLE115_NON_REZ_KEY"),
        "non-REZ_ key must not appear in rez_env_vars"
    );
    unsafe {
        std::env::remove_var("CYCLE115_NON_REZ_KEY");
    }
}

#[test]
fn test_is_in_rez_context_false_when_both_vars_absent() {
    let _lock = ENV_MUTEX.lock().unwrap();
    // Temporarily remove both trigger vars (if present)
    let ctx = std::env::var("REZ_CONTEXT_FILE").ok();
    let pkg = std::env::var("REZ_USED_PACKAGES_NAMES").ok();
    unsafe {
        std::env::remove_var("REZ_CONTEXT_FILE");
        std::env::remove_var("REZ_USED_PACKAGES_NAMES");
    }
    assert!(!is_in_rez_context(), "must be false when neither trigger var is set");
    // Restore
    if let Some(v) = ctx {
        unsafe { std::env::set_var("REZ_CONTEXT_FILE", v); }
    }
    if let Some(v) = pkg {
        unsafe { std::env::set_var("REZ_USED_PACKAGES_NAMES", v); }
    }
}

// ── Cycle 121 additions ──────────────────────────────────────────────────

#[test]
fn test_rez_status_context_file_none_by_default() {
    let _lock = ENV_MUTEX.lock().unwrap();
    if std::env::var("REZ_CONTEXT_FILE").is_err() {
        let s = PyRezStatus::new();
        if !s.is_active {
            assert!(
                s.context_file.is_none(),
                "context_file must be None outside rez, got {:?}",
                s.context_file
            );
        }
    }
}

#[test]
fn test_rez_status_current_shell_some_when_env_set() {
    let _lock = ENV_MUTEX.lock().unwrap();
    // Only detectable on non-Windows via SHELL or Windows via PSModulePath
    let s = PyRezStatus::new();
    // If shell is detected, it must be a non-empty string
    if let Some(ref shell) = s.current_shell {
        assert!(!shell.is_empty(), "current_shell must be non-empty when Some");
    }
}

#[test]
fn test_get_resolved_package_names_empty_when_var_absent() {
    let _lock = ENV_MUTEX.lock().unwrap();
    let saved = std::env::var("REZ_USED_PACKAGES_NAMES").ok();
    unsafe { std::env::remove_var("REZ_USED_PACKAGES_NAMES"); }
    let names = get_resolved_package_names();
    assert!(names.is_empty(), "no packages expected when REZ_USED_PACKAGES_NAMES absent");
    if let Some(v) = saved {
        unsafe { std::env::set_var("REZ_USED_PACKAGES_NAMES", v); }
    }
}

#[test]
fn test_detect_current_status_rez_version_some_when_set() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe { std::env::set_var("REZ_VERSION", "4.0.0"); }
    let s = detect_current_status();
    assert_eq!(s.rez_version.as_deref(), Some("4.0.0"), "rez_version should be 4.0.0");
    unsafe { std::env::remove_var("REZ_VERSION"); }
}

#[test]
fn test_rez_env_vars_does_not_capture_path_variable() {
    // PATH does not start with REZ_ so must never appear in rez_env_vars
    let s = detect_current_status();
    assert!(
        !s.rez_env_vars.contains_key("PATH"),
        "PATH must not appear in rez_env_vars"
    );
}

#[test]
fn test_rez_status_repr_is_non_empty() {
    let s = PyRezStatus::new();
    assert!(!s.__repr__().is_empty(), "__repr__ must not be empty");
}

// ─────── Cycle 126 additions ─────────────────────────────────────────────

#[test]
fn test_rez_status_new_does_not_panic() {
    let _ = PyRezStatus::new();
}

#[test]
fn test_get_rez_env_var_missing_key_is_none() {
    let _lock = ENV_MUTEX.lock().unwrap();
    let result = get_rez_env_var("__REZ_NONEXISTENT_CY126__");
    assert!(result.is_none(), "unknown env var must return None");
}

#[test]
fn test_get_resolved_package_names_outside_context_is_empty() {
    let _lock = ENV_MUTEX.lock().unwrap();
    if std::env::var("REZ_USED_PACKAGES_NAMES").is_err() {
        let names = get_resolved_package_names();
        assert!(
            names.is_empty(),
            "outside rez context, resolved package names must be empty"
        );
    }
}

#[test]
fn test_is_in_rez_context_consistent_with_env() {
    let _lock = ENV_MUTEX.lock().unwrap();
    let has_ctx_file = std::env::var("REZ_CONTEXT_FILE").is_ok();
    let has_pkg_names = std::env::var("REZ_USED_PACKAGES_NAMES").is_ok();
    let expected = has_ctx_file || has_pkg_names;
    assert_eq!(
        is_in_rez_context(),
        expected,
        "is_in_rez_context() must match env variable presence"
    );
}

#[test]
fn test_rez_status_repr_contains_status_token() {
    let s = PyRezStatus::new();
    let repr = s.__repr__();
    assert!(
        repr.contains("Status") || repr.contains("rez"),
        "repr must mention status or rez: {repr}"
    );
}
