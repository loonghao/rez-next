//! Version and VersionRange unit tests

#[cfg(test)]
mod tests {
    use crate::version::Version;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn test_parse_simple() {
        let ver = v("1.0.0");
        assert_eq!(ver.as_str(), "1.0.0");
    }

    #[test]
    fn test_parse_single_digit() {
        let ver = v("3");
        assert_eq!(ver.as_str(), "3");
    }

    #[test]
    fn test_parse_two_components() {
        let ver = v("2.5");
        assert_eq!(ver.as_str(), "2.5");
    }

    #[test]
    fn test_parse_empty_is_epsilon() {
        let ver = v("");
        assert!(ver.is_empty());
    }

    #[test]
    fn test_comparison_equal() {
        assert_eq!(v("1.0.0"), v("1.0.0"));
    }

    #[test]
    fn test_comparison_less() {
        assert!(v("1.0.0") < v("2.0.0"));
        assert!(v("1.2.0") < v("1.3.0"));
        assert!(v("1.0.0") < v("1.0.1"));
    }

    #[test]
    fn test_comparison_greater() {
        assert!(v("2.0.0") > v("1.0.0"));
        assert!(v("1.10.0") > v("1.9.0"));
    }

    #[test]
    fn test_comparison_ge_le() {
        assert!(v("1.0.0") >= v("1.0.0"));
        assert!(v("2.0.0") >= v("1.0.0"));
        assert!(v("1.0.0") <= v("1.0.0"));
        assert!(v("1.0.0") <= v("2.0.0"));
    }

    #[test]
    fn test_rez_shorter_is_greater() {
        // In rez, "1.0" > "1.0.0" (epoch semantics: fewer tokens = higher precedence)
        assert!(v("1.0") > v("1.0.0"));
    }

    #[test]
    fn test_as_str_roundtrip() {
        for s in &["1.0.0", "2.5.3", "10.0.0", "1.2.3.4.5"] {
            let ver = v(s);
            assert_eq!(ver.as_str(), *s);
        }
    }

    #[test]
    fn test_clone_equals_original() {
        let ver = v("3.7.2");
        let cloned = ver.clone();
        assert_eq!(ver, cloned);
    }

    #[test]
    fn test_ne_different_versions() {
        assert_ne!(v("1.0.0"), v("2.0.0"));
    }
}

#[cfg(test)]
mod range_tests {
    use crate::range::VersionRange;
    use crate::version::Version;

    #[test]
    fn test_parse_any() {
        let range = VersionRange::parse("").unwrap();
        assert!(range.is_any());
    }

    #[test]
    fn test_parse_exact() {
        let range = VersionRange::parse("==1.0.0").unwrap();
        assert_eq!(range.as_str(), "==1.0.0");
    }

    #[test]
    fn test_parse_gte() {
        let range = VersionRange::parse(">=1.0.0").unwrap();
        assert_eq!(range.as_str(), ">=1.0.0");
    }

    #[test]
    fn test_any_contains_version() {
        let range = VersionRange::parse("").unwrap();
        let ver = Version::parse("1.0.0").unwrap();
        assert!(range.contains(&ver));
    }

    #[test]
    fn test_intersect_returns_some() {
        let r1 = VersionRange::parse(">=1.0.0").unwrap();
        let r2 = VersionRange::parse("<=2.0.0").unwrap();
        let intersection = r1.intersect(&r2);
        assert!(intersection.is_some());
    }

    #[test]
    fn test_non_any_range_as_str() {
        let range = VersionRange::parse(">=1.2.3").unwrap();
        assert!(!range.is_any());
        assert!(!range.as_str().is_empty());
    }
}

// ── Phase 75: OR / AND mixed expressions + subtract/subset/superset ──────────

#[cfg(test)]
mod range_advanced_tests {
    use crate::range::VersionRange;
    use crate::version::Version;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    fn r(s: &str) -> VersionRange {
        VersionRange::parse(s).unwrap()
    }

    // ── OR (|) disjunction ──────────────────────────────────────────────

    #[test]
    fn test_or_contains_first_alternative() {
        // "==1.0|==2.0" — both exact versions should be contained
        let range = r("==1.0|==2.0");
        assert!(range.contains(&v("1.0")), "1.0 must be in ==1.0|==2.0");
        assert!(range.contains(&v("2.0")), "2.0 must be in ==1.0|==2.0");
    }

    #[test]
    fn test_or_does_not_contain_middle() {
        // "==1.0|==3.0" — 2.0 should not be contained
        let range = r("==1.0|==3.0");
        assert!(!range.contains(&v("2.0")), "2.0 must NOT be in ==1.0|==3.0");
    }

    #[test]
    fn test_or_with_ranges() {
        // "<1.0|>=3.0" — 0.9, 3.0, 5.0 are in; 1.5, 2.9 are not
        let range = r("<1.0|>=3.0");
        assert!(range.contains(&v("0.9")));
        assert!(range.contains(&v("3.0")));
        assert!(range.contains(&v("5.0")));
        assert!(!range.contains(&v("1.5")));
        assert!(!range.contains(&v("2.9")));
    }

    // ── AND (comma / space separated) ──────────────────────────────────

    #[test]
    fn test_and_both_must_match() {
        // ">=1.0,<2.0" — 1.5 yes, 0.9 no, 2.0 no
        let range = r(">=1.0,<2.0");
        assert!(range.contains(&v("1.0")));
        assert!(range.contains(&v("1.5")));
        assert!(!range.contains(&v("0.9")));
        assert!(!range.contains(&v("2.0")));
    }

    #[test]
    fn test_and_with_ne_excludes_specific_version() {
        // ">=1.0,!=1.5,<2.0"
        let range = r(">=1.0,!=1.5,<2.0");
        assert!(range.contains(&v("1.0")));
        assert!(range.contains(&v("1.4")));
        assert!(!range.contains(&v("1.5")));
        assert!(range.contains(&v("1.6")));
        assert!(!range.contains(&v("2.0")));
    }

    // ── Mixed OR + AND ────────────────────────────────────────────────

    #[test]
    fn test_mixed_or_and_expressions() {
        // ">=1.0,<1.5|>=2.0,<3.0"
        // Bracket interpretation: (>=1.0 AND <1.5) OR (>=2.0 AND <3.0)
        let range = r(">=1.0,<1.5|>=2.0,<3.0");
        assert!(range.contains(&v("1.2"))); // first arm
        assert!(!range.contains(&v("1.7"))); // gap between arms
        assert!(range.contains(&v("2.5"))); // second arm
        assert!(!range.contains(&v("3.0"))); // beyond second arm
    }

    // ── is_any / is_empty edge cases ─────────────────────────────────

    #[test]
    fn test_wildcard_is_any() {
        assert!(r("*").is_any());
    }

    #[test]
    fn test_empty_string_is_any() {
        assert!(r("").is_any());
    }

    #[test]
    fn test_empty_marker_is_empty() {
        assert!(r("empty").is_empty());
    }

    // ── intersect ─────────────────────────────────────────────────────

    #[test]
    fn test_intersect_disjoint_returns_none_or_empty() {
        let a = r(">=3.0");
        let b = r("<1.0");
        let result = a.intersect(&b);
        // Either None or an empty range
        if let Some(intersection) = result {
            assert!(intersection.is_empty() || !intersection.contains(&v("2.0")));
        }
    }

    #[test]
    fn test_intersect_overlapping() {
        let a = r(">=1.0,<3.0");
        let b = r(">=2.0,<4.0");
        let result = a.intersect(&b);
        assert!(result.is_some());
        let isect = result.unwrap();
        assert!(isect.contains(&v("2.5")));
        assert!(!isect.contains(&v("1.0")));
        assert!(!isect.contains(&v("3.5")));
    }

    // ── intersects (boolean) ──────────────────────────────────────────

    #[test]
    fn test_intersects_true() {
        let a = r(">=1.0,<3.0");
        let b = r(">=2.0,<4.0");
        assert!(a.intersects(&b));
    }

    #[test]
    fn test_intersects_false_disjoint() {
        let a = r("<1.0");
        let b = r(">=2.0");
        assert!(!a.intersects(&b));
    }

    // ── is_subset_of / is_superset_of ─────────────────────────────────

    #[test]
    fn test_subset_of_wider_range() {
        let narrow = r(">=1.5,<2.0");
        let wide = r(">=1.0,<3.0");
        assert!(narrow.is_subset_of(&wide));
    }

    #[test]
    fn test_not_subset_if_extends_beyond() {
        let a = r(">=1.0,<4.0");
        let b = r(">=1.0,<3.0");
        assert!(!a.is_subset_of(&b));
    }

    #[test]
    fn test_superset_of_narrower_range() {
        let wide = r(">=1.0,<3.0");
        let narrow = r(">=1.5,<2.0");
        assert!(wide.is_superset_of(&narrow));
    }

    // ── subtract ──────────────────────────────────────────────────────

    #[test]
    fn test_subtract_leaves_remainder() {
        let a = r(">=1.0,<3.0");
        let b = r(">=2.0,<3.0"); // subtract upper half
        let result = a.subtract(&b);
        assert!(result.is_some());
        let remainder = result.unwrap();
        // Versions in [1.0, 2.0) should still be covered
        assert!(remainder.contains(&v("1.0")));
        assert!(remainder.contains(&v("1.5")));
        // Versions in [2.0, 3.0) should NOT be covered
        assert!(!remainder.contains(&v("2.0")));
        assert!(!remainder.contains(&v("2.5")));
    }

    #[test]
    fn test_subtract_same_range_is_empty_or_none() {
        let a = r(">=1.0,<2.0");
        let b = r(">=1.0,<2.0");
        let result = a.subtract(&b);
        match result {
            None => {} // None means empty
            Some(r) => assert!(r.is_empty() || !r.contains(&v("1.5"))),
        }
    }

    // ── Compatible release ~= ─────────────────────────────────────────

    #[test]
    fn test_compatible_release_contains_patch() {
        // ~=1.2 means >=1.2 AND <2.0 (compatible release)
        let range = r("~=1.2");
        assert!(range.contains(&v("1.2")));
        assert!(range.contains(&v("1.9")));
    }

    #[test]
    fn test_compatible_release_excludes_next_major() {
        let range = r("~=1.2");
        // 2.0 should be excluded
        assert!(!range.contains(&v("2.0")));
    }
}

// ── Phase 84: dot-dot syntax equivalence + Version helpers ───────────────────

#[cfg(test)]
mod range_dotdot_tests {
    use crate::range::VersionRange;
    use crate::version::Version;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    fn r(s: &str) -> VersionRange {
        VersionRange::parse(s).unwrap()
    }

    /// "1.0..2.0" should contain 1.0, 1.5, but NOT 2.0 (half-open [1.0, 2.0))
    #[test]
    fn test_dotdot_contains_lower() {
        let range = r("1.0..2.0");
        assert!(range.contains(&v("1.0")), "1.0 should be in 1.0..2.0");
        assert!(range.contains(&v("1.5")), "1.5 should be in 1.0..2.0");
    }

    #[test]
    fn test_dotdot_excludes_upper() {
        let range = r("1.0..2.0");
        assert!(!range.contains(&v("2.0")), "2.0 should NOT be in 1.0..2.0");
        assert!(!range.contains(&v("2.5")), "2.5 should NOT be in 1.0..2.0");
    }

    /// "1.0+<2.0" should be equivalent to "1.0..2.0"
    #[test]
    fn test_plus_lt_contains_lower() {
        let range = r("1.0+<2.0");
        assert!(range.contains(&v("1.0")), "1.0 should be in 1.0+<2.0");
        assert!(range.contains(&v("1.9")), "1.9 should be in 1.0+<2.0");
    }

    #[test]
    fn test_plus_lt_excludes_upper() {
        let range = r("1.0+<2.0");
        assert!(!range.contains(&v("2.0")), "2.0 should NOT be in 1.0+<2.0");
    }

    /// "1.0+<2.0" and "1.0..2.0" should contain the same sample versions
    #[test]
    fn test_dotdot_plus_lt_equivalence() {
        let dotdot = r("1.0..2.0");
        let plus_lt = r("1.0+<2.0");
        for ver_str in &["1.0", "1.0.1", "1.5", "1.9", "1.9.9", "2.0", "2.1", "0.9"] {
            let ver = v(ver_str);
            assert_eq!(
                dotdot.contains(&ver),
                plus_lt.contains(&ver),
                "dotdot and plus_lt should agree on {}: dotdot={}, plus_lt={}",
                ver_str,
                dotdot.contains(&ver),
                plus_lt.contains(&ver)
            );
        }
    }

    /// "1.0+" (rez shorthand for >=1.0)
    #[test]
    fn test_plus_shorthand_is_ge() {
        let range = r("1.0+");
        assert!(range.contains(&v("1.0")), "1.0 should be in 1.0+");
        assert!(range.contains(&v("2.0")), "2.0 should be in 1.0+");
        assert!(range.contains(&v("9.9")), "9.9 should be in 1.0+");
        assert!(!range.contains(&v("0.9")), "0.9 should NOT be in 1.0+");
    }

    /// Nested AND in dotdot: "1.0..2.0,!=1.5" excludes 1.5
    #[test]
    fn test_dotdot_and_ne_excludes() {
        let range = r("1.0..2.0,!=1.5");
        assert!(range.contains(&v("1.0")));
        assert!(
            !range.contains(&v("1.5")),
            "1.5 should be excluded by !=1.5"
        );
        assert!(range.contains(&v("1.6")));
    }

    /// Empty range `empty` has no versions
    #[test]
    fn test_empty_range_contains_nothing() {
        let range = r("empty");
        assert!(!range.contains(&v("0.0")));
        assert!(!range.contains(&v("1.0")));
        assert!(!range.contains(&v("999.0")));
    }

    /// "any" (*) range contains everything
    #[test]
    fn test_any_range_contains_everything() {
        let range = r("*");
        assert!(range.contains(&v("0.0")));
        assert!(range.contains(&v("1.0")));
        assert!(range.contains(&v("999.999")));
    }

    /// Two disjoint ranges intersect to empty/None
    #[test]
    fn test_disjoint_ranges_no_intersection() {
        let a = r("1.0..2.0");
        let b = r("3.0..4.0");
        let result = a.intersect(&b);
        match result {
            None => {}
            Some(i) => assert!(i.is_empty() || !i.contains(&v("2.5"))),
        }
    }

    /// Overlapping dotdot ranges intersect correctly
    #[test]
    fn test_overlapping_dotdot_intersect() {
        let a = r("1.0..3.0");
        let b = r("2.0..4.0");
        let result = a.intersect(&b);
        assert!(result.is_some());
        let isect = result.unwrap();
        assert!(
            isect.contains(&v("2.5")),
            "2.5 should be in intersection of [1,3) and [2,4)"
        );
        assert!(
            !isect.contains(&v("1.0")),
            "1.0 should NOT be in intersection"
        );
        assert!(
            !isect.contains(&v("3.5")),
            "3.5 should NOT be in intersection"
        );
    }
}

// ── Phase 84: Version helper methods ────────────────────────────────────────

#[cfg(test)]
mod version_helper_tests {
    use crate::version::Version;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn test_major_component() {
        let ver = v("3.7.2");
        assert_eq!(ver.major(), Some(3));
    }

    #[test]
    fn test_minor_component() {
        let ver = v("3.7.2");
        assert_eq!(ver.minor(), Some(7));
    }

    #[test]
    fn test_patch_component() {
        let ver = v("3.7.2");
        assert_eq!(ver.patch(), Some(2));
    }

    #[test]
    fn test_major_single_digit() {
        let ver = v("5");
        assert_eq!(ver.major(), Some(5));
        assert_eq!(ver.minor(), None);
    }

    #[test]
    fn test_is_empty_empty_version() {
        let ver = v("");
        assert!(ver.is_empty());
        assert_eq!(ver.major(), None);
    }

    #[test]
    fn test_version_len() {
        assert_eq!(v("1.2.3").len(), 3);
        assert_eq!(v("1.2").len(), 2);
        assert_eq!(v("1").len(), 1);
        assert_eq!(v("").len(), 0);
    }

    // ── Phase 110: additional version tests ─────────────────────────────────

    /// Version equality and ordering operators
    #[test]
    fn test_version_ordering_operators() {
        let v1 = v("1.0.0");
        let v2 = v("2.0.0");
        let v3 = v("1.0.0");
        assert!(v1 < v2, "1.0.0 < 2.0.0");
        assert!(v2 > v1, "2.0.0 > 1.0.0");
        assert_eq!(v1, v3, "1.0.0 == 1.0.0");
        assert!(v1 <= v3, "1.0.0 <= 1.0.0");
        assert!(v2 >= v1, "2.0.0 >= 1.0.0");
    }

    /// Version hash: equal versions have equal hashes
    #[test]
    fn test_version_hash_equality() {
        use std::collections::HashSet;
        let v1 = v("1.2.3");
        let v2 = v("1.2.3");
        let mut set = HashSet::new();
        set.insert(v1);
        // The same version string should not produce a duplicate
        assert!(set.contains(&v2), "Same version should be found in HashSet");
    }

    /// Version as_str roundtrip
    #[test]
    fn test_version_as_str_roundtrip() {
        for s in &["1.0.0", "2.3.4", "0.1", "10", "1.2.3.4.5"] {
            let ver = v(s);
            assert_eq!(
                ver.as_str(),
                *s,
                "as_str should return original string for {}",
                s
            );
        }
    }

    /// Version with alphanumeric token (e.g., "1.2.alpha")
    #[test]
    fn test_version_alphanumeric_token() {
        let ver = v("1.2.alpha");
        assert_eq!(ver.as_str(), "1.2.alpha");
        assert_eq!(ver.major(), Some(1));
        assert_eq!(ver.minor(), Some(2));
    }

    /// Pre-release ordering: alphanumeric tokens have different ordering from pure numeric
    #[test]
    fn test_version_prerelease_less_than_release() {
        let pre = v("1.2.alpha");
        let rel = v("1.2.0");
        // They should be different (not equal)
        assert_ne!(pre, rel, "1.2.alpha and 1.2.0 should be different versions");
        // rez spec: alpha token < numeric token → 1.2.alpha < 1.2.0
        assert!(
            pre < rel,
            "rez ordering: 1.2.alpha should be less than 1.2.0 (alpha < numeric)"
        );
    }

    /// Version patch returns None for two-component versions
    #[test]
    fn test_version_patch_optional() {
        let ver = v("3.7");
        assert_eq!(ver.major(), Some(3));
        assert_eq!(ver.minor(), Some(7));
        assert_eq!(ver.patch(), None);
    }

    /// len() matches number of components
    #[test]
    fn test_version_len_four_components() {
        let ver = v("1.2.3.4");
        assert_eq!(ver.len(), 4);
        assert_eq!(ver.major(), Some(1));
        assert_eq!(ver.minor(), Some(2));
        assert_eq!(ver.patch(), Some(3));
    }

    /// Version serialization via serde_json
    #[test]
    fn test_version_serde_json_roundtrip() {
        let original = v("2.0.1");
        let json = serde_json::to_string(&original).unwrap();
        // Should serialize as the string "2.0.1"
        assert!(
            json.contains("2.0.1"),
            "Serialized JSON should contain version string"
        );
        let restored: Version = serde_json::from_str(&json).unwrap();
        assert_eq!(
            original, restored,
            "Deserialized version should equal original"
        );
    }

    /// Version ordering: many versions sorted
    #[test]
    fn test_version_sort_many() {
        let mut versions: Vec<Version> = vec![v("3.0"), v("1.0"), v("2.5"), v("2.0"), v("1.5")];
        versions.sort();
        let sorted_strs: Vec<&str> = versions.iter().map(|v| v.as_str()).collect();
        assert_eq!(sorted_strs, vec!["1.0", "1.5", "2.0", "2.5", "3.0"]);
    }
}
