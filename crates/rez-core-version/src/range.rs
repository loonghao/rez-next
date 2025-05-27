//! Version range implementation

use super::Version;
use rez_core_common::RezCoreError;
use pyo3::prelude::*;
use pyo3::types::PyType;
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

        // For simplicity, return a basic intersection
        let range_str = format!("{}|{}", self.range_str, other.range_str);
        Some(VersionRange {
            range_str,
            lower_version: self.lower_version.clone(),
            lower_inclusive: self.lower_inclusive,
            upper_version: self.upper_version.clone(),
            upper_inclusive: self.upper_inclusive,
        })
    }

    /// Check if this range is the "any" range (matches all versions)
    pub fn is_any(&self) -> bool {
        self.lower_version.is_none() && self.upper_version.is_none()
    }

    /// Create a range from a single version with an operator
    #[classmethod]
    pub fn from_version(_cls: &Bound<'_, PyType>, version: &Version, op: Option<&str>) -> PyResult<Self> {
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
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    format!("Unknown bound operation '{}'", op.unwrap_or(""))
                ));
            }
        };

        let range_str = Self::build_range_string(&lower_version, lower_inclusive, &upper_version, upper_inclusive);

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

        let range_str = Self::build_range_string(&lower_version, lower_inclusive, &upper_version, upper_inclusive);

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
                "Cannot create range from empty version list"
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

        // Simple parsing for basic patterns
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
            "Cannot parse version range: {}",
            s
        )))
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
    fn test_version_range_contains() {
        let range = VersionRange::parse(">=1.0.0").unwrap();
        let version1 = Version::parse("1.0.0").unwrap();
        let version2 = Version::parse("0.9.0").unwrap();
        let version3 = Version::parse("1.1.0").unwrap();

        assert!(range.contains_version(&version1));
        assert!(!range.contains_version(&version2));
        assert!(range.contains_version(&version3));
    }

    #[test]
    fn test_version_range_intersects() {
        let range1 = VersionRange::parse(">=1.0.0").unwrap();
        let range2 = VersionRange::parse("<=2.0.0").unwrap();
        let range3 = VersionRange::parse(">=3.0.0").unwrap();

        assert!(range1.intersects(&range2));
        assert!(!range1.intersects(&range3));
    }
}
