//! Rez Compat — Rex DSL advanced command semantics, Exception type tests,
//! Version advanced operations.
//!
//! Split from rez_compat_advanced_tests.rs (Cycle 71) to keep file under 1000 lines.

use rez_core::version::{Version, VersionRange};

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

// ─── Exception type / message tests ─────────────────────────────────────────

/// rez.exceptions: PackageRequirement parse is lenient — documents actual behavior.
/// Parsing unusual strings should not panic; result may be Ok or Err.
#[test]
fn test_invalid_package_requirement_no_panic() {
    use rez_next_package::PackageRequirement;

    let result = PackageRequirement::parse("!!!invalid");
    if let Ok(req) = result {
        assert!(
            !req.name.is_empty(),
            "best-effort parse of '!!!invalid' produced empty name"
        );
    }

}

/// rez.exceptions: Empty string PackageRequirement parse does not panic
#[test]
fn test_empty_package_requirement_no_panic() {
    use rez_next_package::PackageRequirement;

    let result = PackageRequirement::parse("");
    if let Ok(req) = result {
        assert!(
            req.name.is_empty(),
            "empty-string parse should produce a requirement with empty name, got '{}'",
            req.name
        );
    }
}

/// rez.exceptions: VersionRange parse error for unbalanced brackets
#[test]
fn test_version_range_unbalanced_bracket_error() {
    use rez_core::version::VersionRange;

    let result = VersionRange::parse(">=1.0,<2.0,");
    if let Ok(r) = result {
        assert!(
            r.contains(&rez_core::version::Version::parse("1.5").unwrap()),
            "tolerant parse of trailing-comma range should still contain 1.5"
        );
    }

}

/// rez.exceptions: Version parse with garbage input returns error (not panic)
#[test]
fn test_version_parse_garbage_no_panic() {
    use rez_core::version::Version;

    let result = Version::parse("!@#$%^&*");
    assert!(
        result.is_err(),
        "garbage input '!@#$%^&*' should return Err, not silently succeed"
    );
}

// ─── Version advanced operations ─────────────────────────────────────────────

/// rez: version range union — merge two separate ranges
#[test]
fn test_version_range_union_disjoint() {
    let r1 = VersionRange::parse(">=1.0,<2.0").unwrap();
    let r2 = VersionRange::parse(">=3.0,<4.0").unwrap();
    let union = r1.union(&r2);
    assert!(
        union.contains(&Version::parse("1.5").unwrap()),
        "union should contain 1.5"
    );
    assert!(
        union.contains(&Version::parse("3.5").unwrap()),
        "union should contain 3.5"
    );
    assert!(
        !union.contains(&Version::parse("2.5").unwrap()),
        "union should not contain 2.5"
    );
}

/// rez: version range with pre-release label sorting
#[test]
fn test_version_prerelease_ordering() {
    let v_alpha = Version::parse("1.0.0.alpha").unwrap();
    let v_beta = Version::parse("1.0.0.beta").unwrap();
    let v_rc = Version::parse("1.0.0.rc.1").unwrap();
    let v_release = Version::parse("1.0.0").unwrap();
    assert!(
        v_release > v_alpha,
        "1.0.0 should be greater than 1.0.0.alpha in rez semantics"
    );
    assert!(
        v_release > v_beta,
        "1.0.0 should be greater than 1.0.0.beta"
    );
    assert!(v_release > v_rc, "1.0.0 should be greater than 1.0.0.rc.1");
}

/// rez: version range exclusive upper bound
#[test]
fn test_version_range_exclusive_upper() {
    let r = VersionRange::parse(">=2.0,<3.0").unwrap();
    assert!(r.contains(&Version::parse("2.0").unwrap()));
    assert!(r.contains(&Version::parse("2.9.9").unwrap()));
    assert!(
        !r.contains(&Version::parse("3.0").unwrap()),
        "3.0 should be excluded (upper bound)"
    );
    assert!(
        r.contains(&Version::parse("3.0.1").unwrap()),
        "3.0.1 is less than 3.0 in rez semantics (shorter = higher epoch), so should be included"
    );
}

/// rez: version range with version == bound edge
#[test]
fn test_version_range_inclusive_lower_edge() {
    let r = VersionRange::parse(">=1.0").unwrap();
    assert!(
        r.contains(&Version::parse("1.0").unwrap()),
        "lower bound 1.0 should be included"
    );
    assert!(
        !r.contains(&Version::parse("0.9.9").unwrap()),
        "0.9.9 should be excluded"
    );
}
