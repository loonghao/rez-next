//! Simple package requirement for basic solver/repository use.

use rez_next_common::RezCoreError;
use rez_next_version::Version;
use serde::{Deserialize, Serialize};

use crate::requirement::Requirement as ParsedRequirement;

/// Simple package requirement for basic functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageRequirement {
    /// Package name
    pub name: String,
    /// Version requirement (optional)
    pub version_spec: Option<String>,
    /// Whether this is a weak requirement (prefix ~)
    pub weak: bool,
    /// Whether this is a conflict requirement (prefix !)
    pub conflict: bool,
}

impl std::fmt::Display for PackageRequirement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let base = if let Some(ref version) = self.version_spec {
            format!("{}-{}", self.name, version)
        } else {
            self.name.clone()
        };
        if self.conflict {
            write!(f, "!{}", base)
        } else if self.weak {
            write!(f, "~{}", base)
        } else {
            write!(f, "{}", base)
        }
    }
}

impl PackageRequirement {
    /// Create a new package requirement
    pub fn new(name: String) -> Self {
        Self {
            name,
            version_spec: None,
            weak: false,
            conflict: false,
        }
    }

    /// Create a package requirement with version specification
    pub fn with_version(name: String, version_spec: String) -> Self {
        Self {
            name,
            version_spec: Some(version_spec),
            weak: false,
            conflict: false,
        }
    }

    /// Parse a requirement string.
    ///
    /// Supports the following rez requirement formats:
    /// - `python` — plain name requirement
    /// - `python-3.9` — name with version
    /// - `python>=3.9` — name with operator-prefixed version spec
    /// - `~python` — weak (optional) requirement
    /// - `!python` — conflict requirement (must NOT be present)
    /// - `!python-3.9` — conflict requirement with version
    pub fn parse(requirement_str: &str) -> Result<Self, RezCoreError> {
        let (s, conflict) = if let Some(rest) = requirement_str.strip_prefix('!') {
            (rest, true)
        } else {
            (requirement_str, false)
        };

        let (s, weak) = if s.starts_with('~') && !s.starts_with("~=") {
            if let Some(rest) = s.strip_prefix('~') {
                (rest, true)
            } else {
                (s, false)
            }
        } else {
            (s, false)
        };

        let parsed: ParsedRequirement = s.parse().map_err(RezCoreError::RequirementParse)?;
        let remainder = &s[parsed.name.len()..];
        let version_spec = remainder.strip_prefix('-').unwrap_or(remainder);
        let mut req = if version_spec.is_empty() {
            Self::new(parsed.name)
        } else {
            Self::with_version(parsed.name, version_spec.to_string())
        };
        req.weak = weak;
        req.conflict = conflict;
        Ok(req)
    }

    /// Get the package name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the version specification
    pub fn version_spec(&self) -> Option<&str> {
        self.version_spec.as_deref()
    }

    /// Get requirement string (for compatibility)
    pub fn requirement_string(&self) -> String {
        self.to_string()
    }

    /// Check if this requirement is satisfied by a version
    pub fn satisfied_by(&self, version: &Version) -> bool {
        if let Some(ref version_spec) = self.version_spec {
            let spec = version_spec.trim();
            if spec.is_empty() {
                return true;
            }
            if spec.contains(',') {
                return spec
                    .split(',')
                    .all(|part| Self::check_single_constraint(version, part.trim()));
            }
            Self::check_single_constraint(version, spec)
        } else {
            true
        }
    }

    /// Check a single version constraint like ">=1.0" or "2.1.0"
    fn check_single_constraint(version: &Version, spec: &str) -> bool {
        use rez_next_version::VersionRange;

        let (op, ver_str) = if let Some(rest) = spec.strip_prefix(">=") {
            (">=", rest)
        } else if let Some(rest) = spec.strip_prefix("<=") {
            ("<=", rest)
        } else if let Some(rest) = spec.strip_prefix("!=") {
            ("!=", rest)
        } else if let Some(rest) = spec.strip_prefix("~=") {
            ("~=", rest)
        } else if let Some(rest) = spec.strip_prefix("==") {
            ("==", rest)
        } else if let Some(rest) = spec.strip_prefix('>') {
            (">", rest)
        } else if let Some(rest) = spec.strip_prefix('<') {
            ("<", rest)
        } else {
            if let Ok(range) = VersionRange::parse(spec) {
                return range.contains(version);
            }
            ("==", spec)
        };

        let ver_str = ver_str.trim();
        if let Ok(constraint_ver) = Version::parse(ver_str) {
            use crate::requirement::VersionConstraint;
            let constraint = match op {
                ">=" => VersionConstraint::GreaterThanOrEqual(constraint_ver),
                "<=" => VersionConstraint::LessThanOrEqual(constraint_ver),
                ">" => VersionConstraint::GreaterThan(constraint_ver),
                "<" => VersionConstraint::LessThan(constraint_ver),
                "!=" => VersionConstraint::Exclude(vec![constraint_ver]),
                "~=" => VersionConstraint::Compatible(constraint_ver),
                _ => VersionConstraint::Exact(constraint_ver),
            };
            constraint.is_satisfied_by(version)
        } else {
            version.as_str() == ver_str
        }
    }
}
