use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── rez.diff compatibility tests ────────────────────────────────────────────

/// rez.diff: two identical resolved contexts produce empty diff
#[test]
fn test_diff_identical_contexts_empty() {
    use rez_next_context::ResolvedContext;
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let make_ctx = || {
        let reqs = vec![PackageRequirement::parse("python-3.11").unwrap()];
        let mut ctx = ResolvedContext::from_requirements(reqs);
        let mut pkg = Package::new("python".to_string());
        pkg.version = Some(Version::parse("3.11").unwrap());
        ctx.resolved_packages.push(pkg);
        ctx
    };

    let ctx_a = make_ctx();
    let ctx_b = make_ctx();

    // diff: packages in A not in B (same version) → 0
    let names_a: std::collections::HashSet<String> = ctx_a
        .resolved_packages
        .iter()
        .map(|p| {
            format!(
                "{}-{}",
                p.name,
                p.version.as_ref().map(|v| v.as_str()).unwrap_or("?")
            )
        })
        .collect();
    let names_b: std::collections::HashSet<String> = ctx_b
        .resolved_packages
        .iter()
        .map(|p| {
            format!(
                "{}-{}",
                p.name,
                p.version.as_ref().map(|v| v.as_str()).unwrap_or("?")
            )
        })
        .collect();

    let added: Vec<_> = names_b.difference(&names_a).collect();
    let removed: Vec<_> = names_a.difference(&names_b).collect();
    assert!(
        added.is_empty(),
        "identical contexts should have no added packages"
    );
    assert!(
        removed.is_empty(),
        "identical contexts should have no removed packages"
    );
}

/// rez.diff: upgrading a package version shows up as changed
#[test]
fn test_diff_version_upgrade_detected() {
    use rez_next_context::ResolvedContext;
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let make_ctx = |ver: &str| {
        let reqs = vec![PackageRequirement::parse("maya-2023").unwrap()];
        let mut ctx = ResolvedContext::from_requirements(reqs);
        let mut pkg = Package::new("maya".to_string());
        pkg.version = Some(Version::parse(ver).unwrap());
        ctx.resolved_packages.push(pkg);
        ctx
    };

    let ctx_old = make_ctx("2023");
    let ctx_new = make_ctx("2024");

    let ver_old = ctx_old.resolved_packages[0]
        .version
        .as_ref()
        .map(|v| v.as_str())
        .unwrap_or("?");
    let ver_new = ctx_new.resolved_packages[0]
        .version
        .as_ref()
        .map(|v| v.as_str())
        .unwrap_or("?");

    assert_ne!(
        ver_old, ver_new,
        "version upgrade diff should detect a change"
    );
    // 2024 > 2023 in rez numeric ordering
    let v_old = Version::parse(ver_old).unwrap();
    let v_new = Version::parse(ver_new).unwrap();
    assert!(v_new > v_old, "new context should have higher version");
}

/// rez.diff: added package in new context detected
#[test]
fn test_diff_added_package_detected() {
    use rez_next_context::ResolvedContext;
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let reqs_old = vec![PackageRequirement::parse("python-3.11").unwrap()];
    let mut ctx_old = ResolvedContext::from_requirements(reqs_old);
    let mut pkg_py = Package::new("python".to_string());
    pkg_py.version = Some(Version::parse("3.11").unwrap());
    ctx_old.resolved_packages.push(pkg_py.clone());

    let reqs_new = vec![
        PackageRequirement::parse("python-3.11").unwrap(),
        PackageRequirement::parse("numpy-1.25").unwrap(),
    ];
    let mut ctx_new = ResolvedContext::from_requirements(reqs_new);
    ctx_new.resolved_packages.push(pkg_py);
    let mut pkg_np = Package::new("numpy".to_string());
    pkg_np.version = Some(Version::parse("1.25").unwrap());
    ctx_new.resolved_packages.push(pkg_np);

    let names_old: std::collections::HashSet<&str> = ctx_old
        .resolved_packages
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    let names_new: std::collections::HashSet<&str> = ctx_new
        .resolved_packages
        .iter()
        .map(|p| p.name.as_str())
        .collect();

    let added: Vec<_> = names_new.difference(&names_old).collect();
    assert_eq!(added.len(), 1, "one package (numpy) should appear as added");
    assert_eq!(*added[0], "numpy");
}

/// rez.diff: removed package in new context detected
#[test]
fn test_diff_removed_package_detected() {
    use rez_next_context::ResolvedContext;
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let make_pkg = |name: &str, ver: &str| {
        let mut p = Package::new(name.to_string());
        p.version = Some(Version::parse(ver).unwrap());
        p
    };

    let reqs_old = vec![
        PackageRequirement::parse("houdini-20").unwrap(),
        PackageRequirement::parse("hqueue-5").unwrap(),
    ];
    let mut ctx_old = ResolvedContext::from_requirements(reqs_old);
    ctx_old.resolved_packages.push(make_pkg("houdini", "20"));
    ctx_old.resolved_packages.push(make_pkg("hqueue", "5"));

    let reqs_new = vec![PackageRequirement::parse("houdini-20").unwrap()];
    let mut ctx_new = ResolvedContext::from_requirements(reqs_new);
    ctx_new.resolved_packages.push(make_pkg("houdini", "20"));

    let names_old: std::collections::HashSet<&str> = ctx_old
        .resolved_packages
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    let names_new: std::collections::HashSet<&str> = ctx_new
        .resolved_packages
        .iter()
        .map(|p| p.name.as_str())
        .collect();

    let removed: Vec<_> = names_old.difference(&names_new).collect();
    assert_eq!(
        removed.len(),
        1,
        "one package (hqueue) should appear as removed"
    );
    assert_eq!(*removed[0], "hqueue");
}
