//! Requirement string parser.

use regex::Regex;
use rez_next_version::Version;

use super::types::{EnvCondition, PlatformCondition, Requirement, VersionConstraint};

/// Advanced requirement parser
pub struct RequirementParser {
    patterns: RequirementPatterns,
}

/// Compiled regex patterns for requirement parsing
struct RequirementPatterns {
    basic_version: Regex,
    namespace: Regex,
    wildcard: Regex,
}

impl RequirementParser {
    /// Create a new requirement parser
    pub fn new() -> Self {
        Self {
            patterns: RequirementPatterns::new(),
        }
    }

    /// Parse a requirement string
    pub fn parse(&self, s: &str) -> Result<Requirement, String> {
        let s = s.trim();

        let (s, weak) = if let Some(stripped) = s.strip_prefix('~') {
            (stripped, true)
        } else {
            (s, false)
        };

        let (s, namespace) = if let Some(captures) = self.patterns.namespace.captures(s) {
            let namespace = captures.get(1).unwrap().as_str().to_string();
            let rest = captures.get(2).unwrap().as_str();
            (rest, Some(namespace))
        } else {
            (s, None)
        };

        let (remaining_str, platform_conditions, env_conditions) = self.parse_conditions(s)?;
        let (name, version_constraint) = self.parse_name_and_version(&remaining_str)?;

        Ok(Requirement {
            name,
            version_constraint,
            weak,
            platform_conditions,
            env_conditions,
            conditional_expression: None,
            namespace,
        })
    }

    /// Parse platform and environment conditions from brackets
    fn parse_conditions(
        &self,
        s: &str,
    ) -> Result<(String, Vec<PlatformCondition>, Vec<EnvCondition>), String> {
        let mut platform_conditions = Vec::new();
        let mut env_conditions = Vec::new();
        let mut remaining = s.to_string();

        while let Some(start) = remaining.find('[') {
            if let Some(end) = remaining[start..].find(']') {
                let condition_str = &remaining[start + 1..start + end];
                let before = &remaining[..start];
                let after = &remaining[start + end + 1..];

                if condition_str.starts_with("platform") {
                    platform_conditions.push(self.parse_platform_condition(condition_str)?);
                } else if condition_str.starts_with("env.") {
                    env_conditions.push(self.parse_env_condition(condition_str)?);
                } else {
                    return Err(format!("Unknown condition type: {}", condition_str));
                }

                remaining = format!("{}{}", before, after);
            } else {
                return Err("Unclosed bracket in requirement".to_string());
            }
        }

        Ok((remaining, platform_conditions, env_conditions))
    }

    /// Parse platform condition
    fn parse_platform_condition(&self, condition: &str) -> Result<PlatformCondition, String> {
        let negate = condition.contains("!=");

        if let Some(platform_start) = condition.find('\'') {
            if let Some(platform_end) = condition[platform_start + 1..].find('\'') {
                let platform =
                    condition[platform_start + 1..platform_start + 1 + platform_end].to_string();

                let arch = if condition.contains("arch") {
                    if let Some(arch_start) = condition.rfind('\'') {
                        if arch_start > platform_start + 1 + platform_end {
                            condition[arch_start + 1..].find('\'').map(|arch_end| {
                                condition[arch_start + 1..arch_start + 1 + arch_end].to_string()
                            })
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                return Ok(PlatformCondition {
                    platform,
                    arch,
                    negate,
                });
            }
        }

        Err(format!("Invalid platform condition: {}", condition))
    }

    /// Parse environment condition
    fn parse_env_condition(&self, condition: &str) -> Result<EnvCondition, String> {
        let negate = condition.contains("!=");

        if let Some(var_start) = condition.find("env.") {
            let var_part = &condition[var_start + 4..];

            if let Some(op_pos) = var_part.find("==").or_else(|| var_part.find("!=")) {
                let var_name = var_part[..op_pos].to_string();
                let value_part = &var_part[op_pos + 2..];

                let expected_value = if value_part.trim() == "None" {
                    None
                } else if let Some(quote_start) = value_part.find('\'') {
                    if let Some(quote_end) = value_part[quote_start + 1..].find('\'') {
                        Some(value_part[quote_start + 1..quote_start + 1 + quote_end].to_string())
                    } else {
                        return Err("Unclosed quote in environment condition".to_string());
                    }
                } else {
                    Some(value_part.trim().to_string())
                };

                return Ok(EnvCondition {
                    var_name,
                    expected_value,
                    negate,
                });
            } else {
                return Ok(EnvCondition {
                    var_name: var_part.to_string(),
                    expected_value: None,
                    negate,
                });
            }
        }

        Err(format!("Invalid environment condition: {}", condition))
    }

    /// Parse package name and version constraint
    fn parse_name_and_version(
        &self,
        s: &str,
    ) -> Result<(String, Option<VersionConstraint>), String> {
        if let Some(captures) = self.patterns.wildcard.captures(s) {
            let name = captures.get(1).unwrap().as_str().to_string();
            let pattern = captures.get(2).unwrap().as_str().to_string();
            return Ok((name, Some(VersionConstraint::Wildcard(pattern))));
        }

        if s.contains(',') {
            if let Some(captures) = self.patterns.basic_version.captures(s) {
                let name = captures.get(1).unwrap().as_str().to_string();
                let constraints_str = &s[name.len()..];
                let constraint = self.parse_version_constraints(constraints_str)?;
                return Ok((name, Some(constraint)));
            }
        }

        if let Some(captures) = self.patterns.basic_version.captures(s) {
            let name = captures.get(1).unwrap().as_str().to_string();
            let op = captures.get(2).unwrap().as_str();
            let version_str = captures.get(3).unwrap().as_str();
            let version = Version::parse(version_str)
                .map_err(|e| format!("Invalid version {}: {}", version_str, e))?;

            let constraint = match op {
                "==" => VersionConstraint::Exact(version),
                ">" => VersionConstraint::GreaterThan(version),
                ">=" => VersionConstraint::GreaterThanOrEqual(version),
                "<" => VersionConstraint::LessThan(version),
                "<=" => VersionConstraint::LessThanOrEqual(version),
                "~=" => VersionConstraint::Compatible(version),
                _ => return Err(format!("Unknown version operator: {}", op)),
            };

            return Ok((name, Some(constraint)));
        }

        // Handle "package-1.0+" syntax
        if let Some(without_plus) = s.strip_suffix('+') {
            if let Some(dash_pos) = without_plus.rfind('-') {
                let name = without_plus[..dash_pos].to_string();
                let version_str = &without_plus[dash_pos + 1..];
                if Version::parse(version_str).is_ok() {
                    let version = Version::parse(version_str)
                        .map_err(|e| format!("Invalid version {}: {}", version_str, e))?;
                    return Ok((name, Some(VersionConstraint::GreaterThanOrEqual(version))));
                }
            }
        }

        // Handle rez combined format: "package-ver+<max", "package-ver+", "package-ver"
        {
            let bytes = s.as_bytes();
            let mut split_pos: Option<usize> = None;
            for i in (0..s.len()).rev() {
                if bytes[i] == b'-' {
                    let after = &s[i + 1..];
                    if after.starts_with(|c: char| c.is_ascii_digit()) {
                        split_pos = Some(i);
                        break;
                    }
                }
            }

            if let Some(pos) = split_pos {
                let name = s[..pos].to_string();
                let ver_spec = &s[pos + 1..];

                if let Some(constraint) = Self::parse_rez_version_spec(ver_spec) {
                    return Ok((name, Some(constraint)));
                }
            }
        }

        Ok((s.to_string(), None))
    }

    /// Parse complex version constraints (e.g., ">=1.0,<2.0")
    fn parse_version_constraints(
        &self,
        constraints_str: &str,
    ) -> Result<VersionConstraint, String> {
        let parts: Vec<&str> = constraints_str.split(',').collect();
        if parts.len() == 1 {
            return self.parse_single_constraint(parts[0]);
        }

        let mut constraints = Vec::new();
        for part in parts {
            constraints.push(self.parse_single_constraint(part.trim())?);
        }

        Ok(VersionConstraint::Multiple(constraints))
    }

    /// Parse rez-native version spec attached after the '-' separator.
    fn parse_rez_version_spec(spec: &str) -> Option<VersionConstraint> {
        if let Some(dot_pos) = spec.find("..") {
            let min_str = &spec[..dot_pos];
            let max_str = &spec[dot_pos + 2..];
            if let (Ok(min), Ok(max)) = (Version::parse(min_str), Version::parse(max_str)) {
                return Some(VersionConstraint::Multiple(vec![
                    VersionConstraint::GreaterThanOrEqual(min),
                    VersionConstraint::LessThan(max),
                ]));
            }
        }

        let (base_spec, upper_spec) = if let Some(lt_pos) = spec.find('<') {
            let base = spec[..lt_pos].trim_end_matches('+');
            let upper = &spec[lt_pos..];
            (base, Some(upper))
        } else {
            (spec.trim_end_matches('+'), None)
        };

        if !base_spec.starts_with(|c: char| c.is_ascii_digit()) {
            return None;
        }

        let min_ver = Version::parse(base_spec).ok()?;

        if let Some(upper) = upper_spec {
            let (op, ver_str) = if let Some(s) = upper.strip_prefix("<=") {
                ("<=", s.trim())
            } else {
                ("<", upper[1..].trim())
            };
            let max_ver = Version::parse(ver_str).ok()?;
            let max_constraint = if op == "<=" {
                VersionConstraint::LessThanOrEqual(max_ver)
            } else {
                VersionConstraint::LessThan(max_ver)
            };
            Some(VersionConstraint::Multiple(vec![
                VersionConstraint::GreaterThanOrEqual(min_ver),
                max_constraint,
            ]))
        } else if spec.ends_with('+') {
            Some(VersionConstraint::GreaterThanOrEqual(min_ver))
        } else {
            Some(VersionConstraint::Prefix(min_ver))
        }
    }

    /// Parse a single version constraint
    fn parse_single_constraint(&self, constraint_str: &str) -> Result<VersionConstraint, String> {
        let constraint_str = constraint_str.trim();
        let version_pattern = Regex::new(r"^(==|>=|<=|>|<|~=)(.+)$").unwrap();

        if let Some(captures) = version_pattern.captures(constraint_str) {
            let op = captures.get(1).unwrap().as_str();
            let version_str = captures.get(2).unwrap().as_str();
            let version = Version::parse(version_str)
                .map_err(|e| format!("Invalid version {}: {}", version_str, e))?;

            match op {
                "==" => Ok(VersionConstraint::Exact(version)),
                ">" => Ok(VersionConstraint::GreaterThan(version)),
                ">=" => Ok(VersionConstraint::GreaterThanOrEqual(version)),
                "<" => Ok(VersionConstraint::LessThan(version)),
                "<=" => Ok(VersionConstraint::LessThanOrEqual(version)),
                "~=" => Ok(VersionConstraint::Compatible(version)),
                _ => Err(format!("Unknown version operator: {}", op)),
            }
        } else {
            Err(format!("Invalid version constraint: {}", constraint_str))
        }
    }
}

impl RequirementPatterns {
    fn new() -> Self {
        Self {
            basic_version: Regex::new(r"^([a-zA-Z0-9_\-\.]+)(==|>=|<=|>|<|~=)(.+)$").unwrap(),
            namespace: Regex::new(r"^([a-zA-Z0-9_\-\.]+)::([a-zA-Z0-9_\-\.]+.*)$").unwrap(),
            wildcard: Regex::new(r"^([a-zA-Z0-9_\-\.]+)==(.+\*.*)$").unwrap(),
        }
    }
}

impl Default for RequirementParser {
    fn default() -> Self {
        Self::new()
    }
}
