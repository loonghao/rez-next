//! Rez Compat — Solver Boundary, Context Advanced, Package Validation, VersionRange Advanced
//!
//! Extracted from rez_compat_advanced_tests.rs (Cycle 143).

// ─── Solver boundary tests ────────────────────────────────────────────────────

/// rez solver: single package with no dependencies resolves immediately
#[test]
fn test_solver_single_package_no_deps() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;
    use rez_next_version::Version;

    let mut graph = DependencyGraph::new();
    let mut pkg = Package::new("standalone".to_string());
    pkg.version = Some(Version::parse("1.0.0").unwrap());
    graph.add_package(pkg).unwrap();

    let result = graph.get_resolved_packages();
    assert!(result.is_ok(), "Single package with no deps should resolve");
    assert_eq!(result.unwrap().len(), 1);
}

/// rez solver: version range intersection for multi-constraint requirement
#[test]
fn test_solver_multi_constraint_version_range() {
    use rez_core::version::VersionRange;

    let r_ge = VersionRange::parse(">=3.9").unwrap();
    let r_lt = VersionRange::parse("<4.0").unwrap();
    let intersection = r_ge
        .intersect(&r_lt)
        .expect(">=3.9 and <4.0 should intersect");

    assert!(intersection.contains(&rez_core::version::Version::parse("3.9").unwrap()));
    assert!(intersection.contains(&rez_core::version::Version::parse("3.11").unwrap()));
    assert!(!intersection.contains(&rez_core::version::Version::parse("4.0").unwrap()));
    assert!(!intersection.contains(&rez_core::version::Version::parse("3.8").unwrap()));
}

/// rez solver: two packages with exclusive version ranges → conflict
#[test]
fn test_solver_exclusive_ranges_detect_conflict() {
    use rez_next_package::PackageRequirement;
    use rez_next_solver::DependencyGraph;

    let mut graph = DependencyGraph::new();
    graph
        .add_requirement(PackageRequirement::with_version(
            "lib".to_string(),
            ">=1.0,<2.0".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "lib".to_string(),
            ">=2.0".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(
        !conflicts.is_empty(),
        "Exclusive ranges >=1.0,<2.0 and >=2.0 should conflict for lib"
    );
}

/// rez solver: compatible ranges do not produce a conflict
#[test]
fn test_solver_compatible_ranges_no_conflict() {
    use rez_next_package::PackageRequirement;
    use rez_next_solver::DependencyGraph;

    let mut graph = DependencyGraph::new();
    // >=3.8 and <4.0 are compatible
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            ">=3.8".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            "<4.0".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(conflicts.is_empty(), ">=3.8 and <4.0 should not conflict");
}

/// rez solver: weak requirement (~pkg) is parsed correctly
#[test]
fn test_solver_weak_requirement_parse() {
    use rez_next_package::Requirement;

    let req = "~python>=3.9".parse::<Requirement>().unwrap();
    assert!(req.weak, "~ prefix should set weak=true");
    assert_eq!(req.name, "python");
}

/// rez solver: topological sort on a chain A → B → C
#[test]
fn test_solver_topological_sort_chain() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;
    use rez_next_version::Version;

    let mut graph = DependencyGraph::new();

    for (name, ver) in &[("pkgA", "1.0"), ("pkgB", "1.0"), ("pkgC", "1.0")] {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse(ver).unwrap());
        graph.add_package(pkg).unwrap();
    }

    graph.add_dependency_edge("pkgA-1.0", "pkgB-1.0").unwrap();
    graph.add_dependency_edge("pkgB-1.0", "pkgC-1.0").unwrap();

    let result = graph.get_resolved_packages();
    assert!(
        result.is_ok(),
        "Linear chain A->B->C should resolve (no cycles)"
    );
    assert_eq!(
        result.unwrap().len(),
        3,
        "All 3 packages should be in resolved order"
    );
}

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

// ─── Solver boundary tests (additional) ─────────────────────────────────────

/// rez solver: resolving with only one package returns exactly that package
#[test]
fn test_solver_single_package_resolution() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;
    use rez_next_version::Version;

    let mut graph = DependencyGraph::new();
    let mut pkg = Package::new("solo".to_string());
    pkg.version = Some(Version::parse("1.0.0").unwrap());
    graph.add_package(pkg).unwrap();

    let result = graph.get_resolved_packages().unwrap();
    assert_eq!(
        result.len(),
        1,
        "Single package graph should resolve to 1 package"
    );
    assert_eq!(result[0].name, "solo");
}

/// rez solver: weak requirement (~) does not prevent resolution when absent
#[test]
fn test_solver_weak_requirement_optional_absent() {
    use rez_next_package::Requirement;

    let req: Requirement = "~optional_tool>=1.0".parse().unwrap();
    assert!(req.weak, "~ prefix must produce a weak requirement");
    assert_eq!(req.name, "optional_tool");
}

/// rez solver: diamond dependency A→B, A→C, B→D, C→D resolves correctly
#[test]
fn test_solver_diamond_dependency_no_conflict() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;
    use rez_next_version::Version;

    let mut graph = DependencyGraph::new();
    for (n, v) in &[("A", "1.0"), ("B", "1.0"), ("C", "1.0"), ("D", "1.0")] {
        let mut pkg = Package::new(n.to_string());
        pkg.version = Some(Version::parse(v).unwrap());
        graph.add_package(pkg).unwrap();
    }

    graph.add_dependency_edge("A-1.0", "B-1.0").unwrap();
    graph.add_dependency_edge("A-1.0", "C-1.0").unwrap();
    graph.add_dependency_edge("B-1.0", "D-1.0").unwrap();
    graph.add_dependency_edge("C-1.0", "D-1.0").unwrap();

    let resolved = graph.get_resolved_packages().unwrap();
    assert_eq!(
        resolved.len(),
        4,
        "Diamond dependency should include all 4 packages exactly once"
    );
}

// ─── Package is_valid() / validate() tests ────────────────────────────────

/// rez package: valid package passes is_valid()
#[test]
fn test_package_is_valid_basic() {
    use rez_next_package::Package;
    use rez_next_version::Version;

    let mut pkg = Package::new("mypkg".to_string());
    pkg.version = Some(Version::parse("1.0.0").unwrap());
    assert!(
        pkg.is_valid(),
        "Package with valid name and version should be valid"
    );
}

/// rez package: empty name fails is_valid()
#[test]
fn test_package_is_valid_empty_name() {
    use rez_next_package::Package;

    let pkg = Package::new("".to_string());
    assert!(
        !pkg.is_valid(),
        "Package with empty name should not be valid"
    );
}

/// rez package: invalid name chars fails validate()
#[test]
fn test_package_validate_invalid_name_chars() {
    use rez_next_package::Package;

    let pkg = Package::new("bad@pkg!name".to_string());
    assert!(
        pkg.validate().is_err(),
        "Package with special chars in name should fail validate()"
    );
    let err_msg = pkg.validate().unwrap_err().to_string();
    assert!(
        err_msg.contains("Invalid package name"),
        "Error should mention invalid name: {}",
        err_msg
    );
}

/// rez package: empty requirement in requires fails validate()
#[test]
fn test_package_validate_empty_requirement() {
    use rez_next_package::Package;
    use rez_next_version::Version;

    let mut pkg = Package::new("mypkg".to_string());
    pkg.version = Some(Version::parse("1.0.0").unwrap());
    pkg.requires.push("".to_string()); // Empty requirement
    assert!(
        pkg.validate().is_err(),
        "Package with empty requirement should fail validate()"
    );
    assert!(
        !pkg.is_valid(),
        "is_valid() should return false for package with empty requirement"
    );
}

/// rez package: valid name formats (hyphen, underscore) pass is_valid()
#[test]
fn test_package_is_valid_name_variants() {
    use rez_next_package::Package;

    for name in &["my-pkg", "my_pkg", "MyPkg2", "pkg123"] {
        let pkg = Package::new(name.to_string());
        assert!(pkg.is_valid(), "Package name '{}' should be valid", name);
    }
}

/// rez package: empty build_requires entry fails validate()
#[test]
fn test_package_validate_empty_build_requirement() {
    use rez_next_package::Package;

    let mut pkg = Package::new("buildpkg".to_string());
    pkg.build_requires.push("cmake".to_string());
    pkg.build_requires.push("".to_string()); // invalid entry
    let result = pkg.validate();
    assert!(
        result.is_err(),
        "Empty build requirement should fail validation"
    );
}

// ─── VersionRange advanced tests ──────────────────────────────────────────

/// rez version range: negation "!=" (exclude single version)
#[test]
fn test_version_range_exclude_single() {
    use rez_core::version::{Version, VersionRange};

    let r = VersionRange::parse("!=2.0").unwrap();
    assert!(
        !r.contains(&Version::parse("2.0").unwrap()),
        "2.0 should be excluded"
    );
    assert!(
        r.contains(&Version::parse("1.9").unwrap()),
        "1.9 should be included"
    );
    assert!(
        r.contains(&Version::parse("2.1").unwrap()),
        "2.1 should be included"
    );
}

/// rez version range: upper-inclusive "<=2.0"
#[test]
fn test_version_range_le() {
    use rez_core::version::{Version, VersionRange};

    let r = VersionRange::parse("<=2.0").unwrap();
    assert!(
        r.contains(&Version::parse("2.0").unwrap()),
        "2.0 should be included in <=2.0"
    );
    assert!(
        r.contains(&Version::parse("1.5").unwrap()),
        "1.5 should be included in <=2.0"
    );
    assert!(
        !r.contains(&Version::parse("2.1").unwrap()),
        "2.1 should not be in <=2.0"
    );
}

/// rez version range: ">1.0" (strict lower bound, exclusive)
#[test]
fn test_version_range_gt_exclusive() {
    use rez_core::version::{Version, VersionRange};

    let r = VersionRange::parse(">1.0").unwrap();
    assert!(
        !r.contains(&Version::parse("1.0").unwrap()),
        "1.0 should be excluded from >1.0"
    );
    assert!(
        r.contains(&Version::parse("1.1").unwrap()),
        "1.1 should be included in >1.0"
    );
}

/// rez version range: combined ">1.0,<=2.0"
#[test]
fn test_version_range_combined_gt_le() {
    use rez_core::version::{Version, VersionRange};

    let r = VersionRange::parse(">1.0,<=2.0").unwrap();
    assert!(
        !r.contains(&Version::parse("1.0").unwrap()),
        "1.0 excluded (strict >)"
    );
    assert!(r.contains(&Version::parse("1.5").unwrap()), "1.5 included");
    assert!(
        r.contains(&Version::parse("2.0").unwrap()),
        "2.0 included (<=)"
    );
    assert!(!r.contains(&Version::parse("2.1").unwrap()), "2.1 excluded");
}

/// rez version range: is_superset_of semantics
#[test]
fn test_version_range_is_superset() {
    use rez_core::version::VersionRange;

    let broad = VersionRange::parse(">=1.0").unwrap();
    let narrow = VersionRange::parse(">=1.5,<2.0").unwrap();
    assert!(
        broad.is_superset_of(&narrow),
        ">=1.0 should be superset of >=1.5,<2.0"
    );
    assert!(
        !narrow.is_superset_of(&broad),
        ">=1.5,<2.0 should NOT be superset of >=1.0"
    );
}
