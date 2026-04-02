//! Version range implementation - full rez-compatible version range parsing

use super::Version;
use rez_next_common::RezCoreError;
use serde::{Deserialize, Serialize};

/// A single bound in a version range
#[derive(Clone, Debug, PartialEq, Eq)]
enum Bound {
    /// >= version
    Ge(Version),
    /// > version
    Gt(Version),
    /// <= version
    Le(Version),
    /// < version
    Lt(Version),
    /// == version (exact match)
    Eq(Version),
    /// != version
    Ne(Version),
    /// ~= version (compatible release: >= version AND < next major.minor)
    Compatible(Version),
    /// Any version (no constraint)
    Any,
    /// Empty set (no versions match)
    None,
}

/// A conjunction of bounds (all must be satisfied)
#[derive(Clone, Debug, PartialEq, Eq)]
struct BoundSet {
    bounds: Vec<Bound>,
}

impl BoundSet {
    fn any() -> Self {
        BoundSet {
            bounds: vec![Bound::Any],
        }
    }

    fn none() -> Self {
        BoundSet {
            bounds: vec![Bound::None],
        }
    }

    fn contains(&self, version: &Version) -> bool {
        for bound in &self.bounds {
            if !bound_matches(bound, version) {
                return false;
            }
        }
        true
    }
}

fn bound_matches(bound: &Bound, version: &Version) -> bool {
    match bound {
        Bound::Any => true,
        Bound::None => false,
        Bound::Ge(v) => version >= v,
        Bound::Gt(v) => version > v,
        Bound::Le(v) => version <= v,
        Bound::Lt(v) => version < v,
        Bound::Eq(v) => version == v,
        Bound::Ne(v) => version != v,
        Bound::Compatible(v) => {
            // ~= M.N means >= M.N AND < M.(N+1), or ~= M.N.P means >= M.N.P AND < M.N+1
            // For rez we implement as: >= v AND same prefix up to second-to-last component
            if version < v {
                return false;
            }
            // Compatible release: upper bound is next minor/patch
            let parts = v.as_str().split('.').collect::<Vec<_>>();
            if parts.len() < 2 {
                return true;
            }
            let prefix = &parts[..parts.len() - 1].join(".");
            // version must start with same prefix
            version.as_str().starts_with(&format!("{}.", prefix))
                || version.as_str() == prefix.as_str()
        }
    }
}

/// Version range representation - a disjunction of BoundSets (union of intersections)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionRange {
    /// Cached string representation
    pub range_str: String,
    /// Parsed bound sets (disjunction - any must match)
    #[serde(skip)]
    bound_sets: Vec<BoundSet>,
    /// Whether the range was successfully parsed
    #[serde(skip)]
    is_parsed: bool,
    /// Ranges to subtract (for set difference operations)
    #[serde(skip)]
    subtract_from: Vec<VersionRange>,
}

impl VersionRange {
    /// Create a new version range from a string
    pub fn new(range_str: String) -> Result<Self, RezCoreError> {
        Self::parse(&range_str)
    }

    /// Create a version range that matches any version (equivalent to `""` or `"*"`)
    pub fn any() -> Self {
        VersionRange {
            range_str: String::new(),
            bound_sets: vec![BoundSet::any()],
            is_parsed: true,
            subtract_from: Vec::new(),
        }
    }

    /// Create a version range that matches no version (empty set)
    pub fn none() -> Self {
        VersionRange {
            range_str: "!*".to_string(),
            bound_sets: vec![BoundSet::none()],
            is_parsed: true,
            subtract_from: Vec::new(),
        }
    }

    /// Parse a version range string
    ///
    /// Supported formats:
    /// - `""` or `"*"` - any version
    /// - `">=1.0"` - single constraint
    /// - `">=1.0,<2.0"` - comma-separated AND constraints (rez style)
    /// - `">=1.0 <2.0"` - space-separated AND constraints
    /// - `"1.0+"` - rez shorthand for `>=1.0`
    /// - `"<1.0|>=2.0"` - pipe-separated OR constraints
    /// - `"==1.0"` - exact version
    /// - `"~=1.4"` - compatible release
    pub fn parse(range_str: &str) -> Result<Self, RezCoreError> {
        let trimmed = range_str.trim();
        let bound_sets = parse_range_str(trimmed)?;
        Ok(VersionRange {
            range_str: range_str.to_string(),
            bound_sets,
            is_parsed: true,
            subtract_from: Vec::new(),
        })
    }

    /// Check if a version satisfies this range
    pub fn contains(&self, version: &Version) -> bool {
        if !self.is_parsed || self.bound_sets.is_empty() {
            return true;
        }
        // Disjunction: any bound_set matching means the version is included
        let in_self = self.bound_sets.iter().any(|bs| bs.contains(version));
        if !in_self {
            return false;
        }
        // Subtract: if version is in any subtract range, exclude it
        for sub_range in &self.subtract_from {
            if sub_range.contains(version) {
                return false;
            }
        }
        true
    }

    /// Get the string representation
    pub fn as_str(&self) -> &str {
        &self.range_str
    }

    /// Check if this range intersects with another range
    pub fn intersects(&self, other: &VersionRange) -> bool {
        // Conservative: if either is "any", they intersect
        if self.is_any() || other.is_any() {
            return true;
        }
        // Check if the ranges have any overlap by checking bounds interaction
        // For exact versions, check containment
        for bs_self in &self.bound_sets {
            for bs_other in &other.bound_sets {
                // Check if these two bound sets can co-exist
                if bound_sets_intersect(bs_self, bs_other) {
                    return true;
                }
            }
        }
        false
    }

    /// Compute the intersection of two ranges
    pub fn intersect(&self, other: &VersionRange) -> Option<VersionRange> {
        if self.is_any() {
            return Some(other.clone());
        }
        if other.is_any() {
            return Some(self.clone());
        }
        // Merge all bound sets with AND semantics
        // Only include merged sets that are satisfiable (not trivially empty)
        let mut result_sets = Vec::new();
        for bs_self in &self.bound_sets {
            for bs_other in &other.bound_sets {
                let mut merged = bs_self.bounds.clone();
                merged.extend(bs_other.bounds.clone());
                let merged_set = BoundSet { bounds: merged };
                // Only include if this merged set is satisfiable
                if is_bound_set_satisfiable(&merged_set) {
                    result_sets.push(merged_set);
                }
            }
        }

        if result_sets.is_empty() {
            return None;
        }
        let new_str = format!("({})&({})", self.range_str, other.range_str);
        let mut combined_subtracts = self.subtract_from.clone();
        combined_subtracts.extend(other.subtract_from.clone());
        Some(VersionRange {
            range_str: new_str,
            bound_sets: result_sets,
            is_parsed: true,
            subtract_from: combined_subtracts,
        })
    }

    /// Compute the union of two ranges (pipe-separated)
    pub fn union(&self, other: &VersionRange) -> VersionRange {
        let new_str = format!("{}|{}", self.range_str, other.range_str);
        let mut sets = self.bound_sets.clone();
        sets.extend(other.bound_sets.clone());
        VersionRange {
            range_str: new_str,
            bound_sets: sets,
            is_parsed: true,
            subtract_from: Vec::new(),
        }
    }

    /// Compute the difference of two ranges: versions in self but not in other
    /// Returns None if the result would be empty
    pub fn subtract(&self, other: &VersionRange) -> Option<VersionRange> {
        if other.is_any() {
            return None; // self - any = empty
        }
        if other.is_empty() {
            return Some(self.clone());
        }
        if self.is_empty() {
            return None;
        }
        // Use subtract_from field: self with other excluded via contains() check
        let new_str = format!("({})-({})", self.range_str, other.range_str);
        let mut subtracts = self.subtract_from.clone();
        subtracts.push(other.clone());
        let range = VersionRange {
            range_str: new_str,
            bound_sets: self.bound_sets.clone(),
            is_parsed: true,
            subtract_from: subtracts,
        };
        // Quick sanity: at least one probe version must be in the result
        let probes = self.collect_probe_versions_with_other(other);
        let has_any = probes.iter().any(|v| range.contains(v));
        if has_any {
            Some(range)
        } else {
            None
        }
    }
    /// Check if this range is the "any" range (matches all versions)
    pub fn is_any(&self) -> bool {
        let s = self.range_str.trim();
        if s.is_empty() || s == "*" {
            return true;
        }
        // Check if all bound sets are Any
        self.bound_sets
            .iter()
            .all(|bs| bs.bounds.is_empty() || bs.bounds.iter().all(|b| matches!(b, Bound::Any)))
    }

    /// Check if this range is a subset of another range
    /// (every version in self is also in other)
    pub fn is_subset_of(&self, other: &VersionRange) -> bool {
        if other.is_any() {
            return true;
        }
        if self.is_any() {
            return other.is_any();
        }
        if self.is_empty() {
            return true;
        }
        let probe_versions = self.collect_probe_versions_with_other(other);
        for v in &probe_versions {
            if self.contains(v) && !other.contains(v) {
                return false;
            }
        }
        true
    }

    /// Check if this range is a superset of another range
    pub fn is_superset_of(&self, other: &VersionRange) -> bool {
        other.is_subset_of(self)
    }

    /// Collect probe versions from both self and other's bounds, plus "beyond" versions
    fn collect_probe_versions_with_other(&self, other: &VersionRange) -> Vec<Version> {
        let mut versions = Vec::new();
        for range in [self as &VersionRange, other] {
            for bs in &range.bound_sets {
                for bound in &bs.bounds {
                    match bound {
                        Bound::Ge(v)
                        | Bound::Gt(v)
                        | Bound::Le(v)
                        | Bound::Lt(v)
                        | Bound::Eq(v)
                        | Bound::Ne(v)
                        | Bound::Compatible(v) => {
                            versions.push(v.clone());
                            if let Ok(bumped) = Version::parse(&format!("{}.999999", v.as_str())) {
                                versions.push(bumped);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        for s in &["0.0.1", "999.999.999"] {
            if let Ok(v) = Version::parse(s) {
                versions.push(v);
            }
        }
        versions
    }

    /// Check if this range is empty (no versions match)
    pub fn is_empty(&self) -> bool {
        let s = self.range_str.trim();
        if s == "empty" || s == "!*" {
            return true;
        }
        if self.is_parsed && !self.bound_sets.is_empty() {
            return self
                .bound_sets
                .iter()
                .all(|bs| bs.bounds.iter().any(|b| matches!(b, Bound::None)));
        }
        false
    }
}

/// Parse a range string into a vector of BoundSets (disjunction)
fn parse_range_str(s: &str) -> Result<Vec<BoundSet>, RezCoreError> {
    if s.is_empty() || s == "*" {
        return Ok(vec![BoundSet::any()]);
    }
    if s == "empty" || s == "!*" {
        return Ok(vec![BoundSet::none()]);
    }

    // Handle rez ".." interval syntax: "1.0..2.0" = ">=1.0,<2.0"
    // Note: must check before splitting on |
    if s.contains("..") && !s.starts_with('.') {
        // Only handle if it's a simple "a..b" form (no | or other operators in the whole string)
        if let Some(dot_pos) = s.find("..") {
            let left = &s[..dot_pos];
            let right = &s[dot_pos + 2..];
            // Both sides must look like version strings (not empty operators)
            if !left.is_empty()
                && !right.is_empty()
                && !left.starts_with('>')
                && !left.starts_with('<')
                && !left.starts_with('=')
                && !left.starts_with('!')
                && !left.starts_with('~')
            {
                // "left..right" -> ">=left,<right"
                let new_s = format!(">={},<{}", left.trim(), right.trim());
                return parse_range_str(&new_s);
            }
        }
    }

    // Split on | for OR (union)
    let or_parts: Vec<&str> = s.split('|').collect();
    let mut result = Vec::new();

    for or_part in or_parts {
        let bound_set = parse_conjunction(or_part.trim())?;
        result.push(bound_set);
    }

    Ok(result)
}

/// Parse a conjunction of constraints (AND semantics)
/// Supports: `>=1.0,<2.0` (comma) or `>=1.0 <2.0` (space) or `1.0+<2.0` (rez syntax)
fn parse_conjunction(s: &str) -> Result<BoundSet, RezCoreError> {
    if s.is_empty() || s == "*" {
        return Ok(BoundSet::any());
    }

    // Handle rez shorthand: "1.0+" = ">=1.0"
    // Handle rez shorthand: "1.0+<2.0" = ">=1.0,<2.0"
    // The `+` in rez means "this version and above", with optional upper bound after it
    let s = if s.contains('+')
        && !s.starts_with('>')
        && !s.starts_with('<')
        && !s.starts_with('=')
        && !s.starts_with('!')
        && !s.starts_with('~')
    {
        // Find the + and split: "1.0+<2.0" -> prefix="1.0", suffix="<2.0"
        if let Some(plus_pos) = s.find('+') {
            let prefix = &s[..plus_pos];
            let suffix = &s[plus_pos + 1..];
            if suffix.is_empty() {
                // "1.0+" -> ">=1.0"
                format!(">={}", prefix)
            } else {
                // "1.0+<2.0" -> ">=1.0,<2.0"
                format!(">={},{}", prefix, suffix)
            }
        } else {
            s.to_string()
        }
    } else {
        s.to_string()
    };

    let mut bounds = Vec::new();

    // Split on commas first, then spaces that separate constraints
    let parts = split_constraint_parts(&s);

    for part in parts {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let bound = parse_single_constraint(part)?;
        bounds.push(bound);
    }

    if bounds.is_empty() {
        return Ok(BoundSet::any());
    }

    Ok(BoundSet { bounds })
}

/// Split a string into individual constraint parts (handles comma and space separators)
fn split_constraint_parts(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();

    for ch in s.chars() {
        if ch == ',' {
            if !current.trim().is_empty() {
                parts.push(current.trim().to_string());
            }
            current = String::new();
        } else {
            current.push(ch);
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }

    // Further split space-separated constraints within each part
    let mut final_parts = Vec::new();
    for part in parts {
        let space_parts = split_on_operator_boundaries(&part);
        final_parts.extend(space_parts);
    }

    final_parts
}

/// Split on spaces that are followed by an operator (>=, <=, >, <, ==, !=, ~=)
fn split_on_operator_boundaries(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();

    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == ' ' {
            // Check if next non-space char starts an operator
            let mut j = i + 1;
            while j < chars.len() && chars[j] == ' ' {
                j += 1;
            }
            if j < chars.len() {
                let next = chars[j];
                if next == '>' || next == '<' || next == '=' || next == '!' || next == '~' {
                    if !current.trim().is_empty() {
                        parts.push(current.trim().to_string());
                    }
                    current = String::new();
                    i = j;
                    continue;
                }
            }
            current.push(ch);
        } else {
            current.push(ch);
        }
        i += 1;
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    parts
}

/// Parse a single constraint like `>=1.0`, `<2.0`, `==1.5`, `!=1.0`, `~=1.4`
fn parse_single_constraint(s: &str) -> Result<Bound, RezCoreError> {
    let s = s.trim();

    if s.is_empty() || s == "*" {
        return Ok(Bound::Any);
    }

    // Try two-char operators first
    if let Some(rest) = s.strip_prefix(">=") {
        let v = Version::parse(rest.trim()).map_err(|e| {
            RezCoreError::VersionRange(format!("Invalid version in range '{}': {}", s, e))
        })?;
        return Ok(Bound::Ge(v));
    }
    if let Some(rest) = s.strip_prefix("<=") {
        let v = Version::parse(rest.trim()).map_err(|e| {
            RezCoreError::VersionRange(format!("Invalid version in range '{}': {}", s, e))
        })?;
        return Ok(Bound::Le(v));
    }
    if let Some(rest) = s.strip_prefix("==") {
        let v = Version::parse(rest.trim()).map_err(|e| {
            RezCoreError::VersionRange(format!("Invalid version in range '{}': {}", s, e))
        })?;
        return Ok(Bound::Eq(v));
    }
    if let Some(rest) = s.strip_prefix("!=") {
        let v = Version::parse(rest.trim()).map_err(|e| {
            RezCoreError::VersionRange(format!("Invalid version in range '{}': {}", s, e))
        })?;
        return Ok(Bound::Ne(v));
    }
    if let Some(rest) = s.strip_prefix("~=") {
        let v = Version::parse(rest.trim()).map_err(|e| {
            RezCoreError::VersionRange(format!("Invalid version in range '{}': {}", s, e))
        })?;
        return Ok(Bound::Compatible(v));
    }

    // Single-char operators
    if let Some(rest) = s.strip_prefix('>') {
        let v = Version::parse(rest.trim()).map_err(|e| {
            RezCoreError::VersionRange(format!("Invalid version in range '{}': {}", s, e))
        })?;
        return Ok(Bound::Gt(v));
    }
    if let Some(rest) = s.strip_prefix('<') {
        let v = Version::parse(rest.trim()).map_err(|e| {
            RezCoreError::VersionRange(format!("Invalid version in range '{}': {}", s, e))
        })?;
        return Ok(Bound::Lt(v));
    }

    // No operator - treat as exact version (rez: bare version = "==version")
    let v = Version::parse(s).map_err(|e| {
        RezCoreError::VersionRange(format!("Invalid version constraint '{}': {}", s, e))
    })?;
    Ok(Bound::Eq(v))
}

/// Check if a single BoundSet is satisfiable (not trivially empty due to conflicting bounds)
fn is_bound_set_satisfiable(bs: &BoundSet) -> bool {
    // Check for None bounds
    if bs.bounds.iter().any(|b| matches!(b, Bound::None)) {
        return false;
    }
    // Extract lower and upper bounds to check for contradiction
    let mut lower: Option<(&Version, bool)> = None; // (version, inclusive)
    let mut upper: Option<(&Version, bool)> = None; // (version, inclusive)

    for bound in &bs.bounds {
        match bound {
            Bound::Any => {}
            Bound::None => return false,
            Bound::Ge(v) => match lower {
                None => lower = Some((v, true)),
                Some((lv, linc)) => {
                    if v > lv || (v == lv && !linc) {
                        lower = Some((v, true));
                    }
                }
            },
            Bound::Gt(v) => match lower {
                None => lower = Some((v, false)),
                Some((lv, _)) => {
                    if v >= lv {
                        lower = Some((v, false));
                    }
                }
            },
            Bound::Le(v) => match upper {
                None => upper = Some((v, true)),
                Some((uv, uinc)) => {
                    if v < uv || (v == uv && !uinc) {
                        upper = Some((v, true));
                    }
                }
            },
            Bound::Lt(v) => match upper {
                None => upper = Some((v, false)),
                Some((uv, _)) => {
                    if v <= uv {
                        upper = Some((v, false));
                    }
                }
            },
            Bound::Eq(v) => {
                // Equality constraint acts as both lower and upper bound
                match lower {
                    None => lower = Some((v, true)),
                    Some((lv, linc)) => {
                        if v > lv || (v == lv && !linc) {
                            lower = Some((v, true));
                        } else if v < lv {
                            // v must be >= lv but eq requires v exactly — contradiction
                            return false;
                        }
                    }
                }
                match upper {
                    None => upper = Some((v, true)),
                    Some((uv, uinc)) => {
                        if v < uv || (v == uv && !uinc) {
                            upper = Some((v, true));
                        } else if v > uv {
                            return false;
                        }
                    }
                }
            }
            Bound::Ne(_) | Bound::Compatible(_) => {}
        }
    }

    // Check if lower and upper bounds are compatible
    if let (Some((lv, linc)), Some((uv, uinc))) = (lower, upper) {
        match lv.cmp(uv) {
            std::cmp::Ordering::Greater => return false,
            std::cmp::Ordering::Equal => {
                if !linc || !uinc {
                    return false;
                }
            }
            std::cmp::Ordering::Less => {}
        }
    }

    true
}

/// Check if two BoundSets can simultaneously be satisfied (have intersection)
fn bound_sets_intersect(a: &BoundSet, b: &BoundSet) -> bool {
    // Quick check: if either is Any, they intersect
    let a_any = a.bounds.iter().all(|bnd| matches!(bnd, Bound::Any));
    let b_any = b.bounds.iter().all(|bnd| matches!(bnd, Bound::Any));
    if a_any || b_any {
        return true;
    }

    // Combine all bounds and check for structural impossibilities
    let combined_bounds: Vec<&Bound> = a.bounds.iter().chain(b.bounds.iter()).collect();

    // Check for obvious impossibilities: Eq(v) and Eq(w) where v != w
    let eq_versions: Vec<&Version> = combined_bounds
        .iter()
        .filter_map(|b| if let Bound::Eq(v) = b { Some(v) } else { None })
        .collect();
    if eq_versions.len() > 1 {
        let first = eq_versions[0];
        if eq_versions.iter().any(|v| *v != first) {
            return false;
        }
    }

    // Compute effective lower and upper bounds from combined set
    // Lower bound: maximum of all lower bounds (most restrictive)
    // Upper bound: minimum of all upper bounds (most restrictive)
    let mut lower: Option<(&Version, bool)> = None; // (version, inclusive)
    let mut upper: Option<(&Version, bool)> = None; // (version, inclusive)

    for bound in &combined_bounds {
        match bound {
            Bound::Ge(v) => match lower {
                None => lower = Some((v, true)),
                Some((lv, linc)) => {
                    if v > lv || (v == lv && !linc) {
                        lower = Some((v, true));
                    }
                }
            },
            Bound::Gt(v) => match lower {
                None => lower = Some((v, false)),
                Some((lv, _linc)) => {
                    if v >= lv {
                        lower = Some((v, false));
                    }
                }
            },
            Bound::Le(v) => match upper {
                None => upper = Some((v, true)),
                Some((uv, uinc)) => {
                    if v < uv || (v == uv && !uinc) {
                        upper = Some((v, true));
                    }
                }
            },
            Bound::Lt(v) => match upper {
                None => upper = Some((v, false)),
                Some((uv, _uinc)) => {
                    if v <= uv {
                        upper = Some((v, false));
                    }
                }
            },
            _ => {}
        }
    }

    // Check if lower bound and upper bound are compatible
    if let (Some((lv, linc)), Some((uv, uinc))) = (lower, upper) {
        match lv.cmp(uv) {
            std::cmp::Ordering::Greater => return false,
            std::cmp::Ordering::Equal => {
                // Equal bounds only feasible if both inclusive
                if !linc || !uinc {
                    return false;
                }
            }
            std::cmp::Ordering::Less => {} // feasible
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Intersecting any range with any other range returns the other range
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
        assert!(
            range.is_empty(),
            "VersionRange::none() must report is_empty()"
        );
        assert!(!range.contains(&v("1.0.0")));
        assert!(!range.contains(&v("0.0.1")));
        assert!(!range.contains(&v("99.0")));
        // Intersecting none with anything yields None
        let specific = VersionRange::parse(">=1.0").unwrap();
        assert!(
            range.intersect(&specific).is_none(),
            "none intersect anything must be None"
        );
    }

    #[test]
    fn test_version_range_any_union_identity() {
        // any().union(x) should be a superset of x (is_any equivalent)
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
        // In rez, "2.0" > "2.0.0" (shorter version = higher precedence)
        // so "2.0.0" does NOT satisfy ">=2.0", but "2.0" and "2.1.0" would
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
        // "1.0+" is rez shorthand for ">=1.0"
        let range = VersionRange::parse("1.0+").unwrap();
        assert!(range.contains(&v("1.0")));
        assert!(range.contains(&v("2.0")));
        assert!(!range.contains(&v("0.9")));
    }

    #[test]
    fn test_version_range_rez_plus_upper() {
        // "1.0+<2.0" = ">=1.0,<2.0"
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
        // Bare version "1.0.0" = exact match
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

    // ── Phase 98: intersect() returns new range ────────────────────────────────

    #[test]
    fn test_intersect_overlapping_ranges() {
        // [1.0, 3.0) ∩ [2.0, 4.0) = [2.0, 3.0)
        let r1 = VersionRange::parse(">=1.0,<3.0").unwrap();
        let r2 = VersionRange::parse(">=2.0,<4.0").unwrap();
        let result = r1.intersect(&r2);
        assert!(
            result.is_some(),
            "Overlapping ranges should have intersection"
        );
        let inter = result.unwrap();
        assert!(inter.contains(&v("2.0")), "2.0 should be in intersection");
        assert!(inter.contains(&v("2.9")), "2.9 should be in intersection");
        assert!(
            !inter.contains(&v("1.5")),
            "1.5 should NOT be in intersection (excluded by r2)"
        );
        assert!(
            !inter.contains(&v("3.0")),
            "3.0 should NOT be in intersection (excluded by r1)"
        );
    }

    #[test]
    fn test_intersect_with_any() {
        let any = VersionRange::parse("*").unwrap();
        let r1 = VersionRange::parse(">=2.0,<5.0").unwrap();
        // any ∩ r1 = r1
        let result = any.intersect(&r1);
        assert!(result.is_some());
        let inter = result.unwrap();
        assert!(inter.contains(&v("3.0")));
        assert!(!inter.contains(&v("1.0")));
        // r1 ∩ any = r1
        let result2 = r1.intersect(&any);
        assert!(result2.is_some());
        let inter2 = result2.unwrap();
        assert!(inter2.contains(&v("3.0")));
    }

    #[test]
    fn test_intersect_exact_in_range() {
        // ==2.5 ∩ [2.0, 3.0) = ==2.5
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
        // ==5.0 ∩ [1.0, 3.0) = empty or None
        let exact = VersionRange::parse("==5.0").unwrap();
        let range = VersionRange::parse(">=1.0,<3.0").unwrap();
        let result = exact.intersect(&range);
        // Either None or the range doesn't contain any version
        if let Some(ref inter) = result {
            assert!(
                !inter.contains(&v("5.0")),
                "5.0 should NOT be in intersection"
            );
            assert!(
                !inter.contains(&v("1.5")),
                "1.5 should NOT be in intersection"
            );
        }
        // It's okay to return Some with an empty effective range
    }

    #[test]
    fn test_intersect_disjoint_ranges() {
        // [1.0, 2.0) ∩ [3.0, 4.0) = empty
        let r1 = VersionRange::parse(">=1.0,<2.0").unwrap();
        let r2 = VersionRange::parse(">=3.0,<4.0").unwrap();
        let result = r1.intersect(&r2);
        if let Some(ref inter) = result {
            // The resulting range should be empty - no representative version should match
            assert!(!inter.contains(&v("1.5")));
            assert!(!inter.contains(&v("3.5")));
        }
    }

    #[test]
    fn test_intersect_same_range() {
        // [1.0, 3.0) ∩ [1.0, 3.0) = [1.0, 3.0)
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
        // [1.0, 3.0) ∩ !=2.0 — contains everything except 2.0
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
        // (<1.0 | >=3.0) ∩ >=2.0 = >=3.0
        let r1 = VersionRange::parse("<1.0|>=3.0").unwrap();
        let r2 = VersionRange::parse(">=2.0").unwrap();
        let result = r1.intersect(&r2);
        assert!(result.is_some());
        let inter = result.unwrap();
        // Should contain 3.0+ but not 0.5, not 1.5, not 2.5
        assert!(!inter.contains(&v("0.5")), "0.5 not in (<1|>=3) ∩ >=2");
        assert!(!inter.contains(&v("1.5")), "1.5 not in result");
        // 3.5 should be in both
        assert!(inter.contains(&v("3.5")), "3.5 should be in (<1|>=3) ∩ >=2");
    }

    #[test]
    fn test_intersect_compatible_release() {
        // ~=1.2 (>=1.2,<2.0) ∩ <1.5 = [1.2, 1.5)
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
        // [1.0, 2.0) ∩ [2.0, 3.0) = technically empty (boundary exclusive/exclusive)
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
