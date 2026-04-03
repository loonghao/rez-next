use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Context compat tests ────────────────────────────────────────────────────

/// rez.resolved_context: context created from zero requirements has empty resolved_packages
#[test]
fn test_context_empty_requirements_has_no_packages() {
    use rez_next_context::ResolvedContext;

    let ctx = ResolvedContext::from_requirements(vec![]);
    assert!(
        ctx.resolved_packages.is_empty(),
        "Empty requirements should produce empty resolved_packages"
    );
}

/// rez.resolved_context: get_summary reports correct package count
#[test]
fn test_context_summary_reflects_resolved_packages() {
    use rez_next_context::{ContextStatus, ResolvedContext};
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let reqs = vec![
        PackageRequirement::parse("python-3.11").unwrap(),
        PackageRequirement::parse("houdini-20.0").unwrap(),
    ];
    let mut ctx = ResolvedContext::from_requirements(reqs);

    for (name, ver) in &[("python", "3.11"), ("houdini", "20.0")] {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse(ver).unwrap());
        ctx.resolved_packages.push(pkg);
    }
    ctx.status = ContextStatus::Resolved;

    let summary = ctx.get_summary();
    assert_eq!(summary.package_count, 2);
    assert!(summary.package_versions.contains_key("python"));
    assert!(summary.package_versions.contains_key("houdini"));
}

/// rez.resolved_context: each context receives a unique ID
#[test]
fn test_context_unique_ids() {
    use rez_next_context::ResolvedContext;

    let c1 = ResolvedContext::from_requirements(vec![]);
    let c2 = ResolvedContext::from_requirements(vec![]);
    assert_ne!(c1.id, c2.id, "Each ResolvedContext must have a unique ID");
}

/// rez.resolved_context: created_at timestamp is positive (Unix epoch)
#[test]
fn test_context_created_at_positive() {
    use rez_next_context::ResolvedContext;

    let ctx = ResolvedContext::from_requirements(vec![]);
    assert!(
        ctx.created_at > 0,
        "created_at should be a positive Unix timestamp"
    );
}

/// rez.resolved_context: status transitions Failed → Resolved
#[test]
fn test_context_status_transition() {
    use rez_next_context::{ContextStatus, ResolvedContext};

    let mut ctx = ResolvedContext::from_requirements(vec![]);
    ctx.status = ContextStatus::Failed;
    assert_eq!(ctx.status, ContextStatus::Failed);

    ctx.status = ContextStatus::Resolved;
    assert_eq!(ctx.status, ContextStatus::Resolved);
}

/// rez.resolved_context: environment_vars can be injected (rez env semantics)
#[test]
fn test_context_environment_vars_injection() {
    use rez_next_context::ResolvedContext;

    let mut ctx = ResolvedContext::from_requirements(vec![]);
    ctx.environment_vars
        .insert("REZ_USED_REQUEST".to_string(), "python-3.11".to_string());
    ctx.environment_vars
        .insert("PATH".to_string(), "/usr/bin:/bin".to_string());

    assert_eq!(
        ctx.environment_vars.get("REZ_USED_REQUEST"),
        Some(&"python-3.11".to_string())
    );
    assert!(ctx.environment_vars.contains_key("PATH"));
}

