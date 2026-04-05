//! Core requirement types and their methods.

use rez_next_version::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

    /// Platform-specific conditions
    pub platform_conditions: Vec<PlatformCondition>,

    /// Environment variable conditions
    pub env_conditions: Vec<EnvCondition>,

    /// Conditional expressions (for complex logic)
    pub conditional_expression: Option<String>,

    /// Namespace (for scoped packages)
    pub namespace: Option<String>,
}

/// Platform-specific condition
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlatformCondition {
    /// Platform name (e.g., "windows", "linux", "darwin")
    pub platform: String,
    /// Architecture (e.g., "x86_64", "aarch64")
    pub arch: Option<String>,
    /// Whether this condition should be negated
    pub negate: bool,
}

/// Environment variable condition
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EnvCondition {
    /// Environment variable name
    pub var_name: String,
    /// Expected value (None means just check existence)
    pub expected_value: Option<String>,
    /// Whether this condition should be negated
    pub negate: bool,
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
    /// Multiple constraints (AND logic)
    Multiple(Vec<VersionConstraint>),
    /// Alternative constraints (OR logic)
    Alternative(Vec<VersionConstraint>),
    /// Exclude specific versions
    Exclude(Vec<Version>),
    /// Wildcard pattern (e.g., "1.2.*")
    Wildcard(String),
    /// Prefix match: version starts with the given prefix tokens.
    /// Used for rez "point release" syntax: pkg-3.11 matches 3.11, 3.11.0, 3.11.5, etc.
    Prefix(Version),
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
            platform_conditions: Vec::new(),
            env_conditions: Vec::new(),
            conditional_expression: None,
            namespace: None,
        }
    }

    /// Create a requirement with version constraint
    pub fn with_version(name: String, constraint: VersionConstraint) -> Self {
        Self {
            name,
            version_constraint: Some(constraint),
            weak: false,
            platform_conditions: Vec::new(),
            env_conditions: Vec::new(),
            conditional_expression: None,
            namespace: None,
        }
    }

    /// Create a weak requirement
    pub fn weak(name: String) -> Self {
        Self {
            name,
            version_constraint: None,
            weak: true,
            platform_conditions: Vec::new(),
            env_conditions: Vec::new(),
            conditional_expression: None,
            namespace: None,
        }
    }

    /// Check if a version satisfies this requirement
    pub fn is_satisfied_by(&self, version: &Version) -> bool {
        match &self.version_constraint {
            None => true,
            Some(constraint) => constraint.is_satisfied_by(version),
        }
    }

    /// Check if platform conditions are satisfied
    pub fn is_platform_satisfied(&self, platform: &str, arch: Option<&str>) -> bool {
        if self.platform_conditions.is_empty() {
            return true;
        }

        for condition in &self.platform_conditions {
            let platform_match = condition.platform == platform;
            let arch_match = condition
                .arch
                .as_ref()
                .map_or(true, |a| arch.is_some_and(|arch| arch == a));

            let condition_satisfied = platform_match && arch_match;

            if condition.negate {
                if condition_satisfied {
                    return false;
                }
            } else if condition_satisfied {
                return true;
            }
        }

        self.platform_conditions.iter().all(|c| c.negate)
    }

    /// Check if environment conditions are satisfied
    pub fn is_env_satisfied(&self, env_vars: &HashMap<String, String>) -> bool {
        if self.env_conditions.is_empty() {
            return true;
        }

        for condition in &self.env_conditions {
            let var_exists = env_vars.contains_key(&condition.var_name);
            let value_match = if let Some(expected) = &condition.expected_value {
                env_vars.get(&condition.var_name) == Some(expected)
            } else {
                var_exists
            };

            if condition.negate {
                if value_match {
                    return false;
                }
            } else if !value_match {
                return false;
            }
        }

        true
    }

    /// Get the package name
    pub fn package_name(&self) -> &str {
        &self.name
    }

    /// Get the full qualified name (including namespace if present)
    pub fn qualified_name(&self) -> String {
        if let Some(ref namespace) = self.namespace {
            format!("{}::{}", namespace, self.name)
        } else {
            self.name.clone()
        }
    }

    /// Add a platform condition
    pub fn add_platform_condition(&mut self, platform: String, arch: Option<String>, negate: bool) {
        self.platform_conditions.push(PlatformCondition {
            platform,
            arch,
            negate,
        });
    }

    /// Add an environment condition
    pub fn add_env_condition(
        &mut self,
        var_name: String,
        expected_value: Option<String>,
        negate: bool,
    ) {
        self.env_conditions.push(EnvCondition {
            var_name,
            expected_value,
            negate,
        });
    }
}

impl VersionConstraint {
    /// Check if a version satisfies this constraint.
    ///
    /// Rez semantics: when comparing against a constraint version with fewer tokens,
    /// the comparison is done at the depth of the constraint.
    /// e.g., `>=3` on `3.11.0` → compare first token: `3 >= 3` ✓
    ///        `<4` on `3.11.0`  → compare first token: `3 < 4` ✓
    pub fn is_satisfied_by(&self, version: &Version) -> bool {
        match self {
            VersionConstraint::Exact(v) => {
                Self::cmp_at_depth(version, v) == std::cmp::Ordering::Equal
            }
            VersionConstraint::GreaterThan(v) => {
                Self::cmp_at_depth(version, v) == std::cmp::Ordering::Greater
            }
            VersionConstraint::GreaterThanOrEqual(v) => {
                let ord = Self::cmp_at_depth(version, v);
                ord == std::cmp::Ordering::Greater || ord == std::cmp::Ordering::Equal
            }
            VersionConstraint::LessThan(v) => {
                Self::cmp_at_depth(version, v) == std::cmp::Ordering::Less
            }
            VersionConstraint::LessThanOrEqual(v) => {
                let ord = Self::cmp_at_depth(version, v);
                ord == std::cmp::Ordering::Less || ord == std::cmp::Ordering::Equal
            }
            VersionConstraint::Compatible(v) => {
                // Compatible version (~=) uses a "locked prefix + floor" rule.
                // Rule:
                //   ~=X.Y   → prefix=["X"] (locked), minor >= Y
                //   ~=X.Y.Z → prefix=["X","Y"] (locked), patch >= Z
                let version_parts: Vec<&str> = version.as_str().split('.').collect();
                let constraint_parts: Vec<&str> = v.as_str().split('.').collect();

                if constraint_parts.is_empty() {
                    return true;
                }
                if version_parts.len() < constraint_parts.len() {
                    return false;
                }

                let last_idx = constraint_parts.len() - 1;
                for i in 0..last_idx {
                    if version_parts[i] != constraint_parts[i] {
                        return false;
                    }
                }

                let v_last = version_parts[last_idx];
                let c_last = constraint_parts[last_idx];
                if let (Ok(vn), Ok(cn)) = (v_last.parse::<u64>(), c_last.parse::<u64>()) {
                    vn >= cn
                } else {
                    v_last >= c_last
                }
            }
            VersionConstraint::Range(min, max) => version >= min && version < max,
            VersionConstraint::Multiple(constraints) => {
                constraints.iter().all(|c| c.is_satisfied_by(version))
            }
            VersionConstraint::Alternative(constraints) => {
                constraints.iter().any(|c| c.is_satisfied_by(version))
            }
            VersionConstraint::Exclude(versions) => !versions.iter().any(|v| version == v),
            VersionConstraint::Wildcard(pattern) => self.matches_wildcard(version, pattern),
            VersionConstraint::Prefix(prefix) => {
                let ver_str = version.as_str();
                let prefix_str = prefix.as_str();
                ver_str == prefix_str || ver_str.starts_with(&format!("{}.", prefix_str))
            }
            VersionConstraint::Any => true,
        }
    }

    /// Check if version matches wildcard pattern
    fn matches_wildcard(&self, version: &Version, pattern: &str) -> bool {
        let version_str = version.as_str();
        let pattern_parts: Vec<&str> = pattern.split('.').collect();
        let version_parts: Vec<&str> = version_str.split('.').collect();

        for (i, pattern_part) in pattern_parts.iter().enumerate() {
            if *pattern_part == "*" {
                return true;
            }
            if i >= version_parts.len() {
                return false;
            }
            if *pattern_part != version_parts[i] {
                return false;
            }
        }

        pattern_parts.len() == version_parts.len()
    }

    /// Compare `version` against `constraint_ver` at the depth of `constraint_ver`.
    ///
    /// Rez semantics: constraints with fewer tokens are compared only at the token depth
    /// of the constraint.
    pub fn cmp_at_depth(version: &Version, constraint_ver: &Version) -> std::cmp::Ordering {
        let ver_str = version.as_str();
        let con_str = constraint_ver.as_str();

        let ver_parts: Vec<&str> = ver_str.split('.').collect();
        let con_parts: Vec<&str> = con_str.split('.').collect();

        let depth = con_parts.len();

        for (v_tok, c_tok) in ver_parts.iter().zip(con_parts.iter()).take(depth) {
            let v_tok = *v_tok;
            let c_tok = *c_tok;

            if let (Ok(vn), Ok(cn)) = (v_tok.parse::<u64>(), c_tok.parse::<u64>()) {
                match vn.cmp(&cn) {
                    std::cmp::Ordering::Equal => continue,
                    ord => return ord,
                }
            } else {
                match v_tok.cmp(c_tok) {
                    std::cmp::Ordering::Equal => continue,
                    ord => return ord,
                }
            }
        }

        std::cmp::Ordering::Equal
    }

    /// Combine two constraints with AND logic
    pub fn and(self, other: VersionConstraint) -> VersionConstraint {
        match (self, other) {
            (VersionConstraint::Multiple(mut constraints), other) => {
                constraints.push(other);
                VersionConstraint::Multiple(constraints)
            }
            (self_constraint, VersionConstraint::Multiple(mut constraints)) => {
                constraints.insert(0, self_constraint);
                VersionConstraint::Multiple(constraints)
            }
            (self_constraint, other) => VersionConstraint::Multiple(vec![self_constraint, other]),
        }
    }

    /// Combine two constraints with OR logic
    pub fn or(self, other: VersionConstraint) -> VersionConstraint {
        match (self, other) {
            (VersionConstraint::Alternative(mut constraints), other) => {
                constraints.push(other);
                VersionConstraint::Alternative(constraints)
            }
            (self_constraint, VersionConstraint::Alternative(mut constraints)) => {
                constraints.insert(0, self_constraint);
                VersionConstraint::Alternative(constraints)
            }
            (self_constraint, other) => {
                VersionConstraint::Alternative(vec![self_constraint, other])
            }
        }
    }
}

impl FromStr for Requirement {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        super::parser::RequirementParser::new().parse(s)
    }
}
