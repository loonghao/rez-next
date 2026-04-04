//! Tests for VersionRange — extracted from range.rs for file-size compliance.

#[cfg(test)]
mod tests {
    use crate::range::VersionRange;
    use crate::Version;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn test_version_range_any() {
        let range = VersionRange::parse("").unwrap();
        assert!(range.is_any());
        assert!(range.contains(&v("1.0.0")));
        assert!(range.contains(&v("99.0")));

        let range = VersionRange::parse("*").unwrap();
        assert!(range.is_any());
    }

    #[test]
    fn test_version_range_any_constructor() {
        let range = VersionRange::any();
        assert!(range.is_any(), "VersionRange::any() must report is_any()");
        assert!(range.contains(&v("0.0.1")));
        assert!(range.contains(&v("1.0.0")));
        assert!(range.contains(&v("99.99.99")));
        let specific = VersionRange::parse(">=1.0,<2.0").unwrap();
        let intersected = range
            .intersect(&specific)
            .expect("intersection with any must succeed");
        assert!(!intersected.contains(&v("0.9.0")));
        assert!(intersected.contains(&v("1.5.0")));
    }

    #[test]
    fn test_version_range_none_constructor() {
        let range = VersionRange::none();
        assert!(range.is_empty(), "VersionRange::none() must report is_empty()");
        assert!(!range.contains(&v("1.0.0")));
        assert!(!range.contains(&v("0.0.1")));
        assert!(!range.contains(&v("99.0")));
        let specific = VersionRange::parse(">=1.0").unwrap();
        assert!(range.intersect(&specific).is_none(), "none intersect anything must be None");
    }

    #[test]
    fn test_version_range_any_union_identity() {
        let any = VersionRange::any();
        let specific = VersionRange::parse("==1.0.0").unwrap();
        let unioned = any.union(&specific);
        assert!(unioned.contains(&v("1.0.0")));
        assert!(unioned.contains(&v("2.0.0")));
    }

    #[test]
    fn test_version_range_ge() {
        let range = VersionRange::parse(">=1.0.0").unwrap();
        assert!(range.contains(&v("1.0.0")));
        assert!(range.contains(&v("2.0.0")));
        assert!(!range.contains(&v("0.9.0")));
    }

    #[test]
    fn test_version_range_lt() {
        let range = VersionRange::parse("<2.0.0").unwrap();
        assert!(range.contains(&v("1.9.9")));
        assert!(!range.contains(&v("2.0.0")));
        assert!(!range.contains(&v("2.1.0")));
    }

    #[test]
    fn test_version_range_and() {
        let range = VersionRange::parse(">=1.0.0,<2.0.0").unwrap();
        assert!(range.contains(&v("1.0.0")));
        assert!(range.contains(&v("1.5.0")));
        assert!(!range.contains(&v("0.9.0")));
        assert!(!range.contains(&v("2.0.0")));
        assert!(!range.contains(&v("3.0.0")));
    }

    #[test]
    fn test_version_range_exact() {
        let range = VersionRange::parse("==1.0.0").unwrap();
        assert!(range.contains(&v("1.0.0")));
        assert!(!range.contains(&v("1.0.1")));
        assert!(!range.contains(&v("0.9.0")));
    }

    #[test]
    fn test_version_range_ne() {
        let range = VersionRange::parse("!=1.0.0").unwrap();
        assert!(!range.contains(&v("1.0.0")));
        assert!(range.contains(&v("1.0.1")));
        assert!(range.contains(&v("0.9.0")));
    }

    #[test]
    fn test_version_range_gt() {
        let range = VersionRange::parse(">1.0.0").unwrap();
        assert!(!range.contains(&v("1.0.0")));
        assert!(range.contains(&v("1.0.1")));
    }

    #[test]
    fn test_version_range_le() {
        let range = VersionRange::parse("<=2.0.0").unwrap();
        assert!(range.contains(&v("2.0.0")));
        assert!(range.contains(&v("1.0.0")));
        assert!(!range.contains(&v("2.0.1")));
    }

    #[test]
    fn test_version_range_or() {
        let range = VersionRange::parse("<1.0|>=2.0").unwrap();
        assert!(range.contains(&v("0.9.0")));
        assert!(range.contains(&v("2.0")));
        assert!(!range.contains(&v("1.5.0")));
    }

    #[test]
    fn test_version_range_compatible_release() {
        let range = VersionRange::parse("~=1.4.0").unwrap();
        assert!(range.contains(&v("1.4.0")));
        assert!(range.contains(&v("1.4.5")));
        assert!(!range.contains(&v("1.3.0")));
        assert!(!range.contains(&v("2.0.0")));
    }

    #[test]
    fn test_version_range_rez_plus_syntax() {
        let range = VersionRange::parse("1.0+").unwrap();
        assert!(range.contains(&v("1.0")));
        assert!(range.contains(&v("2.0")));
        assert!(!range.contains(&v("0.9")));
    }

    #[test]
    fn test_version_range_rez_plus_upper() {
        let range = VersionRange::parse("1.0+<2.0").unwrap();
        assert!(range.contains(&v("1.0")));
        assert!(range.contains(&v("1.5")));
        assert!(!range.contains(&v("2.0")));
        assert!(!range.contains(&v("0.9")));
    }

    #[test]
    fn test_version_range_intersect() {
        let r1 = VersionRange::parse(">=1.0.0").unwrap();
        let r2 = VersionRange::parse("<=2.0.0").unwrap();
        let intersection = r1.intersect(&r2).unwrap();
        assert!(intersection.contains(&v("1.5.0")));
    }

    #[test]
    fn test_version_range_union() {
        let r1 = VersionRange::parse(">=1.0.0,<1.5.0").unwrap();
        let r2 = VersionRange::parse(">=2.0.0").unwrap();
        let union = r1.union(&r2);
        assert!(union.contains(&v("1.2.0")));
        assert!(union.contains(&v("2.5.0")));
        assert!(!union.contains(&v("1.7.0")));
    }

    #[test]
    fn test_version_range_bare_version() {
        let range = VersionRange::parse("1.0.0").unwrap();
        assert!(range.contains(&v("1.0.0")));
        assert!(!range.contains(&v("1.0.1")));
    }

    #[test]
    fn test_is_empty() {
        let range = VersionRange::parse("empty").unwrap();
        assert!(range.is_empty());
        let range2 = VersionRange::parse("!*").unwrap();
        assert!(range2.is_empty());
    }

    #[test]
    fn test_space_separated_constraints() {
        let range = VersionRange::parse(">=1.0 <2.0").unwrap();
        assert!(range.contains(&v("1.5")));
        assert!(!range.contains(&v("2.0")));
        assert!(!range.contains(&v("0.9")));
    }

    #[test]
    fn test_is_subset_of_basic() {
        let r1 = VersionRange::parse(">=1.0,<2.0").unwrap();
        let r2 = VersionRange::parse(">=1.0").unwrap();
        assert!(r1.is_subset_of(&r2));
        assert!(!r2.is_subset_of(&r1));
    }

    #[test]
    fn test_is_superset_of_basic() {
        let r1 = VersionRange::parse(">=1.0").unwrap();
        let r2 = VersionRange::parse(">=1.0,<2.0").unwrap();
        assert!(r1.is_superset_of(&r2));
        assert!(!r2.is_superset_of(&r1));
    }

    #[test]
    fn test_is_subset_of_any() {
        let any = VersionRange::parse("*").unwrap();
        let r1 = VersionRange::parse(">=1.0").unwrap();
        assert!(r1.is_subset_of(&any));
        assert!(any.is_subset_of(&any));
    }

    #[test]
    fn test_is_subset_of_empty() {
        let empty = VersionRange::parse("empty").unwrap();
        let r1 = VersionRange::parse(">=1.0").unwrap();
        assert!(empty.is_subset_of(&r1));
        assert!(empty.is_subset_of(&empty));
    }

    #[test]
    fn test_subtract_basic() {
        let r1 = VersionRange::parse(">=1.0").unwrap();
        let r2 = VersionRange::parse(">=2.0").unwrap();
        let diff = r1.subtract(&r2);
        assert!(diff.is_some());
        let diff = diff.unwrap();
        assert!(diff.contains(&v("1.5")));
        assert!(!diff.contains(&v("2.5")));
    }

    #[test]
    fn test_subtract_any_gives_none() {
        let r1 = VersionRange::parse(">=1.0").unwrap();
        let any = VersionRange::parse("*").unwrap();
        assert!(r1.subtract(&any).is_none());
    }

    #[test]
    fn test_subtract_empty_gives_self() {
        let r1 = VersionRange::parse(">=1.0").unwrap();
        let empty = VersionRange::parse("empty").unwrap();
        let diff = r1.subtract(&empty);
        assert!(diff.is_some());
        let diff = diff.unwrap();
        assert!(diff.contains(&v("1.5")));
        assert!(diff.contains(&v("3.0")));
    }

    #[test]
    fn test_subset_exact_version() {
        let r1 = VersionRange::parse("==1.0").unwrap();
        let r2 = VersionRange::parse(">=1.0").unwrap();
        assert!(r1.is_subset_of(&r2));
        assert!(!r2.is_subset_of(&r1));
    }

    // ── intersect() returns new range ──────────────────────────────────────────

    #[test]
    fn test_intersect_overlapping_ranges() {
        let r1 = VersionRange::parse(">=1.0,<3.0").unwrap();
        let r2 = VersionRange::parse(">=2.0,<4.0").unwrap();
        let result = r1.intersect(&r2);
        assert!(result.is_some(), "Overlapping ranges should have intersection");
        let inter = result.unwrap();
        assert!(inter.contains(&v("2.0")), "2.0 should be in intersection");
        assert!(inter.contains(&v("2.9")), "2.9 should be in intersection");
        assert!(!inter.contains(&v("1.5")), "1.5 should NOT be in intersection");
        assert!(!inter.contains(&v("3.0")), "3.0 should NOT be in intersection");
    }

    #[test]
    fn test_intersect_with_any() {
        let any = VersionRange::parse("*").unwrap();
        let r1 = VersionRange::parse(">=2.0,<5.0").unwrap();
        let result = any.intersect(&r1);
        assert!(result.is_some());
        let inter = result.unwrap();
        assert!(inter.contains(&v("3.0")));
        assert!(!inter.contains(&v("1.0")));
        let result2 = r1.intersect(&any);
        assert!(result2.is_some());
        let inter2 = result2.unwrap();
        assert!(inter2.contains(&v("3.0")));
    }

    #[test]
    fn test_intersect_exact_in_range() {
        let exact = VersionRange::parse("==2.5").unwrap();
        let range = VersionRange::parse(">=2.0,<3.0").unwrap();
        let result = exact.intersect(&range);
        assert!(result.is_some());
        let inter = result.unwrap();
        assert!(inter.contains(&v("2.5")));
        assert!(!inter.contains(&v("2.4")));
        assert!(!inter.contains(&v("2.6")));
    }

    #[test]
    fn test_intersect_exact_outside_range() {
        let exact = VersionRange::parse("==5.0").unwrap();
        let range = VersionRange::parse(">=1.0,<3.0").unwrap();
        let result = exact.intersect(&range);
        if let Some(ref inter) = result {
            assert!(!inter.contains(&v("5.0")), "5.0 should NOT be in intersection");
            assert!(!inter.contains(&v("1.5")), "1.5 should NOT be in intersection");
        }
    }

    #[test]
    fn test_intersect_disjoint_ranges() {
        let r1 = VersionRange::parse(">=1.0,<2.0").unwrap();
        let r2 = VersionRange::parse(">=3.0,<4.0").unwrap();
        let result = r1.intersect(&r2);
        if let Some(ref inter) = result {
            assert!(!inter.contains(&v("1.5")));
            assert!(!inter.contains(&v("3.5")));
        }
    }

    #[test]
    fn test_intersect_same_range() {
        let r = VersionRange::parse(">=1.0,<3.0").unwrap();
        let result = r.intersect(&r);
        assert!(result.is_some());
        let inter = result.unwrap();
        assert!(inter.contains(&v("1.5")));
        assert!(!inter.contains(&v("0.9")));
        assert!(!inter.contains(&v("3.0")));
    }

    #[test]
    fn test_intersect_with_ne() {
        let r1 = VersionRange::parse(">=1.0,<3.0").unwrap();
        let r2 = VersionRange::parse("!=2.0").unwrap();
        let result = r1.intersect(&r2);
        assert!(result.is_some());
        let inter = result.unwrap();
        assert!(inter.contains(&v("1.5")));
        assert!(!inter.contains(&v("2.0")));
        assert!(inter.contains(&v("2.5")));
    }

    #[test]
    fn test_intersect_or_range() {
        let r1 = VersionRange::parse("<1.0|>=3.0").unwrap();
        let r2 = VersionRange::parse(">=2.0").unwrap();
        let result = r1.intersect(&r2);
        assert!(result.is_some());
        let inter = result.unwrap();
        assert!(!inter.contains(&v("0.5")), "0.5 not in (<1|>=3) ∩ >=2");
        assert!(!inter.contains(&v("1.5")), "1.5 not in result");
        assert!(inter.contains(&v("3.5")), "3.5 should be in (<1|>=3) ∩ >=2");
    }

    #[test]
    fn test_intersect_compatible_release() {
        let r1 = VersionRange::parse("~=1.2").unwrap();
        let r2 = VersionRange::parse("<1.5").unwrap();
        let result = r1.intersect(&r2);
        assert!(result.is_some());
        let inter = result.unwrap();
        assert!(inter.contains(&v("1.3")));
        assert!(!inter.contains(&v("1.5")));
        assert!(!inter.contains(&v("1.0")));
    }

    #[test]
    fn test_intersect_returns_some_for_adjacent() {
        let r1 = VersionRange::parse(">=1.0,<2.0").unwrap();
        let r2 = VersionRange::parse(">=2.0,<3.0").unwrap();
        let result = r1.intersect(&r2);
        if let Some(ref inter) = result {
            assert!(!inter.contains(&v("1.5")), "1.5 not in r2");
            assert!(!inter.contains(&v("2.5")), "2.5 not in r1");
            assert!(!inter.contains(&v("2.0")), "2.0 not in r1 (strict <)");
        }
    }
}
