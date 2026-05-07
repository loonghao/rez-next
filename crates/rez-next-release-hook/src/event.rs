//! Release hook event types.
//!
//! Enum to help manage release hooks.

use serde::{Deserialize, Serialize};

/// Enum representing the different release hook events.
///
/// This is used to identify which hook method to call
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReleaseHookEvent {
    /// Pre-build hook (called before the build process starts)
    PreBuild,

    /// Pre-release hook (called before any variants are released)
    PreRelease,

    /// Post-release hook (called after all variants have been released)
    PostRelease,
}

impl ReleaseHookEvent {
    /// Returns the label for this event (e.g., "pre-build").
    pub fn label(&self) -> &'static str {
        match self {
            ReleaseHookEvent::PreBuild => "pre-build",
            ReleaseHookEvent::PreRelease => "pre-release",
            ReleaseHookEvent::PostRelease => "post-release",
        }
    }

    /// Returns the noun for this event (e.g., "build" or "release").
    pub fn noun(&self) -> &'static str {
        match self {
            ReleaseHookEvent::PreBuild => "build",
            ReleaseHookEvent::PreRelease => "release",
            ReleaseHookEvent::PostRelease => "release",
        }
    }

    /// Returns the function name for this event (e.g., "pre_build").
    pub fn func_name(&self) -> &'static str {
        match self {
            ReleaseHookEvent::PreBuild => "pre_build",
            ReleaseHookEvent::PreRelease => "pre_release",
            ReleaseHookEvent::PostRelease => "post_release",
        }
    }
}

impl std::fmt::Display for ReleaseHookEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pre_build_event() {
        let event = ReleaseHookEvent::PreBuild;
        assert_eq!(event.label(), "pre-build");
        assert_eq!(event.noun(), "build");
        assert_eq!(event.func_name(), "pre_build");
        assert_eq!(format!("{}", event), "pre-build");
    }

    #[test]
    fn test_pre_release_event() {
        let event = ReleaseHookEvent::PreRelease;
        assert_eq!(event.label(), "pre-release");
        assert_eq!(event.noun(), "release");
        assert_eq!(event.func_name(), "pre_release");
        assert_eq!(format!("{}", event), "pre-release");
    }

    #[test]
    fn test_post_release_event() {
        let event = ReleaseHookEvent::PostRelease;
        assert_eq!(event.label(), "post-release");
        assert_eq!(event.noun(), "release");
        assert_eq!(event.func_name(), "post_release");
        assert_eq!(format!("{}", event), "post-release");
    }

    #[test]
    fn test_event_equality() {
        assert_eq!(ReleaseHookEvent::PreBuild, ReleaseHookEvent::PreBuild);
        assert_ne!(ReleaseHookEvent::PreBuild, ReleaseHookEvent::PreRelease);
    }

    #[test]
    fn test_event_serialization() {
        let event = ReleaseHookEvent::PreBuild;
        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: ReleaseHookEvent = serde_json::from_str(&serialized).unwrap();
        assert_eq!(event, deserialized);
    }
}
