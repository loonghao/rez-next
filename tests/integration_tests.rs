//! Integration tests for rez-core
//!
//! Solve+context+env chain, requirement cross-validation, version/package interop
//! → extracted to integration_rex_solver_tests.rs (Cycle 144)

use rez_core::common::RezCoreConfig;
use rez_core::version::{Version, VersionRange};

#[test]
fn test_version_creation() {
    let version = Version::parse("1.2.3").expect("Should create version");
    assert_eq!(version.as_str(), "1.2.3");
}

#[test]
fn test_version_range_creation() {
    let range = VersionRange::parse("1.0.0..2.0.0").expect("Should create version range");
    assert_eq!(range.as_str(), "1.0.0..2.0.0");
}

#[test]
fn test_config_defaults() {
    let config = RezCoreConfig::default();
    assert!(config.use_rust_version);
    assert!(config.use_rust_solver);
    assert!(config.use_rust_repository);
    assert!(config.rust_fallback);
}

#[test]
fn test_module_structure() {
    // Test that all modules can be imported and basic functionality works
    let _version = Version::parse("1.0.0").expect("Version creation should work");
    let _range = VersionRange::parse("1.0.0+").expect("Range creation should work");
    let _config = RezCoreConfig::default();

    // This test ensures the basic module structure compiles and initializes
}

// Phase 73: Integration tests for new VersionRange APIs
mod version_range_integration {
    use rez_core::version::{Version, VersionRange};

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    fn vr(s: &str) -> VersionRange {
        VersionRange::parse(s).unwrap()
    }

    #[test]
    fn test_version_range_intersect_semantic() {
        let r1 = vr(">=1.0,<3.0");
        let r2 = vr(">=2.0,<4.0");
        let i = r1.intersect(&r2).unwrap();
        // intersection should be >=2.0,<3.0
        assert!(i.contains(&v("2.0")));
        assert!(i.contains(&v("2.9")));
        assert!(!i.contains(&v("1.0"))); // below intersection lower bound
        assert!(!i.contains(&v("3.0"))); // at r1's upper bound (excluded)
    }

    #[test]
    fn test_version_range_union_semantic() {
        let r1 = vr(">=1.0,<2.0");
        let r2 = vr(">=3.0,<4.0");
        let u = r1.union(&r2);
        assert!(u.contains(&v("1.5")));
        assert!(u.contains(&v("3.5")));
        assert!(!u.contains(&v("2.5"))); // gap between the two ranges
    }

    #[test]
    fn test_version_range_subset_chain() {
        // >=1.0,<1.5 ⊂ >=1.0,<2.0 ⊂ >=1.0
        let r1 = vr(">=1.0,<1.5");
        let r2 = vr(">=1.0,<2.0");
        let r3 = vr(">=1.0");

        assert!(r1.is_subset_of(&r2));
        assert!(r1.is_subset_of(&r3));
        assert!(r2.is_subset_of(&r3));

        assert!(!r2.is_subset_of(&r1));
        assert!(!r3.is_subset_of(&r2));
        assert!(!r3.is_subset_of(&r1));
    }

    #[test]
    fn test_version_range_superset_chain() {
        let r1 = vr(">=1.0,<1.5");
        let r2 = vr(">=1.0,<2.0");
        let r3 = vr(">=1.0");

        assert!(r3.is_superset_of(&r2));
        assert!(r3.is_superset_of(&r1));
        assert!(r2.is_superset_of(&r1));
    }

    #[test]
    fn test_version_range_intersects_disjoint() {
        // Two completely disjoint ranges
        let r1 = vr(">=1.0,<2.0");
        let r2 = vr(">=3.0,<4.0");
        assert!(!r1.intersects(&r2));

        // Adjacent ranges (boundary: [1,2) and [2,3))
        let r3 = vr(">=2.0,<3.0");
        // 2.0 is not in r1 (exclusive upper), so no overlap
        assert!(!r1.intersects(&r3));
    }

    #[test]
    fn test_version_range_intersects_overlapping() {
        let r1 = vr(">=1.0,<3.0");
        let r2 = vr(">=2.0,<4.0");
        assert!(r1.intersects(&r2));
    }

    #[test]
    fn test_version_range_subtract_basic() {
        let r1 = vr(">=1.0,<3.0");
        let r2 = vr(">=2.0,<3.0");
        let diff = r1.subtract(&r2);
        // diff should be >=1.0,<2.0
        assert!(diff.is_some());
        let diff = diff.unwrap();
        assert!(diff.contains(&v("1.5")));
        assert!(!diff.contains(&v("2.0")));
        assert!(!diff.contains(&v("2.5")));
    }

    #[test]
    fn test_version_range_exact_subset() {
        let exact = vr("==2.0.0");
        let range = vr(">=1.0,<3.0");
        assert!(exact.is_subset_of(&range));
        assert!(range.is_superset_of(&exact));
    }

    #[test]
    fn test_version_range_any_superset_of_all() {
        let any = vr("*");
        let r1 = vr(">=1.0,<2.0");
        let r2 = vr("==3.5");
        assert!(any.is_superset_of(&r1));
        assert!(any.is_superset_of(&r2));
        assert!(r1.is_subset_of(&any));
        assert!(r2.is_subset_of(&any));
    }

    #[test]
    fn test_version_range_rez_syntax_compatibility() {
        // Test rez-specific syntax
        let r1 = vr("1.0+"); // >=1.0
        let r2 = vr("1.0+<2.0"); // >=1.0,<2.0
        let r3 = vr("1.0..2.0"); // >=1.0,<2.0

        assert!(r2.is_subset_of(&r1));
        // r2 and r3 should be semantically equivalent
        assert!(r2.contains(&v("1.5")));
        assert!(r3.contains(&v("1.5")));
        assert!(!r2.contains(&v("2.0")));
        assert!(!r3.contains(&v("2.0")));
    }
}
