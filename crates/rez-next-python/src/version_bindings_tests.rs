use super::*;
use rez_next_version::{Version, VersionRange};

fn pv(s: &str) -> PyVersion {
    PyVersion(Version::parse(s).unwrap())
}

fn pvr(s: &str) -> PyVersionRange {
    PyVersionRange(VersionRange::parse(s).unwrap())
}

#[test]
fn test_py_version_str() {
    let v = pv("1.2.3");
    assert_eq!(v.__str__(), "1.2.3");
    assert_eq!(v.__repr__(), "Version('1.2.3')");
}

#[test]
fn test_py_version_cmp() {
    let v1 = pv("1.0.0");
    let v2 = pv("2.0.0");
    assert!(v1.__lt__(&v2));
    assert!(v2.__gt__(&v1));
    assert!(v1.__le__(&v1));
    assert!(v1.__ge__(&v1));
    assert!(v1.__eq__(&v1));
    assert!(v1.__ne__(&v2));
}

#[test]
fn test_py_version_major_minor_patch() {
    let v = pv("1.2.3");
    assert_eq!(v.major(), Some("1".to_string()));
    assert_eq!(v.minor(), Some("2".to_string()));
    assert_eq!(v.patch(), Some("3".to_string()));
}

#[test]
fn test_py_version_next() {
    let v = pv("1.2.3");
    let next = v.next().unwrap();
    assert_eq!(next.__str__(), "1.2.4");
}

#[test]
fn test_py_version_trim() {
    let v = pv("1.2.3");
    let trimmed = v.trim(2).unwrap();
    assert_eq!(trimmed.__str__(), "1.2");
}

#[test]
fn test_py_version_range_str() {
    let r = pvr(">=1.0,<2.0");
    assert_eq!(r.__str__(), ">=1.0,<2.0");
    assert!(r.__repr__().contains(">=1.0,<2.0"));
}

#[test]
fn test_py_version_range_contains() {
    let r = pvr(">=1.0,<2.0");
    assert!(r.contains(&pv("1.5")));
    assert!(!r.contains(&pv("2.0")));
    assert!(!r.contains(&pv("0.9")));
}

#[test]
fn test_py_version_range_is_any() {
    let r = pvr("*");
    assert!(r.is_any());
    let r2 = pvr(">=1.0");
    assert!(!r2.is_any());
}

#[test]
fn test_py_version_range_is_empty() {
    let r = pvr("empty");
    assert!(r.is_empty());
}

#[test]
fn test_py_version_range_intersect() {
    let r1 = pvr(">=1.0");
    let r2 = pvr("<=2.0");
    let i = r1.intersect(&r2).unwrap();
    assert!(i.contains(&pv("1.5")));
}

#[test]
fn test_py_version_range_intersects() {
    let r1 = pvr(">=1.0,<2.0");
    let r2 = pvr(">=1.5,<3.0");
    assert!(r1.intersects(&r2));

    let r3 = pvr("<1.0");
    assert!(!r1.intersects(&r3));
}

#[test]
fn test_py_version_range_is_subset_of() {
    let r1 = pvr(">=1.0,<2.0");
    let r2 = pvr(">=1.0");
    assert!(r1.is_subset_of(&r2));
    assert!(!r2.is_subset_of(&r1));
}

#[test]
fn test_py_version_range_is_superset_of() {
    let r1 = pvr(">=1.0");
    let r2 = pvr(">=1.0,<2.0");
    assert!(r1.is_superset_of(&r2));
    assert!(!r2.is_superset_of(&r1));
}

#[test]
fn test_py_version_range_subtract() {
    let r1 = pvr(">=1.0");
    let r2 = pvr(">=2.0");
    let diff = r1.subtract(&r2).unwrap();
    assert!(diff.contains(&pv("1.5")));
    assert!(!diff.contains(&pv("2.5")));
}

#[test]
fn test_py_version_range_union() {
    let r1 = pvr(">=1.0,<1.5");
    let r2 = pvr(">=2.0");
    // union() requires PyResult - test via internal method
    let u = r1.0.union(&r2.0);
    assert!(u.contains(&rez_next_version::Version::parse("1.2").unwrap()));
    assert!(u.contains(&rez_next_version::Version::parse("2.5").unwrap()));
    assert!(!u.contains(&rez_next_version::Version::parse("1.7").unwrap()));
}

#[test]
fn test_py_version_hash_stability() {
    let v1 = pv("1.2.3");
    let v2 = pv("1.2.3");
    assert_eq!(v1.__hash__(), v2.__hash__());
    let v3 = pv("2.0.0");
    assert_ne!(v1.__hash__(), v3.__hash__());
}

#[test]
fn test_py_version_range_hash_stability() {
    let r1 = pvr(">=1.0");
    let r2 = pvr(">=1.0");
    assert_eq!(r1.__hash__(), r2.__hash__());
}

#[test]
fn test_py_version_range_any_classmethod() {
    // any() should be equivalent to VersionRange::any() — matches every version
    let r = PyVersionRange(VersionRange::any());
    assert!(r.is_any());
    assert!(r.contains(&pv("0.0.1")));
    assert!(r.contains(&pv("999.999.999")));
}

#[test]
fn test_py_version_range_none_classmethod() {
    // none() should match no version
    let r = PyVersionRange(VersionRange::none());
    assert!(r.is_empty());
    assert!(!r.contains(&pv("1.0.0")));
}

#[test]
fn test_py_version_range_from_str_static() {
    let r = PyVersionRange::from_str(">=1.0,<2.0").unwrap();
    assert!(r.contains(&pv("1.5")));
    assert!(!r.contains(&pv("2.0")));
}

#[test]
fn test_py_version_range_from_str_invalid() {
    // invalid range string must return Err, not panic
    let result = PyVersionRange::from_str("!!!invalid!!!");
    assert!(result.is_err());
}

#[test]
fn test_py_version_range_as_str() {
    let r = pvr(">=1.0,<2.0");
    assert_eq!(r.as_str(), ">=1.0,<2.0");
}

#[test]
fn test_py_version_range_any_union_identity() {
    // any() union with anything = any()
    let any = PyVersionRange(VersionRange::any());
    let r = pvr(">=3.0");
    // any().union(r) should give back any
    let union_result = any.union(&r).unwrap();
    assert!(union_result.is_any());
}

// ─── Additional version tests ─────────────────────────────────────────────

#[test]
fn test_py_version_empty_string_parses() {
    // empty version should parse without panic
    let result = PyVersion::new(Some(""));
    // empty may be Ok or Err depending on parser; must not panic
    let _ = result;
}

#[test]
fn test_py_version_single_token() {
    let v = pv("5");
    assert_eq!(v.__str__(), "5");
    assert_eq!(v.major(), Some("5".to_string()));
    assert!(
        v.minor().is_none() || v.minor() == Some(String::new()),
        "single-token version has no minor"
    );
}

#[test]
fn test_py_version_cmp_three_way() {
    let v1 = pv("1.0");
    let v2 = pv("1.0");
    let v3 = pv("1.1");
    assert!(v1.__eq__(&v2));
    assert!(v1.__lt__(&v3));
    assert!(v3.__gt__(&v1));
    assert!(!v1.__lt__(&v2));
}

#[test]
fn test_py_version_hash_different_versions_likely_differ() {
    let v1 = pv("1.0.0");
    let v2 = pv("9.9.9");
    // Very unlikely to collide
    assert_ne!(
        v1.__hash__(),
        v2.__hash__(),
        "different versions should have different hash"
    );
}

#[test]
fn test_py_version_trim_to_one() {
    let v = pv("3.11.4");
    let t = v.trim(1).unwrap();
    assert_eq!(t.__str__(), "3");
}

#[test]
fn test_py_version_range_contains_boundary() {
    // ">=1.0,<2.0" — boundary: 2.0 is excluded, 1.0 is included
    let r = pvr(">=1.0,<2.0");
    assert!(r.contains(&pv("1.0")), "lower bound must be inclusive");
    assert!(!r.contains(&pv("2.0")), "upper bound must be exclusive");
}

#[test]
fn test_py_version_range_intersect_non_overlapping_is_none() {
    let r1 = pvr(">=1.0,<2.0");
    let r2 = pvr(">=3.0,<4.0");
    let result = r1.intersect(&r2);
    // Non-overlapping ranges should produce None or empty range
    if let Some(intersection) = result {
        assert!(
            intersection.is_empty(),
            "non-overlapping intersection must be empty"
        );
    }
    // None is also acceptable
}

// ── Cycle 112 additions ───────────────────────────────────────────────────

#[test]
fn test_py_version_next_increments_last_numeric() {
    let v = pv("2.0.9");
    let n = v.next().unwrap();
    assert_eq!(
        n.__str__(),
        "2.0.10",
        "next() should increment last numeric component"
    );
}

#[test]
fn test_py_version_trim_zero_returns_empty_or_no_panic() {
    let v = pv("1.2.3");
    // trim(0) — should not panic; empty string version may be Ok or Err
    let result = v.trim(0);
    let _ = result; // just ensure no panic
}

#[test]
fn test_py_version_range_eq_same_range() {
    let r1 = pvr(">=1.0,<2.0");
    let r2 = pvr(">=1.0,<2.0");
    assert!(r1.__eq__(&r2), "identical ranges must be equal");
}

#[test]
fn test_py_version_range_union_same_range_is_idempotent() {
    let r = pvr(">=1.0,<2.0");
    let r2 = pvr(">=1.0,<2.0");
    let u = r.union(&r2).unwrap();
    // Union of same range with itself should be equivalent to original
    assert_eq!(
        u.as_str(),
        r.as_str(),
        "union of range with itself must be idempotent"
    );
}

#[test]
fn test_py_version_range_as_str_star_is_any() {
    let r = pvr("*");
    // '*' parses as 'any'; as_str may return "" or "*"
    let s = r.as_str();
    assert!(
        s.is_empty() || s == "*",
        "as_str for 'any' range should be '' or '*', got: {s}"
    );
}

#[test]
fn test_py_version_range_none_is_empty() {
    let r = PyVersionRange(VersionRange::none());
    assert!(r.is_empty(), "none range must be empty");
    assert!(!r.is_any(), "none range must not be any");
}

#[test]
fn test_py_version_repr_format() {
    let v = pv("3.11.0");
    assert_eq!(
        v.__repr__(),
        "Version('3.11.0')",
        "repr format must match rez convention"
    );
}

// ── Cycle 117 additions ───────────────────────────────────────────────────

#[test]
fn test_py_version_ge_same_version() {
    let v1 = pv("2.5.0");
    let v2 = pv("2.5.0");
    assert!(v1.__ge__(&v2), "__ge__ must return true for equal versions");
}

#[test]
fn test_py_version_le_same_version() {
    let v1 = pv("1.0.0");
    let v2 = pv("1.0.0");
    assert!(v1.__le__(&v2), "__le__ must return true for equal versions");
}

#[test]
fn test_py_version_ne_same_is_false() {
    let v1 = pv("3.0.0");
    let v2 = pv("3.0.0");
    assert!(!v1.__ne__(&v2), "__ne__ must return false for equal versions");
}

#[test]
fn test_py_version_range_ne_different_ranges() {
    let r1 = pvr(">=1.0,<2.0");
    let r2 = pvr(">=3.0,<4.0");
    assert!(!r1.__eq__(&r2), "different ranges must not be equal");
}

#[test]
fn test_py_version_range_repr_contains_range_str() {
    let r = pvr(">=2.0,<3.0");
    let repr = r.__repr__();
    assert!(
        repr.contains(">=2.0") || repr.contains("2.0"),
        "repr must contain the range string: {repr}"
    );
}

#[test]
fn test_py_version_range_contains_exact_boundary_version() {
    // ">=1.5" — exactly 1.5 must be included
    let r = pvr(">=1.5");
    assert!(r.contains(&pv("1.5")), "exact lower boundary must be contained");
    assert!(
        r.contains(&pv("1.6")),
        "version above lower boundary must be contained"
    );
    assert!(
        !r.contains(&pv("1.4")),
        "version below lower boundary must not be contained"
    );
}

#[test]
fn test_py_version_range_subtract_disjoint_returns_original() {
    // Subtracting a non-overlapping range from another yields the original (or None-wrapped)
    let r1 = pvr(">=1.0,<2.0");
    let r2 = pvr(">=5.0,<6.0");
    let result = r1.subtract(&r2);
    if let Some(diff) = result {
        assert!(
            diff.contains(&pv("1.5")),
            "subtracting non-overlapping range should not remove 1.5"
        );
    }
    // None is also valid if the implementation returns None for identical result
}

// ── Cycle 124 additions ───────────────────────────────────────────────────

#[test]
fn test_py_version_major_token_single_digit() {
    let v = pv("3.11.0");
    assert_eq!(v.major(), Some("3".to_string()), "major of '3.11.0' must be '3'");
}

#[test]
fn test_py_version_minor_token_two_digits() {
    let v = pv("3.11.0");
    assert_eq!(
        v.minor(),
        Some("11".to_string()),
        "minor of '3.11.0' must be '11'"
    );
}

#[test]
fn test_py_version_patch_token_present() {
    let v = pv("1.2.3");
    assert_eq!(v.patch(), Some("3".to_string()), "patch of '1.2.3' must be '3'");
}

#[test]
fn test_py_version_is_empty_true_for_empty_string() {
    let v = pv("");
    assert!(v.is_empty(), "empty version string must report is_empty()=true");
}

#[test]
fn test_py_version_is_empty_false_for_non_empty() {
    let v = pv("1.0");
    assert!(!v.is_empty(), "non-empty version must report is_empty()=false");
}

#[test]
fn test_py_version_trim_to_two_tokens() {
    let v = pv("1.2.3.4");
    let trimmed = v.trim(2).unwrap();
    assert_eq!(trimmed.__str__(), "1.2", "trim(2) of '1.2.3.4' must yield '1.2'");
}
