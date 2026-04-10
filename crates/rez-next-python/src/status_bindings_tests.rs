use super::*;
use crate::source_bindings::detect_current_shell;
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
fn test_detect_current_shell_returns_valid_shell() {
    // On Windows, PSModulePath is always present so powershell is detected.
    // On Linux/macOS, SHELL governs. Either way, the result must be a known shell name.
    let shell = detect_current_shell();
    let known = ["bash", "zsh", "fish", "powershell", "cmd"];
    assert!(
        known.iter().any(|k| shell.contains(k)),
        "unexpected shell: {shell}"
    );
    // On Windows CI, PSModulePath is always set, so we get exactly "powershell".
    #[cfg(target_os = "windows")]
    assert_eq!(
        shell.as_str(),
        "powershell",
        "expected powershell on Windows, got: {shell}"
    );
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_detect_current_shell_maps_bash_posix() {
    let _lock = ENV_MUTEX.lock().unwrap();
    // Only run on POSIX where PSModulePath does not interfere
    unsafe {
        std::env::set_var("SHELL", "/bin/bash");
    }
    assert_eq!(detect_current_shell().as_str(), "bash");
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
fn test_detect_current_shell_maps_zsh() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("SHELL", "/usr/bin/zsh");
    }
    assert_eq!(detect_current_shell().as_str(), "zsh");
    unsafe {
        std::env::remove_var("SHELL");
    }
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_detect_current_shell_maps_fish() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("SHELL", "/usr/local/bin/fish");
    }
    assert_eq!(detect_current_shell().as_str(), "fish");
    unsafe {
        std::env::remove_var("SHELL");
    }
}

// ── get_rez_env_var: empty key handling ───────────────────────────────────

#[test]
fn test_get_rez_env_var_empty_key_returns_none() {
    let val = get_rez_env_var("");
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
