//! Version range implementation - full rez-compatible version range parsing

mod parser;
mod satisfiability;
mod types;

use super::Version;
use rez_next_common::RezCoreError;
use serde::{Deserialize, Serialize};
use types::BoundSet;

use parser::parse_range_str;
use satisfiability::{bound_sets_intersect, is_bound_set_satisfiable};
use types::Bound;

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
