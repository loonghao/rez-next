//! Display implementations for requirement types.

use std::fmt;

use super::types::{EnvCondition, PlatformCondition, Requirement, VersionConstraint};

impl fmt::Display for Requirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.weak {
            write!(f, "~")?;
        }
        if let Some(ref namespace) = self.namespace {
            write!(f, "{}::", namespace)?;
        }
        write!(f, "{}", self.name)?;
        if let Some(ref constraint) = self.version_constraint {
            write!(f, "{}", constraint)?;
        }
        for condition in &self.platform_conditions {
            write!(f, "[{}]", condition)?;
        }
        for condition in &self.env_conditions {
            write!(f, "[{}]", condition)?;
        }
        Ok(())
    }
}

impl fmt::Display for PlatformCondition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op = if self.negate { "!=" } else { "==" };
        write!(
            f,
            "platform{}'{}'{}",
            op,
            self.platform,
            self.arch
                .as_ref()
                .map_or(String::new(), |a| format!(" and arch=='{}'", a))
        )
    }
}

impl fmt::Display for EnvCondition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op = if self.negate { "!=" } else { "==" };
        match &self.expected_value {
            Some(value) => write!(f, "env.{}{}'{}'", self.var_name, op, value),
            None => write!(f, "env.{}", self.var_name),
        }
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
            VersionConstraint::Range(min, max) => {
                write!(f, ">={},<{}", min.as_str(), max.as_str())
            }
            VersionConstraint::Multiple(constraints) => {
                let strs: Vec<String> = constraints.iter().map(|c| c.to_string()).collect();
                write!(f, "{}", strs.join(","))
            }
            VersionConstraint::Alternative(constraints) => {
                let strs: Vec<String> = constraints.iter().map(|c| c.to_string()).collect();
                write!(f, "({})", strs.join(" || "))
            }
            VersionConstraint::Exclude(versions) => {
                let strs: Vec<String> = versions
                    .iter()
                    .map(|v| format!("!={}", v.as_str()))
                    .collect();
                write!(f, "{}", strs.join(","))
            }
            VersionConstraint::Wildcard(pattern) => write!(f, "=={}", pattern),
            VersionConstraint::Prefix(v) => write!(f, "-{}", v.as_str()),
            VersionConstraint::Any => write!(f, ""),
        }
    }
}
