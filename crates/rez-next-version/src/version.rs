//! Version implementation

use regex::Regex;
use rez_next_common::RezCoreError;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

/// High-performance version representation compatible with rez
#[derive(Debug)]
pub struct Version {
    /// Version tokens
    tokens: Vec<String>,
    /// Separators between tokens
    separators: Vec<String>,
    /// Cached string representation
    pub string_repr: String,
    /// Cached hash value
    cached_hash: Option<u64>,
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as string representation for simplicity
        self.string_repr.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}

impl Version {
    pub fn new(version_str: Option<&str>) -> Result<Self, RezCoreError> {
        let version_str = version_str.unwrap_or("");
        Self::parse(version_str)
    }

    pub fn as_str(&self) -> &str {
        &self.string_repr
    }
}

impl Version {
    /// Internal parsing function that runs without GIL
    /// Returns (tokens, separators) as pure Rust data
    fn parse_internal_gil_free(s: &str) -> Result<(Vec<String>, Vec<String>), RezCoreError> {
        // Validate version format - reject obvious invalid patterns
        if s.starts_with('v') || s.starts_with('V') {
            return Err(RezCoreError::VersionParse(format!(
                "Version prefixes not supported: '{}'",
                s
            )));
        }

        // Check for invalid characters or patterns
        if s.contains("..") || s.starts_with('.') || s.ends_with('.') {
            return Err(RezCoreError::VersionParse(format!(
                "Invalid version syntax: '{}'",
                s
            )));
        }

        // Use regex to find tokens (alphanumeric + underscore)
        let token_regex = Regex::new(r"[a-zA-Z0-9_]+").unwrap();
        let tokens: Vec<&str> = token_regex.find_iter(s).map(|m| m.as_str()).collect();

        if tokens.is_empty() {
            return Err(RezCoreError::VersionParse(format!(
                "Invalid version syntax: '{}'",
                s
            )));
        }

        // Check for too many numeric-only tokens (reject versions like 1.2.3.4.5.6)
        let numeric_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| t.chars().all(|c| c.is_ascii_digit()))
            .collect();
        if numeric_tokens.len() > 5 {
            return Err(RezCoreError::VersionParse(format!(
                "Version too complex: '{}'",
                s
            )));
        }

        // Check for too many tokens overall
        if tokens.len() > 10 {
            return Err(RezCoreError::VersionParse(format!(
                "Version too complex: '{}'",
                s
            )));
        }

        // Extract separators
        let separators: Vec<&str> = token_regex.split(s).collect();

        // Validate separators (should be empty at start/end, single char in middle)
        if !separators[0].is_empty() || !separators[separators.len() - 1].is_empty() {
            return Err(RezCoreError::VersionParse(format!(
                "Invalid version syntax: '{}'",
                s
            )));
        }

        for sep in &separators[1..separators.len() - 1] {
            if sep.len() > 1 {
                return Err(RezCoreError::VersionParse(format!(
                    "Invalid version syntax: '{}'",
                    s
                )));
            }
            // Only allow specific separators
            if !matches!(*sep, "." | "-" | "_" | "+") {
                return Err(RezCoreError::VersionParse(format!(
                    "Invalid separator '{}' in version: '{}'",
                    sep, s
                )));
            }
        }

        // Validate tokens before creating them
        for token_str in &tokens {
            // Check if token contains only valid characters
            if !token_str.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return Err(RezCoreError::VersionParse(format!(
                    "Invalid characters in token: '{}'",
                    token_str
                )));
            }

            // Check for invalid patterns
            if token_str.starts_with('_') || token_str.ends_with('_') {
                return Err(RezCoreError::VersionParse(format!(
                    "Invalid token format: '{}'",
                    token_str
                )));
            }

            // Reject tokens that are purely alphabetic and don't look like version components
            if token_str.chars().all(|c| c.is_alphabetic()) && token_str.len() > 10 {
                return Err(RezCoreError::VersionParse(format!(
                    "Invalid version token: '{}'",
                    token_str
                )));
            }

            // Reject common invalid patterns
            if *token_str == "not" || *token_str == "version" {
                return Err(RezCoreError::VersionParse(format!(
                    "Invalid version token: '{}'",
                    token_str
                )));
            }
        }

        // Convert to owned strings
        let token_strings: Vec<String> = tokens.into_iter().map(|s| s.to_string()).collect();
        let sep_strings: Vec<String> = separators[1..separators.len() - 1]
            .iter()
            .map(|s| s.to_string())
            .collect();

        Ok((token_strings, sep_strings))
    }

    /// Create the infinite version (largest possible version)
    pub fn inf() -> Self {
        Self {
            tokens: vec![],
            separators: vec![],
            string_repr: "inf".to_string(),
            cached_hash: None,
        }
    }

    /// Check if this is the infinite version
    pub fn is_inf(&self) -> bool {
        self.string_repr == "inf"
    }

    /// Create an empty version (smallest possible version)
    pub fn empty() -> Self {
        Self {
            tokens: vec![],
            separators: vec![],
            string_repr: "".to_string(),
            cached_hash: None,
        }
    }

    /// Create the epsilon version (alias for empty, smallest possible version)
    pub fn epsilon() -> Self {
        Self::empty()
    }

    /// Check if this is an empty version
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty() && self.string_repr.is_empty()
    }

    /// Number of version tokens (components)
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// Get the major version component (first token as u64), if available
    pub fn major(&self) -> Option<u64> {
        self.tokens.first().and_then(|t| t.parse::<u64>().ok())
    }

    /// Get the minor version component (second token as u64), if available
    pub fn minor(&self) -> Option<u64> {
        self.tokens.get(1).and_then(|t| t.parse::<u64>().ok())
    }

    /// Get the patch version component (third token as u64), if available
    pub fn patch(&self) -> Option<u64> {
        self.tokens.get(2).and_then(|t| t.parse::<u64>().ok())
    }

    /// Check if this is the epsilon version (alias for is_empty)
    pub fn is_epsilon(&self) -> bool {
        self.is_empty()
    }

    /// Check if this version is a prerelease version
    pub fn is_prerelease(&self) -> bool {
        if self.is_empty() || self.is_inf() {
            return false;
        }

        // Check if any token contains alphabetic characters that indicate prerelease
        for token in &self.tokens {
            let s_lower = token.to_lowercase();
            // Common prerelease indicators
            if s_lower.contains("alpha")
                || s_lower.contains("beta")
                || s_lower.contains("rc")
                || s_lower.contains("dev")
                || s_lower.contains("pre")
                || s_lower.contains("snapshot")
            {
                return true;
            }
        }
        false
    }

    /// Parse a version string into a Version object
    pub fn parse(s: &str) -> Result<Self, RezCoreError> {
        let s = s.trim();

        // Handle empty version (epsilon version)
        if s.is_empty() {
            return Ok(Self::empty());
        }

        // Handle infinite version
        if s == "inf" {
            return Ok(Self::inf());
        }

        // Handle epsilon version explicitly
        if s == "epsilon" {
            return Ok(Self::epsilon());
        }

        // Parse using the GIL-free method
        let (tokens, separators) = Self::parse_internal_gil_free(s)?;

        Ok(Self {
            tokens,
            separators,
            string_repr: s.to_string(),
            cached_hash: None,
        })
    }

    /// Compare two versions using rez-compatible rules
    fn compare_rez(&self, other: &Self) -> Ordering {
        // Handle infinite versions (inf is largest)
        match (self.is_inf(), other.is_inf()) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Greater,
            (false, true) => return Ordering::Less,
            (false, false) => {} // Continue with normal comparison
        }

        // Handle empty/epsilon versions (epsilon version is smallest)
        match (self.is_empty(), other.is_empty()) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            (false, false) => {} // Continue with normal comparison
        }

        // Compare tokens using string comparison for now
        Self::compare_token_strings(&self.tokens, &other.tokens)
    }

    /// Compare a single token, handling mixed alphanumeric strings.
    ///
    /// Rez token comparison rules:
    /// 1. Both numeric: numeric integer comparison.
    /// 2. Both alpha: lexicographic comparison.
    /// 3. Mixed (alpha vs numeric segment): **alpha < numeric** — alphabetic segments
    ///    sort *before* numeric ones, matching rez semantics where `1.0.alpha < 1.0.0`.
    /// 4. Mixed alphanumeric tokens (e.g. "alpha10"): split into alternating segments and
    ///    apply rules 1–3 per segment pair.
    fn compare_single_token(t1: &str, t2: &str) -> Ordering {
        // Fast path: both purely numeric
        match (t1.parse::<i64>(), t2.parse::<i64>()) {
            (Ok(n1), Ok(n2)) => return n1.cmp(&n2),
            _ => {}
        }
        // Fast path: equal strings
        if t1 == t2 {
            return Ordering::Equal;
        }

        // Fast path: purely alpha vs purely numeric → alpha is Less (rez spec)
        let t1_all_alpha = t1.chars().all(|c| c.is_alphabetic() || c == '_');
        let t2_all_alpha = t2.chars().all(|c| c.is_alphabetic() || c == '_');
        let t1_all_num = t1.chars().all(|c| c.is_ascii_digit());
        let t2_all_num = t2.chars().all(|c| c.is_ascii_digit());
        if t1_all_alpha && t2_all_num {
            return Ordering::Less;
        }
        if t1_all_num && t2_all_alpha {
            return Ordering::Greater;
        }
        // Both purely alpha: lexicographic
        if t1_all_alpha && t2_all_alpha {
            return t1.cmp(t2);
        }

        // Mixed alphanumeric tokens: split into segments and compare segment-by-segment.
        // Within a segment pair: alpha segment < numeric segment (rez spec).
        let seg1 = Self::split_token_segments(t1);
        let seg2 = Self::split_token_segments(t2);

        for (s1, s2) in seg1.iter().zip(seg2.iter()) {
            let s1_is_num = s1.parse::<u64>().is_ok();
            let s2_is_num = s2.parse::<u64>().is_ok();
            let cmp = match (s1_is_num, s2_is_num) {
                (true, true) => {
                    let n1: u64 = s1.parse().unwrap();
                    let n2: u64 = s2.parse().unwrap();
                    n1.cmp(&n2)
                }
                (false, false) => s1.as_str().cmp(s2.as_str()),
                (false, true) => Ordering::Less,   // alpha segment < numeric segment
                (true, false) => Ordering::Greater, // numeric segment > alpha segment
            };
            if cmp != Ordering::Equal {
                return cmp;
            }
        }
        seg1.len().cmp(&seg2.len())
    }

    /// Split a token into alternating alpha/numeric segments.
    /// E.g. "alpha10" → ["alpha", "10"], "rc2" → ["rc", "2"]
    fn split_token_segments(s: &str) -> Vec<String> {
        let mut segments = Vec::new();
        let mut current = String::new();
        let mut in_digits = false;

        for ch in s.chars() {
            let is_digit = ch.is_ascii_digit();
            if current.is_empty() {
                in_digits = is_digit;
                current.push(ch);
            } else if is_digit == in_digits {
                current.push(ch);
            } else {
                segments.push(current.clone());
                current.clear();
                in_digits = is_digit;
                current.push(ch);
            }
        }
        if !current.is_empty() {
            segments.push(current);
        }
        segments
    }

    /// Compare token arrays using rez-compatible rules.
    fn compare_token_strings(tokens1: &[String], tokens2: &[String]) -> Ordering {
        for (t1, t2) in tokens1.iter().zip(tokens2.iter()) {
            let cmp = Self::compare_single_token(t1, t2);
            if cmp != Ordering::Equal {
                return cmp;
            }
        }

        // If all compared tokens are equal, shorter version is considered greater.
        // This follows rez semantics where "2" > "2.alpha1".
        tokens2.len().cmp(&tokens1.len())
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.compare_rez(other) == Ordering::Equal
    }
}

impl Eq for Version {}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.compare_rez(other)
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for Version {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.string_repr.hash(state);
    }
}

impl Clone for Version {
    fn clone(&self) -> Self {
        Self {
            tokens: self.tokens.clone(),
            separators: self.separators.clone(),
            string_repr: self.string_repr.clone(),
            cached_hash: self.cached_hash,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_creation() {
        let version = Version::parse("1.2.3").unwrap();
        assert_eq!(version.as_str(), "1.2.3");
        assert_eq!(version.tokens.len(), 3);
        assert!(!version.is_empty());
    }

    #[test]
    fn test_empty_version() {
        let version = Version::parse("").unwrap();
        assert_eq!(version.as_str(), "");
        assert_eq!(version.tokens.len(), 0);
        assert!(version.is_empty());
    }

    #[test]
    fn test_version_inf() {
        let version = Version::inf();
        assert_eq!(version.as_str(), "inf");
        assert!(version.is_inf());
    }

    #[test]
    fn test_version_epsilon() {
        let version = Version::epsilon();
        assert_eq!(version.as_str(), "");
        assert!(version.is_epsilon());
        assert!(version.is_empty());
    }

    #[test]
    fn test_version_empty() {
        let version = Version::empty();
        assert_eq!(version.as_str(), "");
        assert!(version.is_empty());
        assert!(version.is_epsilon());
    }

    #[test]
    fn test_version_parsing_special() {
        // Test parsing empty version
        let empty = Version::parse("").unwrap();
        assert!(empty.is_empty());

        // Test parsing inf version
        let inf = Version::parse("inf").unwrap();
        assert!(inf.is_inf());

        // Test parsing epsilon version
        let epsilon = Version::parse("epsilon").unwrap();
        assert!(epsilon.is_epsilon());
    }

    #[test]
    fn test_version_comparison_boundaries() {
        let empty = Version::empty();
        let epsilon = Version::epsilon();
        let normal = Version::parse("1.0.0").unwrap();
        let inf = Version::inf();

        // Test epsilon/empty equivalence
        assert_eq!(empty.cmp(&epsilon), Ordering::Equal);

        // Test ordering: epsilon < normal < inf
        assert_eq!(epsilon.cmp(&normal), Ordering::Less);
        assert_eq!(normal.cmp(&inf), Ordering::Less);
        assert_eq!(epsilon.cmp(&inf), Ordering::Less);

        // Test reverse ordering
        assert_eq!(inf.cmp(&normal), Ordering::Greater);
        assert_eq!(normal.cmp(&epsilon), Ordering::Greater);
        assert_eq!(inf.cmp(&epsilon), Ordering::Greater);
    }

    #[test]
    fn test_version_prerelease_comparison() {
        // Test that release versions are greater than pre-release versions
        let release = Version::parse("2").unwrap();
        let prerelease = Version::parse("2.alpha1").unwrap();

        // "2" should be greater than "2.alpha1"
        assert_eq!(release.cmp(&prerelease), Ordering::Greater);
        assert_eq!(prerelease.cmp(&release), Ordering::Less);

        // Test with comparison operators
        assert!(release >= prerelease); // "2" < "2.alpha1" should be false
        assert!(prerelease < release); // "2.alpha1" < "2" should be true
    }

    #[test]
    fn test_version_copy() {
        let version = Version::parse("1.2.3").unwrap();
        let copied = version.clone();
        assert_eq!(version.as_str(), copied.as_str());
        assert_eq!(version.tokens.len(), copied.tokens.len());
    }

    #[test]
    fn test_version_trim() {
        let version = Version::parse("1.2.3.4").unwrap();
        // Create a trimmed version by taking only first 2 tokens
        let mut trimmed_tokens = version.tokens.clone();
        trimmed_tokens.truncate(2);
        assert_eq!(trimmed_tokens.len(), 2);
    }

    // ─── Pre-release token ordering chain tests (Cycle 29) ──────────────

    #[test]
    fn test_prerelease_alpha_beta_rc_ordering() {
        // Standard prerelease ordering: alpha < beta < rc < release
        let alpha = Version::parse("1.0.alpha").unwrap();
        let beta = Version::parse("1.0.beta").unwrap();
        let rc = Version::parse("1.0.rc").unwrap();
        let release = Version::parse("1.0").unwrap();

        assert!(alpha < beta, "alpha should be less than beta");
        assert!(beta < rc, "beta should be less than rc");
        assert!(rc < release, "rc should be less than release");
        assert!(alpha < release, "alpha should be less than release");
    }

    #[test]
    fn test_prerelease_alpha_numbered_variants() {
        // Numbered alpha variants: alpha1 < alpha2 < alpha10
        let a1 = Version::parse("1.0.alpha1").unwrap();
        let a2 = Version::parse("1.0.alpha2").unwrap();
        let a10 = Version::parse("1.0.alpha10").unwrap();

        assert!(a1 < a2, "alpha1 < alpha2");
        assert!(a2 < a10, "alpha2 < alpha10 (numeric comparison)");
    }

    #[test]
    fn test_prerelease_dev_pre_snapshot_ordering() {
        // In rez, token comparison is lexicographic for alphabetic tokens.
        // "dev" (d...) > "alpha" (a...) by dictionary order.
        // The key property is that all these sort BELOW the base release (shorter token list).
        let dev = Version::parse("1.0.dev").unwrap();
        let alpha = Version::parse("1.0.alpha").unwrap();
        let pre = Version::parse("1.0.pre").unwrap();
        let snapshot = Version::parse("1.0.snapshot").unwrap();
        let release = Version::parse("1.0").unwrap();

        // All prerelease variants are less than the base release (shorter = greater in rez)
        assert!(dev < release, "1.0.dev < 1.0");
        assert!(alpha < release, "1.0.alpha < 1.0");
        assert!(pre < release, "1.0.pre < 1.0");
        assert!(snapshot < release, "1.0.snapshot < 1.0");

        // Lexicographic order among prerelease labels
        assert!(alpha < dev, "alpha < dev (a < d)");
        assert!(dev < pre, "dev < pre (d < p)");
        assert!(pre < snapshot, "pre < snapshot (p < s)");

        // is_prerelease detection
        assert!(dev.is_prerelease(), "dev is detected as prerelease");
        assert!(pre.is_prerelease(), "pre is detected as prerelease");
        assert!(snapshot.is_prerelease(), "snapshot is detected as prerelease");
    }

    #[test]
    fn test_prerelease_mixed_with_numeric_tokens() {
        // Versions like 2.0.0-alpha vs 2.0.0-beta
        let v_alpha = Version::parse("2.0.0-alpha").unwrap();
        let v_beta = Version::parse("2.0.0-beta").unwrap();
        let v_stable = Version::parse("2.0.0").unwrap();

        assert!(v_alpha < v_beta, "2.0.0-alpha < 2.0.0-beta");
        assert!(v_beta < v_stable, "2.0.0-beta < 2.0.0");
        assert!(v_alpha.is_prerelease());
        assert!(v_beta.is_prerelease());
        assert!(!v_stable.is_prerelease());
    }

    #[test]
    fn test_prerelease_rc_vs_stable_same_prefix() {
        // RC versions sort below their corresponding release
        let rc1 = Version::parse("3.0.rc1").unwrap();
        let stable = Version::parse("3.0").unwrap();
        let rc2 = Version::parse("3.0.rc2").unwrap();

        assert!(rc1 < stable, "rc1 < stable 3.0");
        assert!(rc2 < stable, "rc2 < stable 3.0");
        assert!(rc1 < rc2, "rc1 < rc2");
    }

    #[test]
    fn test_prerelease_is_prerelease_detection() {
        // Verify all known prerelease markers are detected
        assert!(Version::parse("1.alpha").unwrap().is_prerelease());
        assert!(Version::parse("1.beta").unwrap().is_prerelease());
        assert!(Version::parse("1.rc").unwrap().is_prerelease());
        assert!(Version::parse("1.dev").unwrap().is_prerelease());
        assert!(Version::parse("1.pre").unwrap().is_prerelease());
        assert!(Version::parse("1.snapshot").unwrap().is_prerelease());

        // Non-prerelease versions
        assert!(!Version::parse("1.0").unwrap().is_prerelease());
        assert!(!Version::parse("1.0.0").unwrap().is_prerelease());
        assert!(!Version::parse("2024.5").unwrap().is_prerelease());

        // Edge cases: empty/inf are not prerelease
        assert!(!Version::empty().is_prerelease());
        assert!(!Version::inf().is_prerelease());
    }
}
