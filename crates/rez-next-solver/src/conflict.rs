//! Conflict detection and resolution

use crate::{ConflictResolution, ConflictStrategy, DependencyConflict};
use rez_next_common::RezCoreError;
use rez_next_package::PackageRequirement;
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
            if let Some(ref version_spec) = requirement.version_spec {
                if let Ok(v) = Version::parse(version_spec) {
                    match &latest_version {
                        Some(current) if v > *current => latest_version = Some(v),
                        None => latest_version = Some(v),
                        _ => {}
                    }
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
            if let Some(ref version_spec) = requirement.version_spec {
                if let Ok(v) = Version::parse(version_spec) {
                    match &earliest_version {
                        Some(current) if v < *current => earliest_version = Some(v),
                        None => earliest_version = Some(v),
                        _ => {}
                    }
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
            if let Some(ref version_spec) = requirement.version_spec {
                if let Ok(v) = Version::parse(version_spec) {
                    // Check if this version satisfies all other requirements
                    let satisfies_all =
                        conflict.conflicting_requirements.iter().all(|other_req| {
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
