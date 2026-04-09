//! Advanced unit tests for `status_bindings` — Cycles 115, 121, 126 additions.
//! Split from status_bindings_tests.rs (Cycle 147) to keep file size ≤400 lines.

use super::*;
use std::sync::Mutex;

static ENV_MUTEX: Mutex<()> = Mutex::new(());

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
    let s = detect_current_status();
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
    let ctx = std::env::var("REZ_CONTEXT_FILE").ok();
    let pkg = std::env::var("REZ_USED_PACKAGES_NAMES").ok();
    unsafe {
        std::env::remove_var("REZ_CONTEXT_FILE");
        std::env::remove_var("REZ_USED_PACKAGES_NAMES");
    }
    assert!(!is_in_rez_context(), "must be false when neither trigger var is set");
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
    let s = PyRezStatus::new();
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
