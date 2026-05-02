//! Core bound types for version range representation.

use super::super::Version;

/// A single bound in a version range
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Bound {
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
pub(super) struct BoundSet {
    pub(super) bounds: Vec<Bound>,
}

impl BoundSet {
    pub(super) fn any() -> Self {
        BoundSet {
            bounds: vec![Bound::Any],
        }
    }

    pub(super) fn none() -> Self {
        BoundSet {
            bounds: vec![Bound::None],
        }
    }

    pub(super) fn contains(&self, version: &Version) -> bool {
        for bound in &self.bounds {
            if !bound_matches(bound, version) {
                return false;
            }
        }
        true
    }
}

pub(super) fn bound_matches(bound: &Bound, version: &Version) -> bool {
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
            // ~=v means >=v AND <upper_bound(v) (PEP 440 / rez semantics)
            // For 2 segments: ~=X.Y -> >=X.Y,<(X+1).0
            // For 3+ segments: ~=X.Y.Z -> >=X.Y.Z,<X.(Y+1).0
            if version < v {
                return false; // Must be >= v (lower bound)
            }
            let parts: Vec<&str> = v.as_str().split('.').collect();
            if parts.len() < 2 {
                return version >= v;
            }
            // Calculate upper bound
            let upper_str = if parts.len() == 2 {
                // X.Y -> (X+1).0
                if let Ok(x) = parts[0].parse::<u64>() {
                    format!("{}.0", x + 1)
                } else {
                    return true;
                }
            } else {
                // X.Y.Z... -> X.(Y+1).0
                if let Ok(y) = parts[1].parse::<u64>() {
                    format!("{}.{}.0", parts[0], y + 1)
                } else {
                    return true;
                }
            };
            if let Ok(upper) = Version::parse(&upper_str) {
                version < &upper
            } else {
                // Fallback: just check >= v
                version >= v
            }
        }
    }
}
