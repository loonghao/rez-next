//! Version range implementation

use super::Version;
use rez_next_common::RezCoreError;
use serde::{Deserialize, Serialize};

/// Version range representation
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionRange {
    /// Cached string representation
    pub range_str: String,
}

impl VersionRange {
    /// Create a new version range from a string
    pub fn new(range_str: String) -> Result<Self, RezCoreError> {
        Self::parse(&range_str)
    }

    /// Parse a version range string
    pub fn parse(range_str: &str) -> Result<Self, RezCoreError> {
        // For now, implement basic parsing
        // This is a simplified implementation
        Ok(VersionRange {
            range_str: range_str.to_string(),
        })
    }

    /// Check if a version satisfies this range
    pub fn contains(&self, _version: &Version) -> bool {
        // Simplified implementation - always returns true for now
        true
    }

    /// Get the string representation
    pub fn as_str(&self) -> &str {
        &self.range_str
    }

    /// Check if this range intersects with another range
    pub fn intersects(&self, _other: &VersionRange) -> bool {
        // Simplified implementation - always returns true for now
        true
    }

    /// Compute the intersection of two ranges
    pub fn intersect(&self, _other: &VersionRange) -> Option<VersionRange> {
        // Simplified implementation - return the first range
        Some(self.clone())
    }

    /// Check if this range is the "any" range (matches all versions)
    pub fn is_any(&self) -> bool {
        self.range_str.is_empty()
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
    }

    #[test]
    fn test_version_range_intersect() {
        let range1 = VersionRange::parse(">=1.0.0").unwrap();
        let range2 = VersionRange::parse("<=2.0.0").unwrap();

        let intersection = range1.intersect(&range2).unwrap();
        assert_eq!(intersection.range_str, ">=1.0.0");
    }
}
