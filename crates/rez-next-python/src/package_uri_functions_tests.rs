//! Tests for package URI functions.
//!
//! These are light smoke tests — the real heavy lifting of URI resolution,
//! variant lookup, and repository interaction is tested end-to-end in the
//! integration test suite (`tests/` directory).

use rez_next_version::Version;

#[test]
fn test_parse_version_for_uri_resolution() {
    // Package URI resolution (get_package_from_uri, etc.) relies on
    // version parsing — verify the dependency chain is wired correctly.
    let v = Version::parse("3.9.5").expect("valid version");
    assert_eq!(v.to_string(), "3.9.5");
}

#[test]
fn test_version_compare_for_variant_matching() {
    // get_variant / get_variant_from_uri need version comparison.
    let v1 = Version::parse("2.0").unwrap();
    let v2 = Version::parse("2.0.0").unwrap();
    // Trailing zeros are implicit (rez semantics).
    assert_eq!(v1, v2);
}
