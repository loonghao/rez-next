use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Solver conflict detection tests ───────────────────────────────────────

/// rez solver: two packages requiring incompatible python versions → conflict
#[test]
fn test_solver_conflict_incompatible_python_versions() {
    use rez_next_package::PackageRequirement;

    // tool_a requires python-3.9, tool_b requires python-3.11+<3.12
    let req_a = PackageRequirement::with_version("python".to_string(), "3.9".to_string());
    let req_b = PackageRequirement::with_version("python".to_string(), "3.11+<3.12".to_string());

    let v39 = Version::parse("3.9").unwrap();
    let v311 = Version::parse("3.11").unwrap();

    // python-3.9 satisfies req_a but NOT req_b
    assert!(req_a.satisfied_by(&v39), "3.9 satisfies python-3.9");
    assert!(
        !req_b.satisfied_by(&v39),
        "3.9 does NOT satisfy python-3.11+<3.12"
    );

    // python-3.11 satisfies req_b but NOT req_a (exact 3.9 required)
    assert!(
        !req_a.satisfied_by(&v311),
        "3.11 does NOT satisfy exact python-3.9"
    );
    assert!(
        req_b.satisfied_by(&v311),
        "3.11 satisfies python-3.11+<3.12"
    );

    // No single version satisfies both → confirmed conflict
    let candidates = ["3.9", "3.10", "3.11", "3.12"];
    let satisfies_both = candidates.iter().any(|v| {
        let ver = Version::parse(v).unwrap();
        req_a.satisfied_by(&ver) && req_b.satisfied_by(&ver)
    });
    assert!(
        !satisfies_both,
        "No python version should satisfy both constraints"
    );
}

/// rez solver: transitive dependency requires a compatible intermediate version
#[test]
fn test_solver_transitive_dependency_resolution() {
    use rez_next_package::PackageRequirement;

    // Scenario: app-1.0 → lib-2.0+ ; framework-3.0 → lib-2.5+<3.0
    // Compatible resolution: lib-2.5 or lib-2.9 satisfies both
    let req_app = PackageRequirement::with_version("lib".to_string(), "2.0+".to_string());
    let req_fw = PackageRequirement::with_version("lib".to_string(), "2.5+<3.0".to_string());

    let v25 = Version::parse("2.5").unwrap();
    let v29 = Version::parse("2.9").unwrap();
    let v30 = Version::parse("3.0").unwrap();
    let v19 = Version::parse("1.9").unwrap();

    assert!(
        req_app.satisfied_by(&v25),
        "lib-2.5 satisfies app req lib-2.0+"
    );
    assert!(
        req_fw.satisfied_by(&v25),
        "lib-2.5 satisfies fw req lib-2.5+<3.0"
    );

    assert!(req_app.satisfied_by(&v29), "lib-2.9 satisfies app req");
    assert!(req_fw.satisfied_by(&v29), "lib-2.9 satisfies fw req");

    assert!(
        !req_fw.satisfied_by(&v30),
        "lib-3.0 does NOT satisfy lib-2.5+<3.0 (exclusive upper)"
    );
    assert!(
        !req_app.satisfied_by(&v19),
        "lib-1.9 does NOT satisfy lib-2.0+"
    );
}

/// rez solver: diamond dependency — A→C-1+, B→C-1.5+ should resolve to C-1.5+
#[test]
fn test_solver_diamond_dependency_resolution() {
    use rez_next_package::PackageRequirement;

    let req_from_a = PackageRequirement::with_version("clib".to_string(), "1.0+".to_string());
    let req_from_b = PackageRequirement::with_version("clib".to_string(), "1.5+".to_string());

    // clib-1.5 satisfies both
    let v15 = Version::parse("1.5").unwrap();
    assert!(req_from_a.satisfied_by(&v15));
    assert!(req_from_b.satisfied_by(&v15));

    // clib-2.0 also satisfies both
    let v20 = Version::parse("2.0").unwrap();
    assert!(req_from_a.satisfied_by(&v20));
    assert!(req_from_b.satisfied_by(&v20));

    // clib-1.4 only satisfies req_from_a
    let v14 = Version::parse("1.4").unwrap();
    assert!(req_from_a.satisfied_by(&v14));
    assert!(
        !req_from_b.satisfied_by(&v14),
        "1.4 < 1.5 so doesn't satisfy 1.5+"
    );
}

/// rez solver: package requiring its own minimum version
#[test]
fn test_solver_self_version_constraint() {
    use rez_next_package::PackageRequirement;

    // A newer package v2 requires itself to be at least v1 (trivially satisfied)
    let self_req = PackageRequirement::with_version("mypkg".to_string(), "1.0+".to_string());
    let v2 = Version::parse("2.0").unwrap();
    assert!(self_req.satisfied_by(&v2), "v2 satisfies >=1.0 self-req");
}

/// rez solver: version range with '+' suffix (rez-specific open-ended range)
#[test]
fn test_solver_rez_plus_suffix_range() {
    // rez range "2.0+" means ">=2.0" (open-ended)
    let range = VersionRange::parse("2.0+").unwrap();
    assert!(
        range.contains(&Version::parse("2.0").unwrap()),
        "2.0+ includes 2.0"
    );
    assert!(
        range.contains(&Version::parse("3.0").unwrap()),
        "2.0+ includes 3.0"
    );
    assert!(
        range.contains(&Version::parse("100.0").unwrap()),
        "2.0+ is open-ended"
    );
    assert!(
        !range.contains(&Version::parse("1.9").unwrap()),
        "2.0+ excludes 1.9"
    );
}

/// rez solver: VersionRange intersection with no overlap → empty
#[test]
fn test_solver_version_range_no_intersection() {
    let r1 = VersionRange::parse(">=1.0,<2.0").unwrap();
    let r2 = VersionRange::parse(">=3.0").unwrap();
    let intersection = r1.intersect(&r2);
    // Either None or empty range
    match intersection {
        None => {} // expected: no intersection
        Some(ref r) => assert!(
            r.is_empty(),
            "Intersection of [1,2) and [3,∞) should be empty"
        ),
    }
}

/// rez solver: multiple constraints on same package coalesce correctly
#[test]
fn test_solver_multiple_constraints_coalesce() {
    // >=1.0 AND <3.0 → effectively 1.0..3.0
    let r1 = VersionRange::parse(">=1.0").unwrap();
    let r2 = VersionRange::parse("<3.0").unwrap();
    let combined = r1.intersect(&r2).expect("should have intersection");
    assert!(combined.contains(&Version::parse("1.0").unwrap()));
    assert!(combined.contains(&Version::parse("2.9").unwrap()));
    assert!(!combined.contains(&Version::parse("3.0").unwrap()));
    assert!(!combined.contains(&Version::parse("0.9").unwrap()));
}

