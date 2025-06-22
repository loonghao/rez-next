//! Package requirement parsing and handling

use rez_next_version::Version;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// A package requirement specification
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Requirement {
    /// Package name
    pub name: String,

    /// Version constraint
    pub version_constraint: Option<VersionConstraint>,

    /// Whether this is a weak requirement (optional)
    pub weak: bool,
}

/// Version constraint types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VersionConstraint {
    /// Exact version match (==)
    Exact(Version),

    /// Greater than (>)
    GreaterThan(Version),

    /// Greater than or equal (>=)
    GreaterThanOrEqual(Version),

    /// Less than (<)
    LessThan(Version),

    /// Less than or equal (<=)
    LessThanOrEqual(Version),

    /// Compatible version (~=)
    Compatible(Version),

    /// Range constraint (>=min, <max)
    Range(Version, Version),

    /// Any version
    Any,
}

impl Requirement {
    /// Create a new requirement
    pub fn new(name: String) -> Self {
        Self {
            name,
            version_constraint: None,
            weak: false,
        }
    }

    /// Check if a version satisfies this requirement
    pub fn is_satisfied_by(&self, version: &Version) -> bool {
        match &self.version_constraint {
            None => true, // No constraint means any version is acceptable
            Some(constraint) => constraint.is_satisfied_by(version),
        }
    }

    /// Get the package name
    pub fn package_name(&self) -> &str {
        &self.name
    }
}

impl VersionConstraint {
    /// Check if a version satisfies this constraint
    pub fn is_satisfied_by(&self, version: &Version) -> bool {
        match self {
            VersionConstraint::Exact(v) => version == v,
            VersionConstraint::GreaterThan(v) => version > v,
            VersionConstraint::GreaterThanOrEqual(v) => version >= v,
            VersionConstraint::LessThan(v) => version < v,
            VersionConstraint::LessThanOrEqual(v) => version <= v,
            VersionConstraint::Compatible(v) => {
                // Compatible version (~=) means >= v but < next major version
                version >= v && version.as_str().split('.').next() == v.as_str().split('.').next()
            }
            VersionConstraint::Range(min, max) => version >= min && version < max,
            VersionConstraint::Any => true,
        }
    }
}

impl FromStr for Requirement {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        // Handle weak requirements (starting with ~)
        let (s, weak) = if s.starts_with("~") {
            (&s[1..], true)
        } else {
            (s, false)
        };

        // Parse version constraints
        if s.ends_with("+") {
            // Handle "package-1.0+" syntax (equivalent to >=1.0)
            let without_plus = &s[..s.len() - 1];
            if let Some(dash_pos) = without_plus.rfind("-") {
                let name = without_plus[..dash_pos].to_string();
                let version_str = &without_plus[dash_pos + 1..];
                let version = Version::parse(version_str)
                    .map_err(|e| format!("Invalid version {}: {}", version_str, e))?;
                Ok(Requirement {
                    name,
                    version_constraint: Some(VersionConstraint::GreaterThanOrEqual(version)),
                    weak,
                })
            } else {
                Ok(Requirement {
                    name: s.to_string(),
                    version_constraint: None,
                    weak,
                })
            }
        } else {
            // Just a package name
            Ok(Requirement {
                name: s.to_string(),
                version_constraint: None,
                weak,
            })
        }
    }
}

impl fmt::Display for Requirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.weak {
            write!(f, "~")?;
        }

        write!(f, "{}", self.name)?;

        if let Some(ref constraint) = self.version_constraint {
            write!(f, "{}", constraint)?;
        }

        Ok(())
    }
}

impl fmt::Display for VersionConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionConstraint::Exact(v) => write!(f, "=={}", v.as_str()),
            VersionConstraint::GreaterThan(v) => write!(f, ">{}", v.as_str()),
            VersionConstraint::GreaterThanOrEqual(v) => write!(f, ">={}", v.as_str()),
            VersionConstraint::LessThan(v) => write!(f, "<{}", v.as_str()),
            VersionConstraint::LessThanOrEqual(v) => write!(f, "<={}", v.as_str()),
            VersionConstraint::Compatible(v) => write!(f, "~={}", v.as_str()),
            VersionConstraint::Range(min, max) => write!(f, ">={},<{}", min.as_str(), max.as_str()),
            VersionConstraint::Any => write!(f, ""),
        }
    }
}
