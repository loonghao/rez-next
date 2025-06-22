//! Version range implementation

use super::Version;
use pyo3::prelude::*;
use pyo3::types::PyType;
use rez_core_common::RezCoreError;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Version range representation
#[pyclass]
#[derive(Clone, Debug)]
pub struct VersionRange {
    /// Cached string representation
    #[pyo3(get)]
    range_str: String,
    /// Simple bounds representation for basic functionality
    lower_version: Option<Version>,
    lower_inclusive: bool,
    upper_version: Option<Version>,
    upper_inclusive: bool,
}

impl Serialize for VersionRange {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as string representation for simplicity
        self.range_str.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for VersionRange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}

#[pymethods]
impl VersionRange {
    #[new]
    pub fn new(range_str: Option<&str>) -> PyResult<Self> {
        let range_str = range_str.unwrap_or("");
        Self::parse(range_str)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.range_str
    }

    fn __str__(&self) -> String {
        self.range_str.clone()
    }

    fn __repr__(&self) -> String {
        format!("VersionRange('{}')", self.range_str)
    }

    /// Check if a version is contained in this range
    pub fn contains_version(&self, version: &Version) -> bool {
        // Check lower bound
        if let Some(ref lower) = self.lower_version {
            let cmp = version.cmp(lower);
            if self.lower_inclusive {
                if cmp == Ordering::Less {
                    return false;
                }
            } else {
                if cmp != Ordering::Greater {
                    return false;
                }
            }
        }

        // Check upper bound
        if let Some(ref upper) = self.upper_version {
            let cmp = version.cmp(upper);
            if self.upper_inclusive {
                if cmp == Ordering::Greater {
                    return false;
                }
            } else {
                if cmp != Ordering::Less {
                    return false;
                }
            }
        }

        true
    }

    /// Alias for contains_version for Python compatibility
    pub fn contains(&self, version: &Version) -> bool {
        self.contains_version(version)
    }

    /// Check if this range intersects with another range
    pub fn intersects(&self, other: &VersionRange) -> bool {
        // Simple intersection check
        // If either range is "any", they intersect
        if self.is_any() || other.is_any() {
            return true;
        }

        // Check if ranges overlap
        match (&self.upper_version, &other.lower_version) {
            (Some(self_upper), Some(other_lower)) => {
                let cmp = self_upper.cmp(other_lower);
                if cmp == Ordering::Less {
                    return false;
                }
                if cmp == Ordering::Equal && !(self.upper_inclusive && other.lower_inclusive) {
                    return false;
                }
            }
            _ => {}
        }

        match (&self.lower_version, &other.upper_version) {
            (Some(self_lower), Some(other_upper)) => {
                let cmp = self_lower.cmp(other_upper);
                if cmp == Ordering::Greater {
                    return false;
                }
                if cmp == Ordering::Equal && !(self.lower_inclusive && other.upper_inclusive) {
                    return false;
                }
            }
            _ => {}
        }

        true
    }

    /// Compute the intersection of two ranges
    pub fn intersect(&self, other: &VersionRange) -> Option<VersionRange> {
        if !self.intersects(other) {
            return None;
        }

        // Compute the actual intersection bounds
        let (lower_version, lower_inclusive) = match (&self.lower_version, &other.lower_version) {
            (None, None) => (None, true),
            (Some(v), None) | (None, Some(v)) => (Some(v.clone()), true),
            (Some(v1), Some(v2)) => match v1.cmp(v2) {
                Ordering::Greater => (Some(v1.clone()), self.lower_inclusive),
                Ordering::Less => (Some(v2.clone()), other.lower_inclusive),
                Ordering::Equal => (
                    Some(v1.clone()),
                    self.lower_inclusive && other.lower_inclusive,
                ),
            },
        };

        let (upper_version, upper_inclusive) = match (&self.upper_version, &other.upper_version) {
            (None, None) => (None, true),
            (Some(v), None) | (None, Some(v)) => (Some(v.clone()), true),
            (Some(v1), Some(v2)) => match v1.cmp(v2) {
                Ordering::Less => (Some(v1.clone()), self.upper_inclusive),
                Ordering::Greater => (Some(v2.clone()), other.upper_inclusive),
                Ordering::Equal => (
                    Some(v1.clone()),
                    self.upper_inclusive && other.upper_inclusive,
                ),
            },
        };

        let range_str = Self::build_range_string(
            &lower_version,
            lower_inclusive,
            &upper_version,
            upper_inclusive,
        );

        Some(VersionRange {
            range_str,
            lower_version,
            lower_inclusive,
            upper_version,
            upper_inclusive,
        })
    }

    /// Compute the union of two ranges (if they overlap or are adjacent)
    pub fn union(&self, other: &VersionRange) -> Option<VersionRange> {
        // For simplicity, only handle cases where ranges overlap or are adjacent
        if !self.intersects(other) && !self.is_adjacent(other) {
            return None;
        }

        // Compute the union bounds
        let (lower_version, lower_inclusive) = match (&self.lower_version, &other.lower_version) {
            (None, _) | (_, None) => (None, true), // Any unbounded lower means unbounded result
            (Some(v1), Some(v2)) => match v1.cmp(v2) {
                Ordering::Less => (Some(v1.clone()), self.lower_inclusive),
                Ordering::Greater => (Some(v2.clone()), other.lower_inclusive),
                Ordering::Equal => (
                    Some(v1.clone()),
                    self.lower_inclusive || other.lower_inclusive,
                ),
            },
        };

        let (upper_version, upper_inclusive) = match (&self.upper_version, &other.upper_version) {
            (None, _) | (_, None) => (None, true), // Any unbounded upper means unbounded result
            (Some(v1), Some(v2)) => match v1.cmp(v2) {
                Ordering::Greater => (Some(v1.clone()), self.upper_inclusive),
                Ordering::Less => (Some(v2.clone()), other.upper_inclusive),
                Ordering::Equal => (
                    Some(v1.clone()),
                    self.upper_inclusive || other.upper_inclusive,
                ),
            },
        };

        let range_str = Self::build_range_string(
            &lower_version,
            lower_inclusive,
            &upper_version,
            upper_inclusive,
        );

        Some(VersionRange {
            range_str,
            lower_version,
            lower_inclusive,
            upper_version,
            upper_inclusive,
        })
    }

    /// Check if two ranges are adjacent (touching but not overlapping)
    pub fn is_adjacent(&self, other: &VersionRange) -> bool {
        // Check if self.upper touches other.lower
        if let (Some(self_upper), Some(other_lower)) = (&self.upper_version, &other.lower_version) {
            if self_upper.cmp(other_lower) == Ordering::Equal {
                return self.upper_inclusive != other.lower_inclusive;
            }
        }

        // Check if other.upper touches self.lower
        if let (Some(other_upper), Some(self_lower)) = (&other.upper_version, &self.lower_version) {
            if other_upper.cmp(self_lower) == Ordering::Equal {
                return other.upper_inclusive != self.lower_inclusive;
            }
        }

        false
    }

    /// Alias for intersect for Python compatibility
    pub fn intersection(&self, other: &VersionRange) -> Option<VersionRange> {
        self.intersect(other)
    }

    /// Check if this range is the "any" range (matches all versions)
    pub fn is_any(&self) -> bool {
        self.lower_version.is_none() && self.upper_version.is_none()
    }

    /// Create a range from a single version with an operator
    #[classmethod]
    pub fn from_version(
        _cls: &Bound<'_, PyType>,
        version: &Version,
        op: Option<&str>,
    ) -> PyResult<Self> {
        let (lower_version, lower_inclusive, upper_version, upper_inclusive) = match op {
            None => {
                // No operator means "version or greater, but less than next version"
                (Some(version.clone()), true, Some(version.next()?), false)
            }
            Some("==") | Some("eq") => {
                // Exact version
                (Some(version.clone()), true, Some(version.clone()), true)
            }
            Some(">") | Some("gt") => {
                // Greater than
                (Some(version.clone()), false, None, true)
            }
            Some(">=") | Some("gte") => {
                // Greater than or equal
                (Some(version.clone()), true, None, true)
            }
            Some("<") | Some("lt") => {
                // Less than
                (None, true, Some(version.clone()), false)
            }
            Some("<=") | Some("lte") => {
                // Less than or equal
                (None, true, Some(version.clone()), true)
            }
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Unknown bound operation '{}'",
                    op.unwrap_or("")
                )));
            }
        };

        let range_str = Self::build_range_string(
            &lower_version,
            lower_inclusive,
            &upper_version,
            upper_inclusive,
        );

        Ok(VersionRange {
            range_str,
            lower_version,
            lower_inclusive,
            upper_version,
            upper_inclusive,
        })
    }

    /// Create a range spanning from lower_version to upper_version
    #[classmethod]
    pub fn as_span(
        _cls: &Bound<'_, PyType>,
        lower_version: Option<&Version>,
        upper_version: Option<&Version>,
        lower_inclusive: Option<bool>,
        upper_inclusive: Option<bool>,
    ) -> PyResult<Self> {
        let lower_inclusive = lower_inclusive.unwrap_or(true);
        let upper_inclusive = upper_inclusive.unwrap_or(true);
        let lower_version = lower_version.cloned();
        let upper_version = upper_version.cloned();

        let range_str = Self::build_range_string(
            &lower_version,
            lower_inclusive,
            &upper_version,
            upper_inclusive,
        );

        Ok(VersionRange {
            range_str,
            lower_version,
            lower_inclusive,
            upper_version,
            upper_inclusive,
        })
    }

    /// Create a range from a list of exact versions
    #[classmethod]
    pub fn from_versions(_cls: &Bound<'_, PyType>, versions: &Bound<'_, PyAny>) -> PyResult<Self> {
        let versions: Vec<Version> = versions.extract()?;
        if versions.is_empty() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Cannot create range from empty version list",
            ));
        }

        // For simplicity, create a range that spans from min to max version
        let mut sorted_versions = versions;
        sorted_versions.sort();

        let lower_version = Some(sorted_versions[0].clone());
        let upper_version = Some(sorted_versions[sorted_versions.len() - 1].clone());
        let range_str = Self::build_range_string(&lower_version, true, &upper_version, true);

        Ok(VersionRange {
            range_str,
            lower_version,
            lower_inclusive: true,
            upper_version,
            upper_inclusive: true,
        })
    }

    /// Return exact version ranges as Version objects, or None if there are no exact versions
    pub fn to_versions(&self) -> Option<Vec<Version>> {
        // Only return versions if this is an exact version range
        if let (Some(ref lower), Some(ref upper)) = (&self.lower_version, &self.upper_version) {
            if lower == upper && self.lower_inclusive && self.upper_inclusive {
                return Some(vec![lower.clone()]);
            }
        }
        None
    }

    /// Parse a version range string (static method)
    #[staticmethod]
    pub fn parse_static(s: &str) -> PyResult<Self> {
        Self::parse(s).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
}

impl VersionRange {
    /// Parse a version range string
    pub fn parse(s: &str) -> Result<Self, RezCoreError> {
        if s.is_empty() {
            // Empty string is the "any" range
            return Ok(Self {
                range_str: "".to_string(),
                lower_version: None,
                lower_inclusive: true,
                upper_version: None,
                upper_inclusive: true,
            });
        }

        // Check for compound ranges (comma-separated conditions)
        if s.contains(',') {
            return Self::parse_compound_range(s);
        }

        // Check for tilde range (~1.2.0)
        if s.starts_with('~') {
            return Self::parse_tilde_range(&s[1..]);
        }

        // Check for caret range (^1.0.0)
        if s.starts_with('^') {
            return Self::parse_caret_range(&s[1..]);
        }

        // For single conditions, use the single condition parser
        Self::parse_single_condition(s)
    }

    /// Build a string representation from bounds
    fn build_range_string(
        lower_version: &Option<Version>,
        lower_inclusive: bool,
        upper_version: &Option<Version>,
        upper_inclusive: bool,
    ) -> String {
        match (lower_version, upper_version) {
            (None, None) => "".to_string(),
            (Some(lower), None) => {
                if lower_inclusive {
                    format!("{}+", lower.as_str())
                } else {
                    format!(">{}", lower.as_str())
                }
            }
            (None, Some(upper)) => {
                if upper_inclusive {
                    format!("<={}", upper.as_str())
                } else {
                    format!("<{}", upper.as_str())
                }
            }
            (Some(lower), Some(upper)) => {
                if lower == upper && lower_inclusive && upper_inclusive {
                    format!("=={}", lower.as_str())
                } else {
                    format!("{}..{}", lower.as_str(), upper.as_str())
                }
            }
        }
    }

    /// Parse compound range like ">=1.0.0,<2.0.0"
    fn parse_compound_range(s: &str) -> Result<Self, RezCoreError> {
        let conditions: Vec<&str> = s.split(',').map(|s| s.trim()).collect();

        if conditions.is_empty() {
            return Err(RezCoreError::VersionParse(
                "Empty compound range".to_string(),
            ));
        }

        let mut lower_version: Option<Version> = None;
        let mut lower_inclusive = true;
        let mut upper_version: Option<Version> = None;
        let mut upper_inclusive = true;

        for condition in conditions {
            if condition.is_empty() {
                continue;
            }

            // Parse each condition and merge bounds
            let single_range = Self::parse_single_condition(condition)?;

            // Merge lower bounds (take the more restrictive one)
            if let Some(ref new_lower) = single_range.lower_version {
                match &lower_version {
                    None => {
                        lower_version = Some(new_lower.clone());
                        lower_inclusive = single_range.lower_inclusive;
                    }
                    Some(existing_lower) => {
                        match new_lower.cmp(existing_lower) {
                            std::cmp::Ordering::Greater => {
                                lower_version = Some(new_lower.clone());
                                lower_inclusive = single_range.lower_inclusive;
                            }
                            std::cmp::Ordering::Equal => {
                                // If versions are equal, use the more restrictive inclusivity
                                lower_inclusive = lower_inclusive && single_range.lower_inclusive;
                            }
                            std::cmp::Ordering::Less => {
                                // Keep existing lower bound
                            }
                        }
                    }
                }
            }

            // Merge upper bounds (take the more restrictive one)
            if let Some(ref new_upper) = single_range.upper_version {
                match &upper_version {
                    None => {
                        upper_version = Some(new_upper.clone());
                        upper_inclusive = single_range.upper_inclusive;
                    }
                    Some(existing_upper) => {
                        match new_upper.cmp(existing_upper) {
                            std::cmp::Ordering::Less => {
                                upper_version = Some(new_upper.clone());
                                upper_inclusive = single_range.upper_inclusive;
                            }
                            std::cmp::Ordering::Equal => {
                                // If versions are equal, use the more restrictive inclusivity
                                upper_inclusive = upper_inclusive && single_range.upper_inclusive;
                            }
                            std::cmp::Ordering::Greater => {
                                // Keep existing upper bound
                            }
                        }
                    }
                }
            }
        }

        // Keep the original string representation for compound ranges
        Ok(VersionRange {
            range_str: s.to_string(),
            lower_version,
            lower_inclusive,
            upper_version,
            upper_inclusive,
        })
    }

    /// Parse a single condition (without comma)
    fn parse_single_condition(s: &str) -> Result<Self, RezCoreError> {
        let s = s.trim();

        if s.starts_with(">=") {
            let version_str = &s[2..];
            let version = Version::parse(version_str)?;
            return Ok(Self {
                range_str: s.to_string(),
                lower_version: Some(version),
                lower_inclusive: true,
                upper_version: None,
                upper_inclusive: true,
            });
        }

        if s.starts_with('>') {
            let version_str = &s[1..];
            let version = Version::parse(version_str)?;
            return Ok(Self {
                range_str: s.to_string(),
                lower_version: Some(version),
                lower_inclusive: false,
                upper_version: None,
                upper_inclusive: true,
            });
        }

        if s.starts_with("<=") {
            let version_str = &s[2..];
            let version = Version::parse(version_str)?;
            return Ok(Self {
                range_str: s.to_string(),
                lower_version: None,
                lower_inclusive: true,
                upper_version: Some(version),
                upper_inclusive: true,
            });
        }

        if s.starts_with('<') {
            let version_str = &s[1..];
            let version = Version::parse(version_str)?;
            return Ok(Self {
                range_str: s.to_string(),
                lower_version: None,
                lower_inclusive: true,
                upper_version: Some(version),
                upper_inclusive: false,
            });
        }

        if s.starts_with("==") {
            let version_str = &s[2..];
            let version = Version::parse(version_str)?;
            return Ok(Self {
                range_str: s.to_string(),
                lower_version: Some(version.clone()),
                lower_inclusive: true,
                upper_version: Some(version),
                upper_inclusive: true,
            });
        }

        if s.ends_with('+') {
            let version_str = &s[..s.len() - 1];
            let version = Version::parse(version_str)?;
            return Ok(Self {
                range_str: s.to_string(),
                lower_version: Some(version),
                lower_inclusive: true,
                upper_version: None,
                upper_inclusive: true,
            });
        }

        // Try to parse as simple version
        if let Ok(version) = Version::parse(s) {
            let next_version = version.next().unwrap_or_else(|_| Version::inf());
            return Ok(Self {
                range_str: s.to_string(),
                lower_version: Some(version),
                lower_inclusive: true,
                upper_version: Some(next_version),
                upper_inclusive: false,
            });
        }

        Err(RezCoreError::VersionParse(format!(
            "Cannot parse version range condition: {}",
            s
        )))
    }

    /// Parse tilde range like "~1.2.0" (allows patch-level changes)
    fn parse_tilde_range(version_str: &str) -> Result<Self, RezCoreError> {
        let base_version = Version::parse(version_str)?;

        // For ~1.2.3, allow 1.2.x but not 1.3.0
        // This is a simplified implementation - in practice, rez might have different semantics
        let upper_version = base_version.next().unwrap_or_else(|_| Version::inf());

        Ok(Self {
            range_str: format!("~{}", version_str),
            lower_version: Some(base_version),
            lower_inclusive: true,
            upper_version: Some(upper_version),
            upper_inclusive: false,
        })
    }

    /// Parse caret range like "^1.0.0" (allows compatible changes)
    fn parse_caret_range(version_str: &str) -> Result<Self, RezCoreError> {
        let base_version = Version::parse(version_str)?;

        // For ^1.2.3, allow 1.x.x but not 2.0.0
        // This is a simplified implementation - in practice, rez might have different semantics
        let upper_version = base_version.next().unwrap_or_else(|_| Version::inf());

        Ok(Self {
            range_str: format!("^{}", version_str),
            lower_version: Some(base_version),
            lower_inclusive: true,
            upper_version: Some(upper_version),
            upper_inclusive: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_range_parsing() {
        // Test empty range (any)
        let range = VersionRange::parse("").unwrap();
        assert!(range.is_any());

        // Test exact version
        let range = VersionRange::parse("==1.0.0").unwrap();
        assert_eq!(range.range_str, "==1.0.0");

        // Test greater than or equal
        let range = VersionRange::parse(">=1.0.0").unwrap();
        assert_eq!(range.range_str, ">=1.0.0");

        // Test plus notation
        let range = VersionRange::parse("1.0.0+").unwrap();
        assert_eq!(range.range_str, "1.0.0+");

        // Test simple version
        let range = VersionRange::parse("1.0.0").unwrap();
        assert_eq!(range.range_str, "1.0.0");
    }

    #[test]
    fn test_version_range_intersect() {
        let range1 = VersionRange::parse(">=1.0.0").unwrap();
        let range2 = VersionRange::parse("<=2.0.0").unwrap();

        let intersection = range1.intersect(&range2).unwrap();
        assert!(intersection.contains_version(&Version::parse("1.5.0").unwrap()));
        assert!(!intersection.contains_version(&Version::parse("0.5.0").unwrap()));
        assert!(!intersection.contains_version(&Version::parse("2.5.0").unwrap()));
    }

    #[test]
    fn test_version_range_union() {
        let range1 = VersionRange::parse(">=1.0.0").unwrap();
        let range2 = VersionRange::parse("<=2.0.0").unwrap();

        // These ranges overlap, so union should work
        let union = range1.union(&range2);
        assert!(union.is_some());

        let union = union.unwrap();
        assert!(union.contains_version(&Version::parse("0.5.0").unwrap()));
        assert!(union.contains_version(&Version::parse("1.5.0").unwrap()));
        assert!(union.contains_version(&Version::parse("2.5.0").unwrap()));
    }

    #[test]
    fn test_version_range_intersects() {
        let range1 = VersionRange::parse(">=1.0.0").unwrap();
        let range2 = VersionRange::parse("<=2.0.0").unwrap();
        let range3 = VersionRange::parse(">=3.0.0").unwrap();

        assert!(range1.intersects(&range2));
        assert!(!range1.intersects(&range3));
    }

    #[test]
    fn test_version_range_contains_compound() {
        let range = VersionRange::parse(">=1.0.0,<=2.0.0").unwrap();

        assert!(range.contains_version(&Version::parse("1.0.0").unwrap()));
        assert!(range.contains_version(&Version::parse("1.5.0").unwrap()));
        assert!(range.contains_version(&Version::parse("2.0.0").unwrap()));
        assert!(!range.contains_version(&Version::parse("0.5.0").unwrap()));
        assert!(!range.contains_version(&Version::parse("2.5.0").unwrap()));
    }

    #[test]
    fn test_version_range_contains_simple() {
        let range = VersionRange::parse(">=1.0.0").unwrap();
        let version1 = Version::parse("1.0.0").unwrap();
        let version2 = Version::parse("0.9.0").unwrap();
        let version3 = Version::parse("1.1.0").unwrap();

        assert!(range.contains_version(&version1));
        assert!(!range.contains_version(&version2));
        assert!(range.contains_version(&version3));
    }

    #[test]
    fn test_compound_range_parsing() {
        // Test compound range
        let range = VersionRange::parse(">=1.0.0,<2.0.0").unwrap();
        assert_eq!(range.range_str, ">=1.0.0,<2.0.0");

        let version1 = Version::parse("1.0.0").unwrap();
        let version2 = Version::parse("1.5.0").unwrap();
        let version3 = Version::parse("2.0.0").unwrap();
        let version4 = Version::parse("0.9.0").unwrap();

        assert!(range.contains_version(&version1));
        assert!(range.contains_version(&version2));
        assert!(!range.contains_version(&version3));
        assert!(!range.contains_version(&version4));
    }

    #[test]
    fn test_tilde_range_parsing() {
        let range = VersionRange::parse("~1.2.0").unwrap();
        assert_eq!(range.range_str, "~1.2.0");
    }

    #[test]
    fn test_caret_range_parsing() {
        let range = VersionRange::parse("^1.0.0").unwrap();
        assert_eq!(range.range_str, "^1.0.0");
    }
}
