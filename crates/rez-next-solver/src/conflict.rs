//! Conflict detection and resolution

use crate::{ConflictResolution, ConflictStrategy, DependencyConflict};
use rez_next_common::RezCoreError;
use rez_next_version::Version;
use std::collections::HashMap;

/// Conflict resolver for dependency conflicts
#[derive(Debug)]
pub struct ConflictResolver {
    /// Conflict resolution strategy
    strategy: ConflictStrategy,
    /// Resolution cache
    cache: HashMap<String, ConflictResolution>,
}

impl ConflictResolver {
    /// Create a new conflict resolver
    pub fn new(strategy: ConflictStrategy) -> Self {
        Self {
            strategy,
            cache: HashMap::new(),
        }
    }

    /// Resolve a list of conflicts
    pub async fn resolve_conflicts(
        &self,
        conflicts: Vec<DependencyConflict>,
    ) -> Result<Vec<ConflictResolution>, RezCoreError> {
        let mut resolutions = Vec::new();

        for conflict in conflicts {
            let resolution = self.resolve_single_conflict(&conflict).await?;
            resolutions.push(resolution);
        }

        Ok(resolutions)
    }

    /// Resolve a single conflict
    async fn resolve_single_conflict(
        &self,
        conflict: &DependencyConflict,
    ) -> Result<ConflictResolution, RezCoreError> {
        // Check cache first
        let cache_key = self.generate_conflict_cache_key(conflict);
        if let Some(cached_resolution) = self.cache.get(&cache_key) {
            return Ok(cached_resolution.clone());
        }

        let resolution = match self.strategy {
            ConflictStrategy::LatestWins => self.resolve_latest_wins(conflict).await?,
            ConflictStrategy::EarliestWins => self.resolve_earliest_wins(conflict).await?,
            ConflictStrategy::FailOnConflict => {
                return Err(RezCoreError::Solver(format!(
                    "Conflict detected for package {}: {:?}",
                    conflict.package_name, conflict.conflicting_requirements
                )));
            }
            ConflictStrategy::FindCompatible => self.resolve_find_compatible(conflict).await?,
        };

        Ok(resolution)
    }

    /// Resolve conflict by selecting the latest version (by lexicographic version spec comparison)
    async fn resolve_latest_wins(
        &self,
        conflict: &DependencyConflict,
    ) -> Result<ConflictResolution, RezCoreError> {
        let mut latest_version: Option<Version> = None;
        let modified_packages = conflict.source_packages.clone();

        // Find the latest version among all requirements by parsing version specs
        for requirement in &conflict.conflicting_requirements {
            if let Some(ref version_spec) = requirement.version_spec
                && let Ok(v) = Version::parse(version_spec)
            {
                match &latest_version {
                    Some(current) if v > *current => latest_version = Some(v),
                    None => latest_version = Some(v),
                    _ => {}
                }
            }
        }

        Ok(ConflictResolution {
            package_name: conflict.package_name.clone(),
            selected_version: latest_version,
            strategy: "latest_wins".to_string(),
            modified_packages,
        })
    }

    /// Resolve conflict by selecting the earliest version
    async fn resolve_earliest_wins(
        &self,
        conflict: &DependencyConflict,
    ) -> Result<ConflictResolution, RezCoreError> {
        let mut earliest_version: Option<Version> = None;
        let modified_packages = conflict.source_packages.clone();

        for requirement in &conflict.conflicting_requirements {
            if let Some(ref version_spec) = requirement.version_spec
                && let Ok(v) = Version::parse(version_spec)
            {
                match &earliest_version {
                    Some(current) if v < *current => earliest_version = Some(v),
                    None => earliest_version = Some(v),
                    _ => {}
                }
            }
        }

        Ok(ConflictResolution {
            package_name: conflict.package_name.clone(),
            selected_version: earliest_version,
            strategy: "earliest_wins".to_string(),
            modified_packages,
        })
    }

    /// Resolve conflict by finding a compatible version (fallback to latest wins)
    async fn resolve_find_compatible(
        &self,
        conflict: &DependencyConflict,
    ) -> Result<ConflictResolution, RezCoreError> {
        // Try to find a version that satisfies all requirements
        // For now, collect all version specs and attempt to find a common one
        let mut candidate: Option<Version> = None;

        for requirement in &conflict.conflicting_requirements {
            if let Some(ref version_spec) = requirement.version_spec
                && let Ok(v) = Version::parse(version_spec)
            {
                // Check if this version satisfies all other requirements
                let satisfies_all = conflict.conflicting_requirements.iter().all(|other_req| {
                    if let Some(ref other_spec) = other_req.version_spec {
                        // Simple compatibility: versions match or other has no constraint
                        other_spec == version_spec || other_spec.is_empty()
                    } else {
                        true
                    }
                });

                if satisfies_all {
                    candidate = Some(v);
                    break;
                }
            }
        }

        // Fall back to latest wins if no compatible version found
        if candidate.is_none() {
            return self.resolve_latest_wins(conflict).await;
        }

        let modified_packages = conflict.source_packages.clone();
        Ok(ConflictResolution {
            package_name: conflict.package_name.clone(),
            selected_version: candidate,
            strategy: "find_compatible".to_string(),
            modified_packages,
        })
    }

    /// Generate a cache key for a conflict
    fn generate_conflict_cache_key(&self, conflict: &DependencyConflict) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        conflict.package_name.hash(&mut hasher);
        for req in &conflict.conflicting_requirements {
            req.name.hash(&mut hasher);
            if let Some(ref spec) = req.version_spec {
                spec.hash(&mut hasher);
            }
        }
        format!("{:x}", hasher.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConflictSeverity, ConflictStrategy};
    use rez_next_package::PackageRequirement;

    /// Helper to create a DependencyConflict for testing
    fn make_conflict(
        name: &str,
        requirements: Vec<(&str, Option<&str>)>,
        source_packages: Vec<&str>,
    ) -> DependencyConflict {
        DependencyConflict {
            package_name: name.to_string(),
            conflicting_requirements: requirements
                .into_iter()
                .map(|(req_name, version_spec)| PackageRequirement {
                    name: req_name.to_string(),
                    version_spec: version_spec.map(|s| s.to_string()),
                    weak: false,
                    conflict: false,
                })
                .collect(),
            source_packages: source_packages.into_iter().map(|s| s.to_string()).collect(),
            severity: ConflictSeverity::Major,
        }
    }

    #[tokio::test]
    async fn test_conflict_resolver_new_latest_wins() {
        let resolver = ConflictResolver::new(ConflictStrategy::LatestWins);
        let conflict = make_conflict(
            "python",
            vec![
                ("python", Some("3.7")),
                ("python", Some("3.9")),
                ("python", Some("3.8")),
            ],
            vec!["pkg_a", "pkg_b"],
        );

        let resolutions = resolver.resolve_conflicts(vec![conflict]).await.unwrap();
        assert_eq!(resolutions.len(), 1);
        let resolution = &resolutions[0];
        assert_eq!(resolution.package_name, "python");
        assert!(resolution.selected_version.is_some());
        // Latest should be 3.9
        let selected = resolution.selected_version.as_ref().unwrap();
        assert_eq!(selected.as_str(), "3.9");
        assert_eq!(resolution.strategy, "latest_wins");
        assert_eq!(resolution.modified_packages.len(), 2);
    }

    #[tokio::test]
    async fn test_conflict_resolver_new_earliest_wins() {
        let resolver = ConflictResolver::new(ConflictStrategy::EarliestWins);
        let conflict = make_conflict(
            "python",
            vec![
                ("python", Some("3.7")),
                ("python", Some("3.9")),
                ("python", Some("3.8")),
            ],
            vec!["pkg_a", "pkg_b"],
        );

        let resolutions = resolver.resolve_conflicts(vec![conflict]).await.unwrap();
        assert_eq!(resolutions.len(), 1);
        let resolution = &resolutions[0];
        assert_eq!(resolution.package_name, "python");
        assert!(resolution.selected_version.is_some());
        // Earliest should be 3.7
        let selected = resolution.selected_version.as_ref().unwrap();
        assert_eq!(selected.as_str(), "3.7");
        assert_eq!(resolution.strategy, "earliest_wins");
    }

    #[tokio::test]
    async fn test_conflict_resolver_fail_on_conflict() {
        let resolver = ConflictResolver::new(ConflictStrategy::FailOnConflict);
        let conflict = make_conflict(
            "python",
            vec![("python", Some("3.7")), ("python", Some("3.9"))],
            vec!["pkg_a", "pkg_b"],
        );

        let result = resolver.resolve_conflicts(vec![conflict]).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Conflict detected"));
    }

    #[tokio::test]
    async fn test_conflict_resolver_find_compatible_success() {
        let resolver = ConflictResolver::new(ConflictStrategy::FindCompatible);
        // All requirements want the same version - should find compatible
        let conflict = make_conflict(
            "python",
            vec![("python", Some("3.9")), ("python", Some("3.9"))],
            vec!["pkg_a", "pkg_b"],
        );

        let resolutions = resolver.resolve_conflicts(vec![conflict]).await.unwrap();
        assert_eq!(resolutions.len(), 1);
        let resolution = &resolutions[0];
        assert_eq!(resolution.package_name, "python");
        assert!(resolution.selected_version.is_some());
        assert_eq!(resolution.strategy, "find_compatible");
    }

    #[tokio::test]
    async fn test_conflict_resolver_find_compatible_fallback() {
        let resolver = ConflictResolver::new(ConflictStrategy::FindCompatible);
        // Incompatible requirements - should fallback to latest_wins
        let conflict = make_conflict(
            "python",
            vec![("python", Some("3.7")), ("python", Some("3.9"))],
            vec!["pkg_a", "pkg_b"],
        );

        let resolutions = resolver.resolve_conflicts(vec![conflict]).await.unwrap();
        assert_eq!(resolutions.len(), 1);
        let resolution = &resolutions[0];
        // When fallback to latest_wins, strategy will be "latest_wins"
        // (the resolve_latest_wins method sets strategy field)
        assert_eq!(resolution.strategy, "latest_wins");
        assert!(resolution.selected_version.is_some());
        // Latest among 3.7 and 3.9 is 3.9
        let selected = resolution.selected_version.as_ref().unwrap();
        assert_eq!(selected.as_str(), "3.9");
    }

    #[tokio::test]
    async fn test_conflict_resolver_empty_version_spec() {
        let resolver = ConflictResolver::new(ConflictStrategy::LatestWins);
        // One requirement has no version spec
        let conflict = make_conflict(
            "python",
            vec![("python", Some("3.9")), ("python", None)],
            vec!["pkg_a"],
        );

        let resolutions = resolver.resolve_conflicts(vec![conflict]).await.unwrap();
        assert_eq!(resolutions.len(), 1);
        // Should still resolve with the version that has a spec
        let resolution = &resolutions[0];
        assert_eq!(resolution.package_name, "python");
    }

    #[tokio::test]
    async fn test_conflict_resolver_multiple_conflicts() {
        let resolver = ConflictResolver::new(ConflictStrategy::LatestWins);
        let conflict1 = make_conflict("python", vec![("python", Some("3.9"))], vec!["pkg_a"]);
        let conflict2 = make_conflict(
            "maya",
            vec![("maya", Some("2024")), ("maya", Some("2023"))],
            vec!["pkg_b", "pkg_c"],
        );

        let resolutions = resolver
            .resolve_conflicts(vec![conflict1, conflict2])
            .await
            .unwrap();
        assert_eq!(resolutions.len(), 2);
        assert_eq!(resolutions[0].package_name, "python");
        assert_eq!(resolutions[1].package_name, "maya");
    }

    #[tokio::test]
    async fn test_conflict_resolver_invalid_version() {
        let resolver = ConflictResolver::new(ConflictStrategy::LatestWins);
        // Version that can't be parsed should be skipped
        let conflict = make_conflict(
            "python",
            vec![("python", Some("3.9")), ("python", Some("invalid_version"))],
            vec!["pkg_a"],
        );

        let resolutions = resolver.resolve_conflicts(vec![conflict]).await.unwrap();
        assert_eq!(resolutions.len(), 1);
        // Should still resolve with the valid version
        let resolution = &resolutions[0];
        assert!(resolution.selected_version.is_some());
    }

    #[tokio::test]
    async fn test_conflict_severity_levels() {
        // Test that different severity levels can be set
        let mut conflict = make_conflict("python", vec![("python", Some("3.9"))], vec!["pkg_a"]);
        conflict.severity = ConflictSeverity::Minor;

        let resolver = ConflictResolver::new(ConflictStrategy::LatestWins);
        let resolutions = resolver.resolve_conflicts(vec![conflict]).await.unwrap();
        assert_eq!(resolutions.len(), 1);

        // Test Major severity
        let mut conflict2 = make_conflict("python", vec![("python", Some("3.9"))], vec!["pkg_a"]);
        conflict2.severity = ConflictSeverity::Major;

        let resolutions = resolver.resolve_conflicts(vec![conflict2]).await.unwrap();
        assert_eq!(resolutions.len(), 1);

        // Test Incompatible severity
        let mut conflict3 = make_conflict("python", vec![("python", Some("3.9"))], vec!["pkg_a"]);
        conflict3.severity = ConflictSeverity::Incompatible;

        let resolutions = resolver.resolve_conflicts(vec![conflict3]).await.unwrap();
        assert_eq!(resolutions.len(), 1);
    }

    #[tokio::test]
    async fn test_conflict_resolver_no_conflicts() {
        let resolver = ConflictResolver::new(ConflictStrategy::LatestWins);

        // Empty conflicts list
        let resolutions = resolver.resolve_conflicts(vec![]).await.unwrap();
        assert!(resolutions.is_empty());
    }

    #[tokio::test]
    async fn test_conflict_resolver_single_requirement() {
        let resolver = ConflictResolver::new(ConflictStrategy::LatestWins);
        let conflict = make_conflict("python", vec![("python", Some("3.9"))], vec!["pkg_a"]);

        let resolutions = resolver.resolve_conflicts(vec![conflict]).await.unwrap();
        assert_eq!(resolutions.len(), 1);
        assert_eq!(resolutions[0].package_name, "python");
        assert!(resolutions[0].selected_version.is_some());
    }

    #[tokio::test]
    async fn test_conflict_resolver_all_same_versions() {
        let resolver = ConflictResolver::new(ConflictStrategy::LatestWins);
        // All requirements want the same version
        let conflict = make_conflict(
            "python",
            vec![("python", Some("3.9")), ("python", Some("3.9"))],
            vec!["pkg_a", "pkg_b"],
        );

        let resolutions = resolver.resolve_conflicts(vec![conflict]).await.unwrap();
        assert_eq!(resolutions.len(), 1);
        let selected = resolutions[0].selected_version.as_ref().unwrap();
        assert_eq!(selected.as_str(), "3.9");
    }

    #[test]
    fn test_conflict_severity_equality() {
        assert_eq!(ConflictSeverity::Minor, ConflictSeverity::Minor);
        assert_eq!(ConflictSeverity::Major, ConflictSeverity::Major);
        assert_eq!(
            ConflictSeverity::Incompatible,
            ConflictSeverity::Incompatible
        );
        assert_ne!(ConflictSeverity::Minor, ConflictSeverity::Major);
    }

    #[test]
    fn test_conflict_strategy_equality() {
        assert_eq!(ConflictStrategy::LatestWins, ConflictStrategy::LatestWins);
        assert_eq!(
            ConflictStrategy::EarliestWins,
            ConflictStrategy::EarliestWins
        );
        assert_eq!(
            ConflictStrategy::FailOnConflict,
            ConflictStrategy::FailOnConflict
        );
        assert_eq!(
            ConflictStrategy::FindCompatible,
            ConflictStrategy::FindCompatible
        );
        assert_ne!(ConflictStrategy::LatestWins, ConflictStrategy::EarliestWins);
    }

    #[test]
    fn test_conflict_resolution_debug() {
        let resolution = ConflictResolution {
            package_name: "python".to_string(),
            selected_version: Some(rez_next_version::Version::parse("3.9").unwrap()),
            strategy: "latest_wins".to_string(),
            modified_packages: vec!["pkg_a".to_string()],
        };

        let debug_output = format!("{:?}", resolution);
        assert!(debug_output.contains("python"));
        assert!(debug_output.contains("latest_wins"));
    }

    #[test]
    fn test_conflict_resolution_clone() {
        let resolution = ConflictResolution {
            package_name: "python".to_string(),
            selected_version: Some(rez_next_version::Version::parse("3.9").unwrap()),
            strategy: "latest_wins".to_string(),
            modified_packages: vec!["pkg_a".to_string()],
        };

        let cloned = resolution.clone();
        assert_eq!(cloned.package_name, resolution.package_name);
        assert_eq!(cloned.strategy, resolution.strategy);
    }
}
