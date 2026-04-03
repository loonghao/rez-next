use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── New rez compat tests (Phase 2) ─────────────────────────────────────────

/// rez: weak requirement with version constraint parses correctly
#[test]
fn test_rez_weak_requirement_with_version() {
    let req = "~python>=3.9".parse::<Requirement>().unwrap();
    assert!(req.weak, "~python>=3.9 should be a weak requirement");
    assert_eq!(req.name, "python");
    assert!(
        req.version_constraint.is_some(),
        "should have version constraint"
    );
    assert!(
        req.is_satisfied_by(&Version::parse("3.11").unwrap()),
        "weak requirement still enforces version when present"
    );
}

/// rez: weak requirement without version parses correctly
#[test]
fn test_rez_weak_requirement_no_version() {
    let req = "~python".parse::<Requirement>().unwrap();
    assert!(req.weak);
    assert_eq!(req.name, "python");
    assert!(req.version_constraint.is_none());
    // Weak requirement with no constraint matches any version
    assert!(req.is_satisfied_by(&Version::parse("2.7").unwrap()));
    assert!(req.is_satisfied_by(&Version::parse("3.11.0").unwrap()));
}

/// rez: namespace-scoped requirement parsing
#[test]
fn test_rez_namespace_requirement() {
    let req = "studio::python>=3.9".parse::<Requirement>().unwrap();
    assert_eq!(req.name, "python");
    assert_eq!(req.namespace, Some("studio".to_string()));
    assert_eq!(req.qualified_name(), "studio::python");
    assert!(req.is_satisfied_by(&Version::parse("3.11.0").unwrap()));
    assert!(!req.is_satisfied_by(&Version::parse("3.8.0").unwrap()));
}

/// rez: platform condition on requirement
#[test]
fn test_rez_platform_condition_requirement() {
    let mut req = Requirement::new("my_lib".to_string());
    req.add_platform_condition("linux".to_string(), None, false);

    assert!(
        req.is_platform_satisfied("linux", None),
        "linux platform should match"
    );
    assert!(
        !req.is_platform_satisfied("windows", None),
        "windows should not match"
    );

    // Negated condition
    let mut req2 = Requirement::new("my_lib".to_string());
    req2.add_platform_condition("windows".to_string(), None, true);
    assert!(
        req2.is_platform_satisfied("linux", None),
        "linux should match (windows negated)"
    );
    assert!(
        !req2.is_platform_satisfied("windows", None),
        "windows should fail (negated)"
    );
}

/// rez: version range Exclude constraint
#[test]
fn test_rez_version_exclude_constraint() {
    use rez_next_package::requirement::VersionConstraint;

    let exclude_v1 = VersionConstraint::Exclude(vec![
        Version::parse("1.0.0").unwrap(),
        Version::parse("1.1.0").unwrap(),
    ]);

    assert!(
        exclude_v1.is_satisfied_by(&Version::parse("1.2.0").unwrap()),
        "1.2.0 not in exclude list, should satisfy"
    );
    assert!(
        !exclude_v1.is_satisfied_by(&Version::parse("1.0.0").unwrap()),
        "1.0.0 in exclude list, should not satisfy"
    );
    assert!(
        !exclude_v1.is_satisfied_by(&Version::parse("1.1.0").unwrap()),
        "1.1.0 in exclude list, should not satisfy"
    );
    assert!(
        exclude_v1.is_satisfied_by(&Version::parse("2.0.0").unwrap()),
        "2.0.0 not in exclude list, should satisfy"
    );
}

/// rez: Multiple (AND) constraint combination
#[test]
fn test_rez_multiple_constraint_and_logic() {
    use rez_next_package::requirement::VersionConstraint;

    let ge_3_9 = VersionConstraint::GreaterThanOrEqual(Version::parse("3.9").unwrap());
    let lt_4 = VersionConstraint::LessThan(Version::parse("4").unwrap());
    let combined = ge_3_9.and(lt_4);

    assert!(combined.is_satisfied_by(&Version::parse("3.9").unwrap()));
    assert!(combined.is_satisfied_by(&Version::parse("3.11.0").unwrap()));
    assert!(
        !combined.is_satisfied_by(&Version::parse("3.8").unwrap()),
        "3.8 should not satisfy >=3.9"
    );
    assert!(
        !combined.is_satisfied_by(&Version::parse("4.0.0").unwrap()),
        "4.0.0 should not satisfy <4"
    );
}

/// rez: Alternative (OR) constraint
#[test]
fn test_rez_alternative_constraint_or_logic() {
    use rez_next_package::requirement::VersionConstraint;

    // Either python 2.7 or python >= 3.9
    let eq_2_7 = VersionConstraint::Exact(Version::parse("2.7").unwrap());
    let ge_3_9 = VersionConstraint::GreaterThanOrEqual(Version::parse("3.9").unwrap());
    let or_constraint = eq_2_7.or(ge_3_9);

    assert!(
        or_constraint.is_satisfied_by(&Version::parse("2.7").unwrap()),
        "2.7 satisfies exact match OR"
    );
    assert!(
        or_constraint.is_satisfied_by(&Version::parse("3.11").unwrap()),
        "3.11 satisfies >=3.9 branch"
    );
    assert!(
        !or_constraint.is_satisfied_by(&Version::parse("3.0").unwrap()),
        "3.0 satisfies neither branch"
    );
    assert!(
        !or_constraint.is_satisfied_by(&Version::parse("2.6").unwrap()),
        "2.6 satisfies neither branch"
    );
}

/// rez: package.yaml with complex requirements and variants
#[test]
fn test_package_yaml_complex_fields() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name: houdini_plugin
version: "3.0.0"
description: "A Houdini plugin"
authors:
  - "SideFX Labs"
requires:
  - "houdini-20+"
  - "python-3.10+"
tools:
  - hplugin
  - hplugin_batch
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.yaml");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "houdini_plugin");
    assert!(pkg.version.is_some());
    assert!(
        !pkg.requires.is_empty(),
        "requires should be parsed from YAML"
    );
}

/// rez: package YAML roundtrip with all common fields
#[test]
fn test_package_yaml_roundtrip_full_fields() {
    use rez_next_package::serialization::PackageSerializer;

    let mut pkg = Package::new("roundtrip_pkg".to_string());
    pkg.version = Some(Version::parse("2.5.0").unwrap());
    pkg.description = Some("Full field roundtrip test".to_string());
    pkg.authors = vec!["Author One".to_string(), "Author Two".to_string()];
    pkg.requires = vec!["python-3.9+".to_string(), "numpy-1.20+".to_string()];
    pkg.tools = vec!["my_tool".to_string(), "my_helper".to_string()];

    let yaml = PackageSerializer::save_to_yaml(&pkg).unwrap();
    assert!(!yaml.is_empty(), "YAML output should not be empty");
    assert!(
        yaml.contains("roundtrip_pkg"),
        "YAML should contain package name"
    );
    assert!(yaml.contains("2.5.0"), "YAML should contain version");

    let loaded = PackageSerializer::load_from_yaml(&yaml).unwrap();
    assert_eq!(loaded.name, "roundtrip_pkg");
    assert_eq!(
        loaded.version.as_ref().map(|v| v.as_str()),
        Some("2.5.0"),
        "Version should roundtrip correctly"
    );
}

/// rez: Requirement display roundtrip (to_string -> parse consistency)
#[test]
fn test_requirement_display_roundtrip() {
    let cases = ["python", "python>=3.9", "python>=3.9,<4.0", "~python>=3.9"];

    for case in &cases {
        let req = case
            .parse::<Requirement>()
            .unwrap_or_else(|e| panic!("Failed to parse '{}': {}", case, e));
        let display = req.to_string();
        // Re-parse the display representation
        let reparsed = display.parse::<Requirement>().unwrap_or_else(|e| {
            panic!(
                "Failed to re-parse display '{}' (original: '{}'): {}",
                display, case, e
            )
        });
        assert_eq!(
            req.name, reparsed.name,
            "Name should be stable in roundtrip for '{}'",
            case
        );
        assert_eq!(
            req.weak, reparsed.weak,
            "Weak flag should be stable in roundtrip for '{}'",
            case
        );
    }
}

/// rez: solver handles diamond dependency pattern correctly
/// A -> B and C; B -> D-1.0; C -> D-2.0 (conflict)
#[test]
fn test_solver_diamond_dependency_conflict_detection() {
    use rez_next_package::PackageRequirement;
    use rez_next_solver::DependencyGraph;

    let mut graph = DependencyGraph::new();

    // Package A requires B and C
    // Package B requires D>=1.0,<2.0
    // Package C requires D>=2.0
    // These D requirements are disjoint → conflict
    graph
        .add_requirement(PackageRequirement::with_version(
            "D".to_string(),
            ">=1.0".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "D".to_string(),
            "<2.0".to_string(),
        ))
        .unwrap();
    // No conflict yet (>=1.0 AND <2.0 are compatible)
    assert!(
        graph.detect_conflicts().is_empty(),
        ">=1.0 and <2.0 are compatible for D"
    );

    // Now add disjoint constraint
    let mut conflict_graph = DependencyGraph::new();
    conflict_graph
        .add_requirement(PackageRequirement::with_version(
            "D".to_string(),
            ">=1.0,<2.0".to_string(),
        ))
        .unwrap();
    conflict_graph
        .add_requirement(PackageRequirement::with_version(
            "D".to_string(),
            ">=2.0".to_string(),
        ))
        .unwrap();
    let conflicts = conflict_graph.detect_conflicts();
    assert!(
        !conflicts.is_empty(),
        "D requiring >=1.0,<2.0 AND >=2.0 simultaneously should conflict"
    );
}

/// rez: version range operations compose correctly (intersection chains)
#[test]
fn test_version_range_chained_intersections() {
    // Start with "any" and progressively narrow down
    let any = VersionRange::parse("").unwrap();
    assert!(any.is_any());

    let r1 = VersionRange::parse(">=1.0").unwrap();
    let r2 = VersionRange::parse("<5.0").unwrap();
    let r3 = VersionRange::parse(">=2.0").unwrap();

    // any ∩ r1 = r1
    let step1 = any.intersect(&r1);
    assert!(step1.is_some(), "any ∩ r1 should be Some");

    // r1 ∩ r2 = [1.0, 5.0)
    let step2 = r1.intersect(&r2);
    assert!(step2.is_some());
    let s2 = step2.unwrap();
    assert!(s2.contains(&Version::parse("3.0").unwrap()));
    assert!(!s2.contains(&Version::parse("5.0").unwrap()));

    // [1.0, 5.0) ∩ r3 = [2.0, 5.0)
    let step3 = s2.intersect(&r3);
    assert!(step3.is_some());
    let s3 = step3.unwrap();
    assert!(
        !s3.contains(&Version::parse("1.5").unwrap()),
        "After intersecting with >=2.0, 1.5 should be excluded"
    );
    assert!(s3.contains(&Version::parse("2.0").unwrap()));
    assert!(s3.contains(&Version::parse("4.5").unwrap()));
}

