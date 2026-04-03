use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Rex DSL advanced command semantics (Phase 93) ────────────────────────

/// rez rex: info() records a diagnostic message (does not affect env vars)
#[test]
fn test_rex_info_does_not_affect_env() {
    use rez_next_rex::RexExecutor;

    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(r#"info("Loading package mypkg")"#, "mypkg", None, None)
        .unwrap();
    // info() should not create any env vars
    assert!(
        env.vars.is_empty(),
        "info() should not set any env var; vars: {:?}",
        env.vars
    );
}

/// rez rex: setenv_if_empty only sets var when absent (not overwrite)
#[test]
fn test_rex_setenv_if_empty_absent_sets_value() {
    use rez_next_rex::RexExecutor;

    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"env.setenv_if_empty("NEW_VAR", "initial")"#,
            "mypkg",
            None,
            None,
        )
        .unwrap();
    assert_eq!(
        env.vars.get("NEW_VAR").map(String::as_str),
        Some("initial"),
        "setenv_if_empty should set value when variable is absent"
    );
}

/// rez rex: mixed setenv + append_path in single commands string
#[test]
fn test_rex_mixed_setenv_and_append_path() {
    use rez_next_rex::RexExecutor;

    let mut exec = RexExecutor::new();
    let cmds = r#"
env.setenv('PKG_HOME', '/opt/pkg/2.0')
env.append_path('PATH', '/opt/pkg/2.0/bin')
env.append_path('LD_LIBRARY_PATH', '/opt/pkg/2.0/lib')
"#;
    let env = exec
        .execute_commands(cmds, "pkg", Some("/opt/pkg/2.0"), Some("2.0"))
        .unwrap();
    assert_eq!(
        env.vars.get("PKG_HOME").map(String::as_str),
        Some("/opt/pkg/2.0")
    );
    assert!(
        env.vars
            .get("PATH")
            .map(|v| v.contains("/opt/pkg/2.0/bin"))
            .unwrap_or(false),
        "PATH should contain the bin dir"
    );
    assert!(
        env.vars
            .get("LD_LIBRARY_PATH")
            .map(|v| v.contains("/opt/pkg/2.0/lib"))
            .unwrap_or(false),
        "LD_LIBRARY_PATH should contain the lib dir"
    );
}

/// rez rex: context var {name} expansion for package name
#[test]
fn test_rex_context_var_name_expansion() {
    use rez_next_rex::RexExecutor;

    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"env.setenv("ACTIVE_PKG", "{name}")"#,
            "myspecialpkg",
            None,
            None,
        )
        .unwrap();
    assert_eq!(
        env.vars.get("ACTIVE_PKG").map(String::as_str),
        Some("myspecialpkg"),
        "{{name}} should expand to the package name"
    );
}

/// rez rex: three-pkg sequential PATH accumulation preserves order
#[test]
fn test_rex_three_pkg_path_order() {
    use rez_next_rex::RexExecutor;

    let mut exec = RexExecutor::new();
    exec.execute_commands(
        r#"env.prepend_path("PATH", "/pkg_c/bin")"#,
        "pkgC",
        None,
        None,
    )
    .unwrap();
    exec.execute_commands(
        r#"env.prepend_path("PATH", "/pkg_b/bin")"#,
        "pkgB",
        None,
        None,
    )
    .unwrap();
    let env = exec
        .execute_commands(
            r#"env.prepend_path("PATH", "/pkg_a/bin")"#,
            "pkgA",
            None,
            None,
        )
        .unwrap();
    let path = env.vars.get("PATH").cloned().unwrap_or_default();
    // Each prepend goes to front, so: pkgA < pkgB < pkgC (position-wise)
    let pos_a = path.find("/pkg_a/bin").unwrap_or(999);
    let pos_b = path.find("/pkg_b/bin").unwrap_or(999);
    let pos_c = path.find("/pkg_c/bin").unwrap_or(999);
    assert!(
        pos_a < pos_b,
        "pkgA (last prepended) should precede pkgB; PATH={}",
        path
    );
    assert!(pos_b < pos_c, "pkgB should precede pkgC; PATH={}", path);
}

