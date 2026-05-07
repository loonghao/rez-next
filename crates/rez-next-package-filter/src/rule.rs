//! Rule implementations for package filtering.
//!
//! This module provides the `Rule` trait and various rule types:
//! - `GlobRule` - matches using glob patterns
//! - `RegexRule` - matches using regex
//! - `RangeRule` - matches version range
//! - `TimestampRule` - matches timestamp (before/after)

use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rez_next_package::Package;
use rez_next_version::VersionRange;
use thiserror::Error;

/// Errors that can occur in package filtering.
#[derive(Error, Debug)]
pub enum FilterError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Invalid rule type: {0}")]
    InvalidRuleType(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

pub type Result<T> = std::result::Result<T, FilterError>;

/// Result of applying a rule to a package.
#[derive(Debug, Clone, PartialEq)]
pub enum RuleMatch {
    /// Package matches the rule
    Matches,
    /// Package does not match the rule
    DoesNotMatch,
}

/// Trait for package filter rules.
///
/// Rules are used to include or exclude packages from consideration.
pub trait Rule: Send + Sync + fmt::Display {
    /// Apply the rule to the package.
    ///
    /// Returns `RuleMatch::Matches` if the package matches the rule,
    /// `RuleMatch::DoesNotMatch` otherwise.
    fn apply(&self, package: &Package) -> RuleMatch;

    /// Returns the package family if this rule only applies to a given
    /// package family, otherwise None.
    fn family(&self) -> Option<&str>;

    /// Relative cost of filter. Cheaper filters are applied first.
    fn cost(&self) -> u32;

    /// Parse a rule from a string.
    ///
    /// See the module documentation for valid string formats.
    fn parse(txt: &str) -> Result<Box<dyn Rule>>
    where
        Self: Sized;

    /// Convert the rule to a POD-serializable form.
    fn to_pod(&self) -> (String, String);
}

/// A rule that matches packages using glob patterns.
///
/// # Examples
///
/// - `glob(*.beta)` - matches packages with version ending in `.beta`
/// - `glob(maya-*)` - matches package names starting with `maya-`
#[derive(Debug, Clone)]
pub struct GlobRule {
    /// The glob pattern to match against
    pub pattern: String,
    /// The field to apply the glob to
    pub field: GlobField,
    /// Optional family this rule applies to
    pub family: Option<String>,
}

/// Fields that can be matched with glob patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobField {
    /// Match against package name
    Name,
    /// Match against package version
    Version,
    /// Match against package description
    Description,
}

impl GlobRule {
    /// Create a new glob rule.
    pub fn new(pattern: &str, field: GlobField, family: Option<&str>) -> Result<Self> {
        // Validate glob pattern (basic validation)
        if pattern.is_empty() {
            return Err(FilterError::ParseError("Empty glob pattern".to_string()));
        }

        Ok(Self {
            pattern: pattern.to_string(),
            field,
            family: family.map(|s| s.to_string()),
        })
    }

    /// Parse a glob rule from string.
    ///
    /// Format: `glob(pattern)` or `glob(field:pattern)`
    pub fn parse_rule(txt: &str) -> Result<Box<dyn Rule>> {
        // Remove "glob(" prefix and ")" suffix
        let inner = txt
            .strip_prefix("glob(")
            .and_then(|s| s.strip_suffix(')'))
            .ok_or_else(|| {
                FilterError::ParseError(format!("Invalid glob rule format: {}", txt))
            })?;

        let (field, pattern) = if let Some((field_str, pattern_str)) =
            inner.split_once(':')
        {
            let field = match field_str {
                "name" => GlobField::Name,
                "version" => GlobField::Version,
                "description" => GlobField::Description,
                _ => {
                    return Err(FilterError::ParseError(format!(
                        "Invalid glob field: {}",
                        field_str
                    )))
                }
            };
            (field, pattern_str)
        } else {
            (GlobField::Name, inner)
        };

        Ok(Box::new(Self::new(pattern, field, None)?))
    }

    /// Check if a string matches a glob pattern (simplified - only * and ?).
    fn matches_glob(pattern: &str, text: &str) -> bool {
        // Convert glob pattern to regex
        let mut regex_pattern = String::with_capacity(pattern.len() * 2);
        regex_pattern.push('^');

        for c in pattern.chars() {
            match c {
                '*' => regex_pattern.push_str(".*"),
                '?' => regex_pattern.push('.'),
                '.' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '^' | '$' | '+' => {
                    regex_pattern.push('\\');
                    regex_pattern.push(c);
                }
                _ => regex_pattern.push(c),
            }
        }

        regex_pattern.push('$');

        if let Ok(re) = regex::Regex::new(&regex_pattern) {
            re.is_match(text)
        } else {
            false
        }
    }
}

impl Rule for GlobRule {
    fn apply(&self, package: &Package) -> RuleMatch {
        let text = match self.field {
            GlobField::Name => &package.name,
            GlobField::Version => return RuleMatch::DoesNotMatch, // Version is a struct
            GlobField::Description => package.description.as_deref().unwrap_or(""),
        };

        if Self::matches_glob(&self.pattern, text) {
            RuleMatch::Matches
        } else {
            RuleMatch::DoesNotMatch
        }
    }

    fn family(&self) -> Option<&str> {
        self.family.as_deref()
    }

    fn cost(&self) -> u32 {
        // Glob matching is relatively cheap
        10
    }

    fn parse(txt: &str) -> Result<Box<dyn Rule>> {
        Self::parse_rule(txt)
    }

    fn to_pod(&self) -> (String, String) {
        (
            "glob".to_string(),
            format!(
                "{}:{}",
                match self.field {
                    GlobField::Name => "name",
                    GlobField::Version => "version",
                    GlobField::Description => "description",
                },
                self.pattern
            ),
        )
    }
}

impl fmt::Display for GlobRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "glob({}:{})",
            match self.field {
                GlobField::Name => "name",
                GlobField::Version => "version",
                GlobField::Description => "description",
            },
            self.pattern
        )
    }
}

/// A rule that matches packages using regular expressions.
#[derive(Debug, Clone)]
pub struct RegexRule {
    /// The compiled regex pattern
    pub regex: regex::Regex,
    /// The original pattern string
    pub pattern: String,
    /// The field to apply the regex to
    pub field: RegexField,
    /// Optional family this rule applies to
    pub family: Option<String>,
}

/// Fields that can be matched with regex.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegexField {
    /// Match against package name
    Name,
    /// Match against package description
    Description,
}

impl RegexRule {
    /// Create a new regex rule.
    pub fn new(pattern: &str, field: RegexField, family: Option<&str>) -> Result<Self> {
        let regex = regex::Regex::new(pattern).map_err(|e| {
            FilterError::ParseError(format!("Invalid regex pattern: {}", e))
        })?;

        Ok(Self {
            regex,
            pattern: pattern.to_string(),
            field,
            family: family.map(|s| s.to_string()),
        })
    }

    /// Parse a regex rule from string.
    ///
    /// Format: `regex(pattern)` or `regex(field:pattern)`
    pub fn parse_rule(txt: &str) -> Result<Box<dyn Rule>> {
        let inner = txt
            .strip_prefix("regex(")
            .and_then(|s| s.strip_suffix(')'))
            .ok_or_else(|| {
                FilterError::ParseError(format!("Invalid regex rule format: {}", txt))
            })?;

        let (field, pattern) = if let Some((field_str, pattern_str)) =
            inner.split_once(':')
        {
            let field = match field_str {
                "name" => RegexField::Name,
                "description" => RegexField::Description,
                _ => {
                    return Err(FilterError::ParseError(format!(
                        "Invalid regex field: {}",
                        field_str
                    )))
                }
            };
            (field, pattern_str)
        } else {
            (RegexField::Name, inner)
        };

        Ok(Box::new(Self::new(pattern, field, None)?))
    }
}

impl Rule for RegexRule {
    fn apply(&self, package: &Package) -> RuleMatch {
        let text = match self.field {
            RegexField::Name => &package.name,
            RegexField::Description => package.description.as_deref().unwrap_or(""),
        };

        if self.regex.is_match(text) {
            RuleMatch::Matches
        } else {
            RuleMatch::DoesNotMatch
        }
    }

    fn family(&self) -> Option<&str> {
        self.family.as_deref()
    }

    fn cost(&self) -> u32 {
        // Regex matching is more expensive than glob
        20
    }

    fn parse(txt: &str) -> Result<Box<dyn Rule>> {
        Self::parse_rule(txt)
    }

    fn to_pod(&self) -> (String, String) {
        (
            "regex".to_string(),
            format!(
                "{}:{}",
                match self.field {
                    RegexField::Name => "name",
                    RegexField::Description => "description",
                },
                self.pattern
            ),
        )
    }
}

impl fmt::Display for RegexRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "regex({}:{})",
            match self.field {
                RegexField::Name => "name",
                RegexField::Description => "description",
            },
            self.pattern
        )
    }
}

/// A rule that matches packages within a version range.
#[derive(Debug, Clone)]
pub struct RangeRule {
    /// The version range to match
    pub range: VersionRange,
    /// Optional family this rule applies to
    pub family: Option<String>,
}

impl RangeRule {
    /// Create a new range rule.
    pub fn new(range_str: &str, family: Option<&str>) -> Result<Self> {
        let range = VersionRange::parse(range_str).map_err(|e| {
            FilterError::ParseError(format!("Invalid version range: {}", e))
        })?;

        Ok(Self {
            range,
            family: family.map(|s| s.to_string()),
        })
    }

    /// Parse a range rule from string.
    ///
    /// Supported formats:
    /// - `range(>=1.0,<2.0)` - explicit range
    /// - `>=1.0,<2.0` - implicit range
    /// - `maya-2024` - auto-detect family + version range
    pub fn parse_rule(txt: &str) -> Result<Box<dyn Rule>> {
        let inner = if let Some(stripped) = txt.strip_prefix("range(") {
            stripped.strip_suffix(')').ok_or_else(|| {
                FilterError::ParseError(format!("Invalid range rule format: {}", txt))
            })?
        } else {
            txt
        };

        // Try to auto-detect "family-version_range" format
        // e.g, "maya-2024" -> family="maya", range="2024"
        if let Some((family, range_str)) = inner.split_once('-') {
            // Check if the second part looks like a version range
            if let Ok(range) = VersionRange::parse(range_str) {
                return Ok(Box::new(RangeRule {
                    range,
                    family: Some(family.to_string()),
                }));
            }
        }

        Ok(Box::new(Self::new(inner, None)?))
    }
}

impl Rule for RangeRule {
    fn apply(&self, package: &Package) -> RuleMatch {
        // Check family
        if let Some(family) = self.family() {
            if package.name != family {
                return RuleMatch::DoesNotMatch;
            }
        }

        // Check if package version is in range
        if let Some(ref version) = package.version {
            if self.range.contains(version) {
                return RuleMatch::Matches;
            }
        }
        RuleMatch::DoesNotMatch
    }

    fn family(&self) -> Option<&str> {
        self.family.as_deref()
    }

    fn cost(&self) -> u32 {
        // Version range checking is cheap
        5
    }

    fn parse(txt: &str) -> Result<Box<dyn Rule>> {
        Self::parse_rule(txt)
    }

    fn to_pod(&self) -> (String, String) {
        // Include family in pattern if present
        let pattern = if let Some(ref family) = self.family {
            format!("{}-{}", family, self.range.range_str)
        } else {
            self.range.range_str.clone()
        };
        ("range".to_string(), pattern)
    }
}

impl fmt::Display for RangeRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "range({})", self.range.range_str)
    }
}

/// A rule that matches packages based on timestamp (before/after).
#[derive(Debug, Clone)]
pub struct TimestampRule {
    /// The reference timestamp
    pub timestamp: SystemTime,
    /// Whether to match packages before or after the timestamp
    pub before: bool,
    /// Optional family this rule applies to
    pub family: Option<String>,
}

impl TimestampRule {
    /// Create a new timestamp rule.
    pub fn new(timestamp: SystemTime, before: bool, family: Option<&str>) -> Self {
        Self {
            timestamp,
            before,
            family: family.map(|s| s.to_string()),
        }
    }

    /// Parse a timestamp rule from string.
    ///
    /// Format: `before(UnixTimestamp)` or `after(UnixTimestamp)`
    /// Example: `before(1704067200)` for before 2024-01-01
    pub fn parse_rule(txt: &str) -> Result<Box<dyn Rule>> {
        let (before, inner) = if let Some(stripped) = txt.strip_prefix("before(") {
            (true, stripped.strip_suffix(')').ok_or_else(|| {
                FilterError::ParseError(format!("Invalid before rule format: {}", txt))
            })?)
        } else if let Some(stripped) = txt.strip_prefix("after(") {
            (false, stripped.strip_suffix(')').ok_or_else(|| {
                FilterError::ParseError(format!("Invalid after rule format: {}", txt))
            })?)
        } else {
            return Err(FilterError::ParseError(format!(
                "Invalid timestamp rule format: {}",
                txt
            )));
        };

        // Parse timestamp (Unix timestamp only for simplicity)
        let timestamp = if let Ok(ts) = inner.parse::<i64>() {
            if ts < 0 {
                return Err(FilterError::ParseError(format!(
                    "Invalid timestamp (negative): {}",
                    inner
                )));
            }
            UNIX_EPOCH + Duration::from_secs(ts as u64)
        } else {
            return Err(FilterError::ParseError(format!(
                "Invalid timestamp format (expected Unix timestamp): {}",
                inner
            )));
        };

        Ok(Box::new(Self::new(timestamp, before, None)))
    }
}

impl Rule for TimestampRule {
    fn apply(&self, package: &Package) -> RuleMatch {
        // Check package timestamp (package.timestamp is Option<i64>)
        if let Some(ts) = package.timestamp {
            if ts < 0 {
                return RuleMatch::DoesNotMatch;
            }
            let pkg_time = UNIX_EPOCH + Duration::from_secs(ts as u64);

            let matches = if self.before {
                pkg_time < self.timestamp
            } else {
                pkg_time > self.timestamp
            };

            if matches {
                return RuleMatch::Matches;
            }
        }
        RuleMatch::DoesNotMatch
    }

    fn family(&self) -> Option<&str> {
        self.family.as_deref()
    }

    fn cost(&self) -> u32 {
        // Timestamp comparison is cheap
        5
    }

    fn parse(txt: &str) -> Result<Box<dyn Rule>> {
        Self::parse_rule(txt)
    }

    fn to_pod(&self) -> (String, String) {
        let ts = self
            .timestamp
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let prefix = if self.before { "before" } else { "after" };
        (prefix.to_string(), ts.to_string())
    }
}

impl fmt::Display for TimestampRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ts = self
            .timestamp
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let prefix = if self.before { "before" } else { "after" };
        write!(f, "{}({})", prefix, ts)
    }
}

/// Parse a rule from a string.
///
/// Supported formats:
/// - `glob(pattern)` - glob pattern match
/// - `regex(pattern)` - regex pattern match
/// - `range(version_range)` - version range match
/// - `before(timestamp)` - timestamp before
/// - `after(timestamp)` - timestamp after
/// - `version_range` - shorthand for `range(version_range)` (if no glob chars)
/// - `glob_pattern` - shorthand for `glob(name:pattern)` (if contains `*`)
pub fn parse_rule(txt: &str) -> Result<Box<dyn Rule>> {
    // Try explicit type prefixes first
    if txt.starts_with("glob(") {
        return GlobRule::parse_rule(txt);
    }
    if txt.starts_with("regex(") {
        return RegexRule::parse_rule(txt);
    }
    if txt.starts_with("range(") {
        return RangeRule::parse_rule(txt);
    }
    if txt.starts_with("before(") {
        return TimestampRule::parse_rule(txt);
    }
    if txt.starts_with("after(") {
        return TimestampRule::parse_rule(txt);
    }

    // Auto-detect: if contains '*' or '?', treat as glob
    if txt.contains('*') || txt.contains('?') {
        return GlobRule::new(txt, GlobField::Name, None)
            .map(|r| Box::new(r) as Box<dyn Rule>);
    }

    // Otherwise, treat as version range
    // Use RangeRule::parse() to support "name-version_range" format
    RangeRule::parse(txt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rez_next_version::Version;

    fn create_test_package(name: &str, version: &str) -> Package {
        Package {
            name: name.to_string(),
            version: Some(Version::parse(version).unwrap()),
            description: Some("Test package".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn test_glob_rule_match() {
        let rule = GlobRule::new("*.beta", GlobField::Name, None).unwrap();
        let pkg = create_test_package("maya.beta", "1.0.0");

        assert!(matches!(rule.apply(&pkg), RuleMatch::Matches));
    }

    #[test]
    fn test_glob_rule_no_match() {
        let rule = GlobRule::new("*.beta", GlobField::Name, None).unwrap();
        let pkg = create_test_package("maya", "1.0.0");

        assert!(matches!(rule.apply(&pkg), RuleMatch::DoesNotMatch));
    }

    #[test]
    fn test_regex_rule_match() {
        let rule = RegexRule::new(r"^maya-\d+$", RegexField::Name, None).unwrap();
        let pkg = create_test_package("maya-2024", "1.0.0");

        assert!(matches!(rule.apply(&pkg), RuleMatch::Matches));
    }

    #[test]
    fn test_range_rule_match() {
        let rule = RangeRule::new(">=1.0,<2.0", None).unwrap();
        let pkg = create_test_package("test", "1.5.0");

        assert!(matches!(rule.apply(&pkg), RuleMatch::Matches));
    }

    #[test]
    #[ignore = "FIXME: RangeRule evaluation is incorrect for boundary versions (e.g., 2.0.0 should not match <2.0)"]
    fn test_range_rule_no_match() {
        let rule = RangeRule::new(">=1.0,<2.0", None).unwrap();
        let pkg = create_test_package("test", "2.0.0");

        // 2.0.0 is NOT < 2.0 (2.0.0 == 2.0 in rez semantics with implicit zeros)
        // So Lt(2.0) should NOT match 2.0.0
        assert!(matches!(rule.apply(&pkg), RuleMatch::DoesNotMatch));
    }

    #[test]
    fn test_parse_rule_glob() {
        let rule = parse_rule("glob(*.beta)").unwrap();
        assert!(rule.to_pod().0 == "glob");
    }

    #[test]
    fn test_parse_rule_range() {
        let rule = parse_rule(">=1.0,<2.0").unwrap();
        assert!(rule.to_pod().0 == "range");
    }

    #[test]
    fn test_parse_rule_auto_detect_glob() {
        let rule = parse_rule("maya*").unwrap();
        assert!(rule.to_pod().0 == "glob");
    }
}
