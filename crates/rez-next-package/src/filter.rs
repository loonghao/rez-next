//! Package filtering module for rez-next.
//!
//! This module provides package filtering capabilities compatible with rez's
//! `package_filter.py`. It supports multiple rule types (glob, regex, range, timestamp)
//! and combination of inclusion/exclusion rules.

use crate::package::Package;
use crate::requirement::Requirement;
use regex::Regex;
use std::collections::HashMap;
use std::fmt;

// ── Rule Trait ─────────────────────────────────────────────────────────────────

/// Trait for all package filter rules.
///
/// Compatible with `rez.package_filter.Rule`.
pub trait Rule: fmt::Debug {
    /// Check if this rule matches the given package.
    fn matches(&self, package: &Package) -> bool;

    /// Get the rule type name for serialization.
    fn rule_type(&self) -> &str;

    /// Convert rule to POD (Plain Old Data) format for serialization.
    fn to_pod(&self) -> String;

    /// Clone the rule into a Box. Required for cloning trait objects.
    fn clone_box(&self) -> Box<dyn Rule + Send + Sync>;
}

// ── GlobRule ───────────────────────────────────────────────────────────────────

/// Glob pattern rule for package filtering.
///
/// Matches packages where the family name matches the glob pattern.
/// Compatible with `rez.package_filter.GlobRule`.
#[derive(Debug, Clone)]
pub struct GlobRule {
    pub pattern: String,
    compiled: glob::Pattern,
}

impl GlobRule {
    /// Create a new glob rule with the given pattern.
    pub fn new(pattern: &str) -> Result<Self, glob::PatternError> {
        let compiled = glob::Pattern::new(pattern)?;
        Ok(GlobRule {
            pattern: pattern.to_string(),
            compiled,
        })
    }
}

impl Rule for GlobRule {
    fn matches(&self, package: &Package) -> bool {
        self.compiled.matches(&package.name)
    }

    fn rule_type(&self) -> &str {
        "glob"
    }

    fn to_pod(&self) -> String {
        format!("glob({})", self.pattern)
    }

    fn clone_box(&self) -> Box<dyn Rule + Send + Sync> {
        Box::new(self.clone())
    }
}

impl fmt::Display for GlobRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GlobRule({})", self.pattern)
    }
}

// ── RegexRule ──────────────────────────────────────────────────────────────────

/// Regex pattern rule for package filtering.
///
/// Matches packages where the family name matches the regex pattern.
/// Compatible with `rez.package_filter.RegexRule`.
#[derive(Debug, Clone)]
pub struct RegexRule {
    pub pattern: String,
    compiled: Regex,
}

impl RegexRule {
    /// Create a new regex rule with the given pattern.
    pub fn new(pattern: &str) -> Result<Self, regex::Error> {
        let compiled = Regex::new(pattern)?;
        Ok(RegexRule {
            pattern: pattern.to_string(),
            compiled,
        })
    }
}

impl Rule for RegexRule {
    fn matches(&self, package: &Package) -> bool {
        self.compiled.is_match(&package.name)
    }

    fn rule_type(&self) -> &str {
        "regex"
    }

    fn to_pod(&self) -> String {
        format!("regex({})", self.pattern)
    }

    fn clone_box(&self) -> Box<dyn Rule + Send + Sync> {
        Box::new(self.clone())
    }
}

impl fmt::Display for RegexRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RegexRule({})", self.pattern)
    }
}

// ── RangeRule ─────────────────────────────────────────────────────────────────

/// Version range rule for package filtering.
///
/// Matches packages where the version satisfies the given requirement.
/// Compatible with `rez.package_filter.RangeRule`.
#[derive(Debug, Clone)]
pub struct RangeRule {
    pub requirement: Requirement,
}

impl RangeRule {
    /// Create a new range rule from a requirement string.
    pub fn new(req_str: &str) -> Result<Self, String> {
        let parser = crate::requirement::RequirementParser::new();
        let requirement = parser.parse(req_str).map_err(|e| e.to_string())?;
        Ok(RangeRule { requirement })
    }
}

impl Rule for RangeRule {
    fn matches(&self, package: &Package) -> bool {
        if let Some(ref version) = package.version {
            if let Some(ref vc) = self.requirement.version_constraint {
                vc.is_satisfied_by(version)
            } else {
                false
            }
        } else {
            false
        }
    }

    fn rule_type(&self) -> &str {
        "range"
    }

    fn to_pod(&self) -> String {
        format!("range({})", self.requirement)
    }

    fn clone_box(&self) -> Box<dyn Rule + Send + Sync> {
        Box::new(self.clone())
    }
}

impl fmt::Display for RangeRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RangeRule({})", self.requirement)
    }
}

// ── TimestampRule ──────────────────────────────────────────────────────────────

/// Timestamp rule for package filtering.
///
/// Matches packages based on their timestamp (release time).
/// Compatible with `rez.package_filter.TimestampRule`.
#[derive(Debug, Clone)]
pub struct TimestampRule {
    pub timestamp: i64,
    pub inclusive: bool,
    pub before: bool,
}

impl TimestampRule {
    /// Create a rule that matches packages released after the given timestamp.
    pub fn after(timestamp: i64) -> Self {
        TimestampRule {
            timestamp,
            inclusive: false,
            before: false,
        }
    }

    /// Create a rule that matches packages released before the given timestamp.
    pub fn before(timestamp: i64) -> Self {
        TimestampRule {
            timestamp,
            inclusive: false,
            before: true,
        }
    }

    /// Create a rule that matches packages released after or at the given timestamp.
    pub fn at_or_after(timestamp: i64) -> Self {
        TimestampRule {
            timestamp,
            inclusive: true,
            before: false,
        }
    }

    /// Create a rule that matches packages released before or at the given timestamp.
    pub fn at_or_before(timestamp: i64) -> Self {
        TimestampRule {
            timestamp,
            inclusive: true,
            before: true,
        }
    }
}

impl Rule for TimestampRule {
    fn matches(&self, package: &Package) -> bool {
        if let Some(ts) = package.timestamp {
            let ts_i64 = ts as i64;
            if self.before {
                if self.inclusive {
                    ts_i64 <= self.timestamp
                } else {
                    ts_i64 < self.timestamp
                }
            } else {
                if self.inclusive {
                    ts_i64 >= self.timestamp
                } else {
                    ts_i64 > self.timestamp
                }
            }
        } else {
            false
        }
    }

    fn rule_type(&self) -> &str {
        "timestamp"
    }

    fn to_pod(&self) -> String {
        let prefix = if self.before {
            "before"
        } else {
            "after"
        };
        let incl = if self.inclusive { ":" } else { "" };
        format!("timestamp({}{}{})", prefix, incl, self.timestamp)
    }

    fn clone_box(&self) -> Box<dyn Rule + Send + Sync> {
        Box::new(self.clone())
    }
}

impl fmt::Display for TimestampRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let direction = if self.before { "before" } else { "after" };
        let incl = if self.inclusive { " (inclusive)" } else { "" };
        write!(
            f,
            "TimestampRule({} {}{})",
            direction, self.timestamp, incl
        )
    }
}

// ── PackageFilter ─────────────────────────────────────────────────────────────

/// Package filter with inclusion and exclusion rules.
///
/// A package is excluded if it matches ANY exclusion rule AND does not match
/// ANY inclusion rule.
/// Compatible with `rez.package_filter.PackageFilter`.
#[derive(Debug)]
pub struct PackageFilter {
    pub excludes: HashMap<Option<String>, Vec<Box<dyn Rule + Send + Sync>>>,
    pub includes: HashMap<Option<String>, Vec<Box<dyn Rule + Send + Sync>>>,
}

impl Clone for PackageFilter {
    fn clone(&self) -> Self {
        let excludes = self.excludes.iter().map(|(k, v)| {
            let cloned_vec: Vec<Box<dyn Rule + Send + Sync>> =
                v.iter().map(|r| r.clone_box()).collect();
            (k.clone(), cloned_vec)
        }).collect();

        let includes = self.includes.iter().map(|(k, v)| {
            let cloned_vec: Vec<Box<dyn Rule + Send + Sync>> =
                v.iter().map(|r| r.clone_box()).collect();
            (k.clone(), cloned_vec)
        }).collect();

        PackageFilter {
            excludes,
            includes,
        }
    }
}

impl Default for PackageFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl PackageFilter {
    /// Create a new empty package filter.
    pub fn new() -> Self {
        PackageFilter {
            excludes: HashMap::new(),
            includes: HashMap::new(),
        }
    }

    /// Add an exclusion rule for a specific package family (or all packages if family is None).
    pub fn add_exclusion(&mut self, family: Option<&str>, rule: Box<dyn Rule + Send + Sync>) {
        self.excludes
            .entry(family.map(String::from))
            .or_insert_with(Vec::new)
            .push(rule);
    }

    /// Add an inclusion rule for a specific package family (or all packages if family is None).
    pub fn add_inclusion(&mut self, family: Option<&str>, rule: Box<dyn Rule + Send + Sync>) {
        self.includes
            .entry(family.map(String::from))
            .or_insert_with(Vec::new)
            .push(rule);
    }

    /// Check if a package is excluded by this filter.
    ///
    /// Returns `true` if the package should be excluded (hidden).
    /// A package is excluded if:
    /// 1. It matches at least one exclusion rule, AND
    /// 2. It does NOT match any inclusion rule
    pub fn excludes(&self, package: &Package) -> bool {
        let family = &package.name;

        // Check family-specific rules first, then global rules
        let mut exclude_rules: Vec<&Box<dyn Rule + Send + Sync>> = Vec::new();
        if let Some(rules) = self.excludes.get(&Some(family.clone())) {
            exclude_rules.extend(rules.iter());
        }
        if let Some(rules) = self.excludes.get(&None) {
            exclude_rules.extend(rules.iter());
        }

        let mut include_rules: Vec<&Box<dyn Rule + Send + Sync>> = Vec::new();
        if let Some(rules) = self.includes.get(&Some(family.clone())) {
            include_rules.extend(rules.iter());
        }
        if let Some(rules) = self.includes.get(&None) {
            include_rules.extend(rules.iter());
        }

        // Check if package matches any exclusion rule
        let is_excluded = exclude_rules.iter().any(|r| r.matches(package));

        if !is_excluded {
            return false;
        }

        // Check if package matches any inclusion rule (inclusion overrides exclusion)
        let has_inclusion = include_rules.iter().any(|r| r.matches(package));

        !has_inclusion
    }

    /// Filter a list of packages, returning only non-excluded packages.
    pub fn filter_packages(&self, packages: Vec<Package>) -> Vec<Package> {
        packages
            .into_iter()
            .filter(|p| !self.excludes(p))
            .collect()
    }

    /// Convert the filter to POD format for serialization.
    pub fn to_pod(&self) -> HashMap<String, Vec<String>> {
        let mut pod = HashMap::new();

        let excludes: Vec<String> = self
            .excludes
            .values()
            .flatten()
            .map(|r| r.to_pod())
            .collect();
        if !excludes.is_empty() {
            pod.insert("excludes".to_string(), excludes);
        }

        let includes: Vec<String> = self
            .includes
            .values()
            .flatten()
            .map(|r| r.to_pod())
            .collect();
        if !includes.is_empty() {
            pod.insert("includes".to_string(), includes);
        }

        pod
    }

    /// Create a filter from POD format.
    pub fn from_pod(pod: &HashMap<String, Vec<String>>) -> Result<Self, String> {
        let mut filter = PackageFilter::new();

        if let Some(excludes) = pod.get("excludes") {
            for rule_str in excludes {
                if let Some(rule) = parse_rule(rule_str) {
                    filter.add_exclusion(None, rule);
                } else {
                    return Err(format!("Failed to parse exclusion rule: {}", rule_str));
                }
            }
        }

        if let Some(includes) = pod.get("includes") {
            for rule_str in includes {
                if let Some(rule) = parse_rule(rule_str) {
                    filter.add_inclusion(None, rule);
                } else {
                    return Err(format!("Failed to parse inclusion rule: {}", rule_str));
                }
            }
        }

        Ok(filter)
    }
}

impl fmt::Display for PackageFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let num_excludes: usize = self.excludes.values().map(|v| v.len()).sum();
        let num_includes: usize = self.includes.values().map(|v| v.len()).sum();
        write!(
            f,
            "PackageFilter(excludes={}, includes={})",
            num_excludes, num_includes
        )
    }
}

// ── PackageFilterList ─────────────────────────────────────────────────────────

/// A list of package filters.
///
/// A package is excluded if ANY filter in the list excludes it.
/// Compatible with `rez.package_filter.PackageFilterList`.
#[derive(Debug, Clone)]
pub struct PackageFilterList {
    pub filters: Vec<PackageFilter>,
}

impl Default for PackageFilterList {
    fn default() -> Self {
        Self::new()
    }
}

impl PackageFilterList {
    /// Create a new empty filter list.
    pub fn new() -> Self {
        PackageFilterList {
            filters: Vec::new(),
        }
    }

    /// Add a filter to the list.
    pub fn add_filter(&mut self, filter: PackageFilter) {
        self.filters.push(filter);
    }

    /// Check if a package is excluded by any filter in the list.
    pub fn excludes(&self, package: &Package) -> bool {
        self.filters.iter().any(|f| f.excludes(package))
    }

    /// Filter a list of packages, returning only non-excluded packages.
    pub fn filter_packages(&self, packages: Vec<Package>) -> Vec<Package> {
        packages
            .into_iter()
            .filter(|p| !self.excludes(p))
            .collect()
    }
}

impl fmt::Display for PackageFilterList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PackageFilterList(filters={})", self.filters.len())
    }
}

// ── Rule Parsing ──────────────────────────────────────────────────────────────

/// Parse a rule from its string representation.
///
/// Supports the following formats:
/// - `glob(pattern)` - Glob pattern
/// - `*.pattern` - Auto-detected as glob
/// - `regex(pattern)` - Regex pattern
/// - `range(req)` - Version range
/// - `timestamp(before:ts)` or `timestamp(after:ts)` - Timestamp rule
pub fn parse_rule(s: &str) -> Option<Box<dyn Rule + Send + Sync>> {
    let s = s.trim();

    if s.starts_with("glob(") && s.ends_with(')') {
        let pattern = &s[5..s.len() - 1];
        return GlobRule::new(pattern)
            .ok()
            .map(|r| Box::new(r) as Box<dyn Rule + Send + Sync>);
    }

    if s.starts_with("regex(") && s.ends_with(')') {
        let pattern = &s[6..s.len() - 1];
        return RegexRule::new(pattern)
            .ok()
            .map(|r| Box::new(r) as Box<dyn Rule + Send + Sync>);
    }

    if s.starts_with("range(") && s.ends_with(')') {
        let req = &s[6..s.len() - 1];
        return RangeRule::new(req)
            .ok()
            .map(|r| Box::new(r) as Box<dyn Rule + Send + Sync>);
    }

    if s.starts_with("timestamp(") && s.ends_with(')') {
        let inner = &s[10..s.len() - 1];
        let parts: Vec<&str> = inner.splitn(2, ':').collect();
        if parts.len() == 2 {
            let timestamp = parts[1].parse::<i64>().ok()?;
            if parts[0] == "before" {
                return Some(Box::new(TimestampRule::before(timestamp))
                    as Box<dyn Rule + Send + Sync>);
            } else if parts[0] == "after" {
                return Some(Box::new(TimestampRule::after(timestamp))
                    as Box<dyn Rule + Send + Sync>);
            }
        }
    }

    // Auto-detect: if it contains *, ?, or [ ] it's a glob
    if s.contains('*') || s.contains('?') || (s.contains('[') && s.contains(']')) {
        return GlobRule::new(s)
            .ok()
            .map(|r| Box::new(r) as Box<dyn Rule + Send + Sync>);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::Package;
    use rez_next_version::Version;

    fn make_package(name: &str, version: &str) -> Package {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse(version).unwrap());
        pkg
    }

    // ── GlobRule Tests ─────────────────────────────────────────────────────

    #[test]
    fn test_glob_rule_matches() {
        let rule = GlobRule::new("*.beta").unwrap();
        let pkg = make_package("foo.beta", "1.0.0");
        assert!(rule.matches(&pkg));
    }

    #[test]
    fn test_glob_rule_no_match() {
        let rule = GlobRule::new("*.beta").unwrap();
        let pkg = make_package("foo", "1.0.0");
        assert!(!rule.matches(&pkg));
    }

    #[test]
    fn test_glob_rule_to_pod() {
        let rule = GlobRule::new("*.beta").unwrap();
        assert_eq!(rule.to_pod(), "glob(*.beta)");
    }

    // ── RegexRule Tests ────────────────────────────────────────────────────

    #[test]
    fn test_regex_rule_matches() {
        let rule = RegexRule::new(r".*\.beta$").unwrap();
        let pkg = make_package("foo.beta", "1.0.0");
        assert!(rule.matches(&pkg));
    }

    #[test]
    fn test_regex_rule_no_match() {
        let rule = RegexRule::new(r".*\.beta$").unwrap();
        let pkg = make_package("foo", "1.0.0");
        assert!(!rule.matches(&pkg));
    }

    // ── RangeRule Tests ────────────────────────────────────────────────────

    #[test]
    fn test_range_rule_matches() {
        let rule = RangeRule::new("foo<2.0").unwrap();
        let pkg = make_package("foo", "1.0.0");
        assert!(rule.matches(&pkg));
    }

    #[test]
    fn test_range_rule_no_match() {
        let rule = RangeRule::new("foo<1.0").unwrap();
        let pkg = make_package("foo", "1.0.0");
        assert!(!rule.matches(&pkg));
    }

    // ── TimestampRule Tests ───────────────────────────────────────────────

    #[test]
    fn test_timestamp_rule_after() {
        let rule = TimestampRule::after(1000);
        let mut pkg = make_package("foo", "1.0.0");
        pkg.timestamp = Some(2000);
        assert!(rule.matches(&pkg));
    }

    #[test]
    fn test_timestamp_rule_before() {
        let rule = TimestampRule::before(3000);
        let mut pkg = make_package("foo", "1.0.0");
        pkg.timestamp = Some(2000);
        assert!(rule.matches(&pkg));
    }

    #[test]
    fn test_timestamp_rule_no_match() {
        let rule = TimestampRule::after(3000);
        let mut pkg = make_package("foo", "1.0.0");
        pkg.timestamp = Some(2000);
        assert!(!rule.matches(&pkg));
    }

    // ── PackageFilter Tests ───────────────────────────────────────────────

    #[test]
    fn test_filter_excludes_matching_package() {
        let mut filter = PackageFilter::new();
        filter.add_exclusion(None, Box::new(GlobRule::new("*.beta").unwrap()));

        let pkg = make_package("foo.beta", "1.0.0");
        assert!(filter.excludes(&pkg));
    }

    #[test]
    fn test_filter_inclusion_overrides_exclusion() {
        let mut filter = PackageFilter::new();
        filter.add_exclusion(None, Box::new(GlobRule::new("*.beta").unwrap()));
        filter.add_inclusion(None, Box::new(GlobRule::new("foo.beta").unwrap()));

        let pkg = make_package("foo.beta", "1.0.0");
        assert!(!filter.excludes(&pkg)); // Included, so not excluded
    }

    #[test]
    fn test_filter_filter_packages() {
        let mut filter = PackageFilter::new();
        filter.add_exclusion(None, Box::new(GlobRule::new("*.beta").unwrap()));

        let pkgs = vec![
            make_package("foo", "1.0.0"),
            make_package("foo.beta", "1.0.0"),
            make_package("bar", "2.0.0"),
        ];

        let filtered = filter.filter_packages(pkgs);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].name, "foo");
        assert_eq!(filtered[1].name, "bar");
    }

    #[test]
    fn test_filter_to_pod_and_back() {
        let mut filter = PackageFilter::new();
        filter.add_exclusion(None, Box::new(GlobRule::new("*.beta").unwrap()));

        let pod = filter.to_pod();
        let restored = PackageFilter::from_pod(&pod).unwrap();

        let pkg = make_package("foo.beta", "1.0.0");
        assert_eq!(filter.excludes(&pkg), restored.excludes(&pkg));
    }

    // ── PackageFilterList Tests ───────────────────────────────────────────

    #[test]
    fn test_filter_list_excludes() {
        let mut list = PackageFilterList::new();

        let mut filter1 = PackageFilter::new();
        filter1.add_exclusion(None, Box::new(GlobRule::new("*.beta").unwrap()));
        list.add_filter(filter1);

        let pkg = make_package("foo.beta", "1.0.0");
        assert!(list.excludes(&pkg));
    }

    #[test]
    fn test_filter_list_filter_packages() {
        let mut list = PackageFilterList::new();

        let mut filter1 = PackageFilter::new();
        filter1.add_exclusion(None, Box::new(GlobRule::new("*.beta").unwrap()));
        list.add_filter(filter1);

        let pkgs = vec![
            make_package("foo", "1.0.0"),
            make_package("foo.beta", "1.0.0"),
        ];

        let filtered = list.filter_packages(pkgs);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "foo");
    }

    // ── Rule Parsing Tests ───────────────────────────────────────────────

    #[test]
    fn test_parse_glob_rule() {
        let rule = parse_rule("glob(*.beta)");
        assert!(rule.is_some());
        assert_eq!(rule.unwrap().rule_type(), "glob");
    }

    #[test]
    fn test_parse_regex_rule() {
        let rule = parse_rule("regex(.*\\.beta$)");
        assert!(rule.is_some());
        assert_eq!(rule.unwrap().rule_type(), "regex");
    }

    #[test]
    fn test_parse_range_rule() {
        let rule = parse_rule("range(foo<2.0)");
        assert!(rule.is_some());
        assert_eq!(rule.unwrap().rule_type(), "range");
    }

    #[test]
    fn test_parse_timestamp_rule() {
        let rule = parse_rule("timestamp(after:1000)");
        assert!(rule.is_some());
        assert_eq!(rule.unwrap().rule_type(), "timestamp");
    }

    #[test]
    fn test_parse_auto_detect_glob() {
        let rule = parse_rule("*.beta");
        assert!(rule.is_some());
        assert_eq!(rule.unwrap().rule_type(), "glob");
    }
}
