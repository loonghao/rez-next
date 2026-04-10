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
