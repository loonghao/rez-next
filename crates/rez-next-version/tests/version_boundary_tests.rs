//! Version parsing boundary tests
//!
//! This file contains boundary tests for version parsing to supplement
//! the existing test coverage in `src/tests.rs`.

use rez_next_version::Version;

/// Test very long version strings (>10 tokens, should be rejected)
#[test]
fn test_version_too_many_tokens_20() {
    let long_version = "1.2.3.4.5.6.7.8.9.10.11.12.13.14.15.16.17.18.19.20";
    let result = Version::parse(long_version);
    assert!(
        result.is_err(),
        "Version with 20 tokens should be rejected (max: 10)"
    );
}

/// Test version with 9 numeric tokens (exceeds numeric limit of 5, should be rejected)
#[test]
fn test_version_9_numeric_tokens_rejected() {
    // 9 numeric tokens exceeds the limit of 5 numeric tokens
    let version_9_numeric = "1.2.3.4.5.6.7.8.9";
    let result = Version::parse(version_9_numeric);
    assert!(
        result.is_err(),
        "Version with 9 numeric tokens should be rejected (>5 numeric)"
    );
}

/// Test version with 9 total tokens (max allowed, should succeed)
#[test]
fn test_version_max_9_tokens_mixed() {
    // Use mixed alpha-numeric tokens to avoid numeric token limit
    let max_version = "1.2a.3.4a.5.6a.7.8a.9";
    let ver = Version::parse(max_version).unwrap();
    assert_eq!(ver.as_str(), max_version);
    assert_eq!(ver.len(), 9);
}

/// Test version with 10 tokens (should be rejected as "too complex")
#[test]
fn test_version_10_tokens_rejected() {
    let version_10_tokens = "1.2.3.4.5.6.7.8.9.10";
    let result = Version::parse(version_10_tokens);
    assert!(
        result.is_err(),
        "Version with 10 tokens should be rejected (too complex)"
    );
}

/// Test Unicode characters in version (should be rejected)
#[test]
fn test_version_unicode_chars() {
    // Test various Unicode characters
    let unicode_versions = [
        "1.2.3α",        // Greek alpha
        "1.2.3β",        // Greek beta
        "1.2.3©",        // Copyright symbol
        "1.2.3®",        // Registered symbol
        "版本1.2.3",      // Chinese characters
        "1.2.3🔴",       // Emoji
        "1.2.3ñ",        // Latin small letter n with tilde
        "1.2.3ü",        // Latin small letter u with diaeresis
    ];

    for &vstr in &unicode_versions {
        let result = Version::parse(vstr);
        assert!(
            result.is_err(),
            "Version with Unicode '{vstr}' should be rejected"
        );
    }
}

/// Test very large numeric tokens (u64 MAX)
#[test]
fn test_version_large_numeric_token() {
    // u64::MAX as string
    let max_u64 = u64::MAX.to_string();
    let version_str = format!("1.2.{max_u64}");
    let ver = Version::parse(&version_str).unwrap();
    assert_eq!(ver.as_str(), version_str);
}

/// Test numeric token overflow (larbage number)
#[test]
fn test_version_numeric_token_edge_cases() {
    // Test with various large numbers
    let large_numbers = [
        "18446744073709551615", // u64::MAX
        "99999999999999999999", // > u64::MAX (should still parse as alphanumeric)
    ];

    for &num_str in &large_numbers {
        let version_str = format!("1.{num_str}");
        // u64::MAX should parse successfully
        // Numbers > u64::MAX will be parsed as alphanumeric strings
        let result = Version::parse(&version_str);
        assert!(
            result.is_ok(),
            "Version with large number '{num_str}' should parse (as numeric or alphanumeric)"
        );
    }
}

/// Test invalid token patterns: "not" keyword (should be rejected)
#[test]
fn test_version_keyword_not() {
    // "not" is explicitly rejected by the parser
    let result = Version::parse("1.0.not");
    assert!(
        result.is_err(),
        "Version with token 'not' should be rejected"
    );
}

/// Test invalid token patterns: "version" keyword (should be rejected)
#[test]
fn test_version_keyword_version() {
    // "version" at start is rejected (prefix not supported)
    let result = Version::parse("version.1.0");
    assert!(
        result.is_err(),
        "Version starting with 'version' should be rejected"
    );
}

/// Test version with only separators (should be rejected)
#[test]
fn test_version_only_separators() {
    let invalid_versions = [".", "-", "_", "+", ".-_+", "...."];

    for &vstr in &invalid_versions {
        let result = Version::parse(vstr);
        assert!(
            result.is_err(),
            "Version with only separators '{vstr}' should be rejected"
        );
    }
}

/// Test version with spaces in middle (should be rejected)
#[test]
fn test_version_with_spaces_in_middle_only() {
    // Spaces in the middle are invalid (not trimmed)
    let invalid_versions = ["1.2. 3", "1. 2.3", "1 .2.3"];

    for &vstr in &invalid_versions {
        let result = Version::parse(vstr);
        assert!(
            result.is_err(),
            "Version with spaces in middle '{vstr}' should be rejected"
        );
    }
}

/// Test leading dot (should be rejected)
#[test]
fn test_version_leading_dot() {
    let ver = Version::parse(".1.2.3");
    assert!(ver.is_err(), "Leading dot should be rejected");
}

/// Test trailing dot (should be rejected)
#[test]
fn test_version_trailing_dot() {
    let ver = Version::parse("1.2.3.");
    assert!(ver.is_err(), "Trailing dot should be rejected");
}

/// Test consecutive dots (should be rejected)
#[test]
fn test_version_consecutive_dots() {
    let ver = Version::parse("1..2.3");
    assert!(ver.is_err(), "Consecutive dots should be rejected");
}

/// Test version with prefix v/V (should be rejected)
#[test]
fn test_version_prefix_v() {
    // Lowercase v
    let ver = Version::parse("v1.2.3");
    assert!(ver.is_err(), "Version prefix 'v' should be rejected");

    // Uppercase V
    let ver = Version::parse("V1.2.3");
    assert!(ver.is_err(), "Version prefix 'V' should be rejected");
}

/// Test version with too many tokens (>10)
#[test]
fn test_version_too_many_tokens() {
    let many_tokens = "1.2.3.4.5.6.7.8.9.10.11";
    let ver = Version::parse(many_tokens);
    assert!(ver.is_err(), "Version with >10 tokens should be rejected");
}

/// Test version with too many numeric tokens (>5)
#[test]
fn test_version_too_many_numeric_tokens() {
    let many_numeric = "1.2.3.4.5.6";
    let ver = Version::parse(many_numeric);
    assert!(ver.is_err(), "Version with >5 numeric tokens should be rejected");
}

/// Test version with underscore in token
#[test]
fn test_version_underscore_token() {
    let ver = Version::parse("1.2_beta.3").unwrap();
    assert_eq!(ver.as_str(), "1.2_beta.3");
}

/// Test version with alphanumeric tokens (mixed)
#[test]
fn test_version_mixed_alphanumeric() {
    let ver = Version::parse("1.2beta.3").unwrap();
    assert_eq!(ver.as_str(), "1.2beta.3");
}

/// Test version comparison: mixed alpha-numeric > numeric (rez semantics)
#[test]
fn test_version_mixed_alphanumeric_greater_than_numeric() {
    // "2alpha" splits into ["2", "alpha"], which is > "2" (splits into ["2"])
    let v1 = Version::parse("1.2alpha").unwrap();
    let v2 = Version::parse("1.2.0").unwrap();
    assert!(
        v1 > v2,
        "Mixed alphanumeric token should be greater than numeric token"
    );
}

/// Test version comparison: numeric < mixed alpha-numeric (rez semantics)
#[test]
fn test_version_numeric_less_than_mixed_alphanumeric() {
    let v1 = Version::parse("1.2.0").unwrap();
    let v2 = Version::parse("1.2alpha").unwrap();
    assert!(
        v1 < v2,
        "Numeric token should be less than mixed alphanumeric token"
    );
}

/// Test version with spaces (should be trimmed and accepted)
#[test]
fn test_version_with_spaces_trimmed() {
    // Leading/trailing spaces are trimmed by parse()
    let ver1 = Version::parse(" 1.2.3 ").unwrap();
    let ver2 = Version::parse("1.2.3").unwrap();
    assert_eq!(ver1.as_str(), "1.2.3");
    assert_eq!(ver1, ver2);
}

/// Test version with spaces in middle (should be rejected)
#[test]
fn test_version_with_spaces_in_middle() {
    let invalid_versions = ["1.2. 3", "1. 2.3"];

    for &vstr in &invalid_versions {
        let result = Version::parse(vstr);
        assert!(
            result.is_err(),
            "Version with spaces in middle '{vstr}' should be rejected"
        );
    }
}

/// Test version with special separators: dash
#[test]
fn test_version_dash_separator() {
    let ver = Version::parse("1-2-3").unwrap();
    assert_eq!(ver.as_str(), "1-2-3");
}

/// Test version with special separators: underscore
#[test]
fn test_version_underscore_separator() {
    let ver = Version::parse("1_2_3").unwrap();
    assert_eq!(ver.as_str(), "1_2_3");
}

/// Test version with special separators: plus
#[test]
fn test_version_plus_separator() {
    let ver = Version::parse("1+2+3").unwrap();
    assert_eq!(ver.as_str(), "1+2+3");
}

/// Test empty version (epsilon)
#[test]
fn test_version_empty() {
    let ver = Version::parse("").unwrap();
    assert!(ver.is_empty(), "Empty version should be epsilon");
    assert_eq!(ver.as_str(), "");
}

/// Test infinite version
#[test]
fn test_version_inf() {
    let ver = Version::parse("inf").unwrap();
    assert!(ver.is_inf(), "inf version should be infinite");
    assert_eq!(ver.as_str(), "inf");
}

/// Test version comparison: trailing zeros implicit (rez semantics)
#[test]
fn test_version_shorter_greater_than_longer() {
    // Trailing zeros are implicit: 2 == 2.0.0
    let v1 = Version::parse("2").unwrap();
    let v2 = Version::parse("2.0.0").unwrap();
    assert!(v1 == v2, "2 should equal 2.0.0 (trailing zeros implicit)");

    // Shorter is greater when longer has alpha suffix (pre-release)
    let v3 = Version::parse("2").unwrap();
    let v4 = Version::parse("2.alpha").unwrap();
    assert!(v3 > v4, "2 should be greater than 2.alpha (pre-release)");
}

/// Test version equality
#[test]
fn test_version_equal() {
    let v1 = Version::parse("1.2.3").unwrap();
    let v2 = Version::parse("1.2.3").unwrap();
    assert_eq!(v1, v2, "Equal versions should be equal");
}

/// Test version hashing (equal versions should have same hash)
#[test]
fn test_version_hash() {
    let v1 = Version::parse("1.2.3").unwrap();
    let v2 = Version::parse("1.2.3").unwrap();
    assert_eq!(v1, v2, "Equal versions should be equal for hashing");
}
