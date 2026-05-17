//! Package filter implementation.
//!
//! `PackageFilter` manages inclusion and exclusion rules to filter packages.

use std::collections::HashMap;

use rez_next_package::Package;
use serde::{Deserialize, Serialize};

use super::FilterError;
use super::Rule;
use super::RuleMatch;

/// A package filter that manages inclusion and exclusion rules.
///
/// A package is excluded if it matches ANY exclusion rule AND does not
/// match ANY inclusion rule. Inclusion rules override exclusion rules.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageFilter {
    /// Exclusion rules - packages matching any of these are excluded
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exclusions: Vec<RulePod>,

    /// Inclusion rules - packages matching any of these are included
    /// (overrides exclusion)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub inclusions: Vec<RulePod>,

    /// Cache for computed SHA1 (lazily computed)
    #[serde(skip)]
    sha1_cache: Option<String>,
}

/// Plain-old-data representation of a rule for serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulePod {
    /// Rule type: "glob", "regex", "range", "before", "after"
    pub rule_type: String,
    /// Rule pattern/value
    pub pattern: String,
}

impl PackageFilter {
    /// Create a new empty package filter.
    pub fn new() -> Self {
        Self {
            exclusions: Vec::new(),
            inclusions: Vec::new(),
            sha1_cache: None,
        }
    }

    /// Check if the filter excludes the given package.
    ///
    /// Returns `Some(rule)` if the package is excluded by `rule`,
    /// `None` if the package is not excluded.
    pub fn excludes(&self, package: &Package) -> Option<&RulePod> {
        // Check exclusion rules
        for rule_pod in &self.exclusions {
            if let Ok(rule) = self.pod_to_rule(rule_pod) {
                if matches!(rule.apply(package), RuleMatch::Matches) {
                    // Check if any inclusion rule matches
                    let included = self.inclusions.iter().any(|inc| {
                        if let Ok(r) = self.pod_to_rule(inc) {
                            matches!(r.apply(package), RuleMatch::Matches)
                        } else {
                            false
                        }
                    });

                    if !included {
                        return Some(rule_pod);
                    }
                }
            }
        }

        None
    }

    /// Check if the filter includes the given package.
    ///
    /// Returns `true` if the package is explicitly included or not excluded.
    pub fn includes(&self, package: &Package) -> bool {
        // Check inclusion rules first
        for rule_pod in &self.inclusions {
            if let Ok(rule) = self.pod_to_rule(rule_pod) {
                if matches!(rule.apply(package), RuleMatch::Matches) {
                    return true;
                }
            }
        }

        // Check if excluded
        self.excludes(package).is_none()
    }

    /// Add an exclusion rule.
    pub fn add_exclusion(&mut self, rule: Box<dyn Rule>) {
        let pod = self.rule_to_pod(&*rule);
        self.exclusions.push(pod);
        self.sha1_cache = None; // Invalidate cache
    }

    /// Add an inclusion rule.
    pub fn add_inclusion(&mut self, rule: Box<dyn Rule>) {
        let pod = self.rule_to_pod(&*rule);
        self.inclusions.push(pod);
        self.sha1_cache = None; // Invalidate cache
    }

    /// Add an exclusion rule from a string.
    pub fn add_exclusion_from_str(&mut self, txt: &str) -> Result<(), FilterError> {
        let rule = super::parse_rule(txt)?;
        self.add_exclusion(rule);
        Ok(())
    }

    /// Add an inclusion rule from a string.
    pub fn add_inclusion_from_str(&mut self, txt: &str) -> Result<(), FilterError> {
        let rule = super::parse_rule(txt)?;
        self.add_inclusion(rule);
        Ok(())
    }

    /// Convert to POD (Plain Old Data) for serialization.
    ///
    /// Returns a map with "exclusions" and "inclusions" keys.
    pub fn to_pod(&self) -> HashMap<String, Vec<String>> {
        let mut result = HashMap::new();

        if !self.exclusions.is_empty() {
            result.insert(
                "exclusions".to_string(),
                self.exclusions
                    .iter()
                    .map(|p| format!("{}({})", p.rule_type, p.pattern))
                    .collect(),
            );
        }

        if !self.inclusions.is_empty() {
            result.insert(
                "inclusions".to_string(),
                self.inclusions
                    .iter()
                    .map(|p| format!("{}({})", p.rule_type, p.pattern))
                    .collect(),
            );
        }

        result
    }

    /// Create a `PackageFilter` from POD.
    pub fn from_pod(data: &HashMap<String, Vec<String>>) -> Result<Self, FilterError> {
        let mut filter = Self::new();

        if let Some(exclusions) = data.get("exclusions") {
            for txt in exclusions {
                filter.add_exclusion_from_str(txt)?;
            }
        }

        if let Some(inclusions) = data.get("inclusions") {
            for txt in inclusions {
                filter.add_inclusion_from_str(txt)?;
            }
        }

        Ok(filter)
    }

    /// Calculate SHA1 hash of this filter.
    pub fn sha1(&self) -> String {
        if let Some(ref cached) = self.sha1_cache {
            return cached.clone();
        }

        use sha1::{Digest, Sha1};

        let mut hasher = Sha1::new();
        let repr = format!("{}", self);
        hasher.update(repr.as_bytes());

        let result = hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join("");
        // Note: can't mutate self here because of borrowing
        // Cache will be populated on next call after a mutable operation
        result
    }

    /// Convert a rule to POD representation.
    fn rule_to_pod(&self, rule: &dyn Rule) -> RulePod {
        let (rule_type, pattern) = rule.to_pod();
        RulePod { rule_type, pattern }
    }

    /// Convert POD to a rule.
    fn pod_to_rule(&self, pod: &RulePod) -> Result<Box<dyn Rule>, FilterError> {
        // Format as "type(pattern)" for parse_rule
        let txt = format!("{}({})", pod.rule_type, pod.pattern);
        super::parse_rule(&txt)
    }
}

impl std::fmt::Display for PackageFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PackageFilter(")?;

        if !self.exclusions.is_empty() {
            write!(f, "exclusions=[")?;
            for (i, rule) in self.exclusions.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}:{}", rule.rule_type, rule.pattern)?;
            }
            write!(f, "]")?;
        }

        if !self.inclusions.is_empty() {
            if !self.exclusions.is_empty() {
                write!(f, ", ")?;
            }
            write!(f, "inclusions=[")?;
            for (i, rule) in self.inclusions.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}:{}", rule.rule_type, rule.pattern)?;
            }
            write!(f, "]")?;
        }

        write!(f, ")")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_package(name: &str, version: &str) -> Package {
        Package {
            name: name.to_string(),
            version: Some(rez_next_version::Version::parse(version).unwrap()),
            description: Some("Test package".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn test_excludes_no_rules() {
        let filter = PackageFilter::new();
        let pkg = create_test_package("maya", "2024.0.0");

        assert!(filter.excludes(&pkg).is_none());
    }

    #[test]
    fn test_excludes_with_exclusion() {
        let mut filter = PackageFilter::new();
        filter.add_exclusion_from_str("glob(*.beta)").unwrap();

        let pkg_beta = create_test_package("maya.beta", "2024.0.0");
        let pkg_release = create_test_package("maya", "2024.0.0");

        assert!(filter.excludes(&pkg_beta).is_some());
        assert!(filter.excludes(&pkg_release).is_none());
    }

    #[test]
    fn test_excludes_with_inclusion_override() {
        let mut filter = PackageFilter::new();
        filter.add_exclusion_from_str("glob(*.beta)").unwrap();
        // Use glob(maya.beta) for exact match (no glob chars = exact match)
        filter.add_inclusion_from_str("glob(maya.beta)").unwrap();

        let pkg = create_test_package("maya.beta", "2024.0.0");

        // Should NOT be excluded because inclusion overrides
        assert!(filter.excludes(&pkg).is_none());
        // Should be included
        assert!(filter.includes(&pkg));
    }

    #[test]
    fn test_to_pod_and_from_pod() {
        let mut filter = PackageFilter::new();
        filter.add_exclusion_from_str("glob(*.beta)").unwrap();
        filter.add_inclusion_from_str("range(>=2024)").unwrap();

        let pod = filter.to_pod();
        let restored = PackageFilter::from_pod(&pod).unwrap();

        assert_eq!(filter.exclusions.len(), restored.exclusions.len());
        assert_eq!(filter.inclusions.len(), restored.inclusions.len());

        // Verify the restored filter produces the same POD
        let pod2 = restored.to_pod();
        assert_eq!(pod, pod2);
    }

    #[test]
    fn test_sha1() {
        let mut filter = PackageFilter::new();
        filter.add_exclusion_from_str("glob(*.beta)").unwrap();

        let sha1 = filter.sha1();
        assert!(!sha1.is_empty());
        assert_eq!(sha1.len(), 40); // SHA1 is 40 hex chars
    }

    #[test]
    fn test_includes() {
        let mut filter = PackageFilter::new();
        filter.add_exclusion_from_str("glob(*.beta)").unwrap();

        let pkg_beta = create_test_package("maya.beta", "2024.0.0");
        let pkg_release = create_test_package("maya", "2024.0.0");

        assert!(!filter.includes(&pkg_beta));
        assert!(filter.includes(&pkg_release));
    }
}
