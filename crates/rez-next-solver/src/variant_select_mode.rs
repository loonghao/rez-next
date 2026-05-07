//! VariantSelectMode enum for variant selection mode.
//!
//! This module provides the `VariantSelectMode` enum that defines the
//! variant selection mode for the solver.

/// Variant selection mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariantSelectMode {
    /// Version priority mode
    VersionPriority = 0,
    /// Intersection priority mode
    IntersectionPriority = 1,
}

impl VariantSelectMode {
    /// Get the value of the variant select mode.
    ///
    /// # Returns
    ///
    /// The integer value of the mode.
    pub fn value(&self) -> i32 {
        *self as i32
    }

    /// Create a `VariantSelectMode` from an integer value.
    ///
    /// # Arguments
    ///
    /// * `value` - The integer value.
    ///
    /// # Returns
    ///
    /// `Some(VariantSelectMode)` if the value is valid, `None` otherwise.
    pub fn from_value(value: i32) -> Option<Self> {
        match value {
            0 => Some(VariantSelectMode::VersionPriority),
            1 => Some(VariantSelectMode::IntersectionPriority),
            _ => None,
        }
    }
}

impl std::fmt::Display for VariantSelectMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            VariantSelectMode::VersionPriority => "version_priority",
            VariantSelectMode::IntersectionPriority => "intersection_priority",
        };
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variant_select_mode_version_priority() {
        let mode = VariantSelectMode::VersionPriority;
        assert_eq!(mode.value(), 0);
        assert_eq!(format!("{}", mode), "version_priority");
    }

    #[test]
    fn test_variant_select_mode_intersection_priority() {
        let mode = VariantSelectMode::IntersectionPriority;
        assert_eq!(mode.value(), 1);
        assert_eq!(format!("{}", mode), "intersection_priority");
    }

    #[test]
    fn test_variant_select_mode_from_value() {
        assert_eq!(
            VariantSelectMode::from_value(0),
            Some(VariantSelectMode::VersionPriority)
        );
        assert_eq!(
            VariantSelectMode::from_value(1),
            Some(VariantSelectMode::IntersectionPriority)
        );
        assert_eq!(VariantSelectMode::from_value(2), None);
        assert_eq!(VariantSelectMode::from_value(-1), None);
    }

    #[test]
    fn test_variant_select_mode_partial_eq() {
        assert_eq!(
            VariantSelectMode::VersionPriority,
            VariantSelectMode::VersionPriority
        );
        assert_ne!(
            VariantSelectMode::VersionPriority,
            VariantSelectMode::IntersectionPriority
        );
    }

    #[test]
    fn test_variant_select_mode_clone() {
        let mode = VariantSelectMode::VersionPriority;
        let cloned = mode;
        assert_eq!(mode, cloned);
    }
}
