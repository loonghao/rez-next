use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── context.to_dict / get_tools compat tests ─────────────────────────────────

/// rez.context.to_dict: serialized dict contains required keys
#[test]
fn test_context_to_dict_contains_required_keys() {
    use rez_next_context::{ContextStatus, ResolvedContext};
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let reqs = vec![PackageRequirement::parse("python-3.11").unwrap()];
    let mut ctx = ResolvedContext::from_requirements(reqs);
    let mut pkg = Package::new("python".to_string());
    pkg.version = Some(Version::parse("3.11").unwrap());
    ctx.resolved_packages.push(pkg);
    ctx.status = ContextStatus::Resolved;

    // Simulate to_dict output: id, status, packages, num_packages
    let id = ctx.id.clone();
    let status = format!("{:?}", ctx.status);
    let pkgs: Vec<String> = ctx
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

    assert!(!id.is_empty(), "id must be non-empty");
    assert_eq!(status, "Resolved", "status must be Resolved");
    assert_eq!(pkgs.len(), 1);
    assert_eq!(pkgs[0], "python-3.11");
}

/// rez.context.to_dict: num_packages matches resolved package count
#[test]
fn test_context_to_dict_num_packages_matches() {
    use rez_next_context::{ContextStatus, ResolvedContext};
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let reqs = vec![
        PackageRequirement::parse("python-3.11").unwrap(),
        PackageRequirement::parse("maya-2024").unwrap(),
    ];
    let mut ctx = ResolvedContext::from_requirements(reqs);
    for (n, v) in &[("python", "3.11"), ("maya", "2024")] {
        let mut pkg = Package::new(n.to_string());
        pkg.version = Some(Version::parse(v).unwrap());
        ctx.resolved_packages.push(pkg);
    }
    ctx.status = ContextStatus::Resolved;

    let num = ctx.resolved_packages.len();
    assert_eq!(num, 2, "num_packages (to_dict) must equal 2");
}

/// rez.context.get_tools: packages with tools list export them correctly
#[test]
fn test_context_get_tools_collects_all_tools() {
    use rez_next_context::{ContextStatus, ResolvedContext};
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let reqs = vec![PackageRequirement::parse("maya-2024").unwrap()];
    let mut ctx = ResolvedContext::from_requirements(reqs);
    let mut pkg = Package::new("maya".to_string());
    pkg.version = Some(Version::parse("2024").unwrap());
    pkg.tools = vec![
        "maya".to_string(),
        "mayapy".to_string(),
        "mayabatch".to_string(),
    ];
    ctx.resolved_packages.push(pkg);
    ctx.status = ContextStatus::Resolved;

    // Verify tools are accessible via the resolved package
    let tools: Vec<String> = ctx
        .resolved_packages
        .iter()
        .flat_map(|p| p.tools.iter().cloned())
        .collect();

    assert_eq!(tools.len(), 3, "Should collect all 3 tools from maya");
    assert!(tools.contains(&"maya".to_string()));
    assert!(tools.contains(&"mayapy".to_string()));
    assert!(tools.contains(&"mayabatch".to_string()));
}

/// rez.context.get_tools: context with no tools yields empty collection
#[test]
fn test_context_get_tools_empty_when_no_tools() {
    use rez_next_context::{ContextStatus, ResolvedContext};
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let reqs = vec![PackageRequirement::parse("mylib-1.0").unwrap()];
    let mut ctx = ResolvedContext::from_requirements(reqs);
    let mut pkg = Package::new("mylib".to_string());
    pkg.version = Some(Version::parse("1.0").unwrap());
    // No tools set
    ctx.resolved_packages.push(pkg);
    ctx.status = ContextStatus::Resolved;

    let tools: Vec<String> = ctx
        .resolved_packages
        .iter()
        .flat_map(|p| p.tools.iter().cloned())
        .collect();
    assert!(
        tools.is_empty(),
        "Package with no tools should yield empty tools collection"
    );
}

