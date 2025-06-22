//! Conflict detection and resolution

use crate::{ConflictResolution, ConflictStrategy, DependencyConflict};
use rez_core_common::RezCoreError;
use rez_core_package::PackageRequirement;
use rez_core_version::{Version, VersionRange};
use serde::{Deserialize, Serialize};
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
                return Err(RezCoreError::SolverError(format!(
                    "Conflict detected for package {}: {:?}",
                    conflict.package_name, conflict.conflicting_requirements
                )));
            }
            ConflictStrategy::FindCompatible => self.resolve_find_compatible(conflict).await?,
        };

        Ok(resolution)
    }

    /// Resolve conflict by selecting the latest version
    async fn resolve_latest_wins(
        &self,
        conflict: &DependencyConflict,
    ) -> Result<ConflictResolution, RezCoreError> {
        let mut latest_version: Option<Version> = None;
        let mut modified_packages = Vec::new();

        // Find the latest version among all requirements
        for requirement in &conflict.conflicting_requirements {
            if let Some(ref range) = requirement.range {
                if let Some(max_version) = range.get_max_version() {
                    match &latest_version {
                        Some(current_latest) => {
                            if max_version > *current_latest {
                                latest_version = Some(max_version);
                            }
                        }
                        None => latest_version = Some(max_version),
                    }
                }
            }
        }

        // Mark source packages as modified
        modified_packages.extend(conflict.source_packages.iter().cloned());

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
        let mut modified_packages = Vec::new();

        // Find the earliest version among all requirements
        for requirement in &conflict.conflicting_requirements {
            if let Some(ref range) = requirement.range {
                if let Some(min_version) = range.get_min_version() {
                    match &earliest_version {
                        Some(current_earliest) => {
                            if min_version < *current_earliest {
                                earliest_version = Some(min_version);
                            }
                        }
                        None => earliest_version = Some(min_version),
                    }
                }
            }
        }

        // Mark source packages as modified
        modified_packages.extend(conflict.source_packages.iter().cloned());

        Ok(ConflictResolution {
            package_name: conflict.package_name.clone(),
            selected_version: earliest_version,
            strategy: "earliest_wins".to_string(),
            modified_packages,
        })
    }

    /// Resolve conflict by finding a compatible version
    async fn resolve_find_compatible(
        &self,
        conflict: &DependencyConflict,
    ) -> Result<ConflictResolution, RezCoreError> {
        // Try to find a version that satisfies all requirements
        let compatible_range = self.find_compatible_range(&conflict.conflicting_requirements)?;

        let selected_version = if let Some(range) = compatible_range {
            // Select the latest version within the compatible range
            range.get_max_version()
        } else {
            // No compatible range found, fall back to latest wins
            return self.resolve_latest_wins(conflict).await;
        };

        let mut modified_packages = Vec::new();
        modified_packages.extend(conflict.source_packages.iter().cloned());

        Ok(ConflictResolution {
            package_name: conflict.package_name.clone(),
            selected_version,
            strategy: "find_compatible".to_string(),
            modified_packages,
        })
    }

    /// Find a version range that is compatible with all requirements
    fn find_compatible_range(
        &self,
        requirements: &[PackageRequirement],
    ) -> Result<Option<VersionRange>, RezCoreError> {
        if requirements.is_empty() {
            return Ok(None);
        }

        let mut compatible_range: Option<VersionRange> = None;

        for requirement in requirements {
            if let Some(ref range) = requirement.range {
                match compatible_range {
                    Some(ref current_range) => {
                        if let Some(intersection) = current_range.intersect(range) {
                            compatible_range = Some(intersection);
                        } else {
                            // No intersection found
                            return Ok(None);
                        }
                    }
                    None => compatible_range = Some(range.clone()),
                }
            }
        }

        Ok(compatible_range)
    }

    /// Generate a cache key for a conflict
    fn generate_conflict_cache_key(&self, conflict: &DependencyConflict) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        conflict.package_name.hash(&mut hasher);
        for req in &conflict.conflicting_requirements {
            req.requirement_string.hash(&mut hasher);
        }
        conflict.source_packages.hash(&mut hasher);

        format!("conflict_{}_{:x}", conflict.package_name, hasher.finish())
    }

    /// Clear the resolution cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> ConflictResolverStats {
        ConflictResolverStats {
            cache_size: self.cache.len(),
            strategy: self.strategy.clone(),
        }
    }
}

/// Conflict resolver statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolverStats {
    /// Number of cached resolutions
    pub cache_size: usize,
    /// Current resolution strategy
    pub strategy: ConflictStrategy,
}

/// Conflict analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictAnalysis {
    /// Total number of conflicts
    pub total_conflicts: usize,
    /// Conflicts by severity
    pub conflicts_by_severity: HashMap<String, usize>,
    /// Most conflicted packages
    pub most_conflicted_packages: Vec<String>,
    /// Conflict resolution suggestions
    pub suggestions: Vec<ConflictSuggestion>,
}

/// Conflict resolution suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictSuggestion {
    /// Package name
    pub package_name: String,
    /// Suggested action
    pub action: SuggestionAction,
    /// Reason for the suggestion
    pub reason: String,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,
}

/// Types of conflict resolution suggestions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SuggestionAction {
    /// Upgrade to a specific version
    Upgrade(String),
    /// Downgrade to a specific version
    Downgrade(String),
    /// Remove the package
    Remove,
    /// Add a constraint
    AddConstraint(String),
    /// Change resolution strategy
    ChangeStrategy(ConflictStrategy),
}

impl ConflictResolver {
    /// Analyze conflicts and provide suggestions
    pub fn analyze_conflicts(&self, conflicts: &[DependencyConflict]) -> ConflictAnalysis {
        let mut conflicts_by_severity = HashMap::new();
        let mut package_conflict_count: HashMap<String, usize> = HashMap::new();
        let mut suggestions = Vec::new();

        // Count conflicts by severity
        for conflict in conflicts {
            let severity_str = format!("{:?}", conflict.severity);
            *conflicts_by_severity.entry(severity_str).or_insert(0) += 1;

            // Count conflicts per package
            *package_conflict_count
                .entry(conflict.package_name.clone())
                .or_insert(0) += 1;

            // Generate suggestions for this conflict
            let conflict_suggestions = self.generate_suggestions_for_conflict(conflict);
            suggestions.extend(conflict_suggestions);
        }

        // Find most conflicted packages
        let mut most_conflicted: Vec<_> = package_conflict_count.into_iter().collect();
        most_conflicted.sort_by(|a, b| b.1.cmp(&a.1));
        let most_conflicted_packages: Vec<String> = most_conflicted
            .into_iter()
            .take(5)
            .map(|(name, _)| name)
            .collect();

        ConflictAnalysis {
            total_conflicts: conflicts.len(),
            conflicts_by_severity,
            most_conflicted_packages,
            suggestions,
        }
    }

    /// Generate suggestions for a specific conflict
    fn generate_suggestions_for_conflict(
        &self,
        conflict: &DependencyConflict,
    ) -> Vec<ConflictSuggestion> {
        let mut suggestions = Vec::new();

        // Suggest upgrading to latest compatible version
        if let Some(latest_version) =
            self.find_latest_compatible_version(&conflict.conflicting_requirements)
        {
            suggestions.push(ConflictSuggestion {
                package_name: conflict.package_name.clone(),
                action: SuggestionAction::Upgrade(latest_version.as_str().to_string()),
                reason: "Upgrade to latest compatible version".to_string(),
                confidence: 0.8,
            });
        }

        // Suggest changing strategy for incompatible conflicts
        if conflict.severity == crate::ConflictSeverity::Incompatible {
            suggestions.push(ConflictSuggestion {
                package_name: conflict.package_name.clone(),
                action: SuggestionAction::ChangeStrategy(ConflictStrategy::FindCompatible),
                reason: "Try to find compatible version range".to_string(),
                confidence: 0.6,
            });
        }

        suggestions
    }

    /// Find the latest version that is compatible with all requirements
    fn find_latest_compatible_version(
        &self,
        requirements: &[PackageRequirement],
    ) -> Option<Version> {
        if let Ok(Some(compatible_range)) = self.find_compatible_range(requirements) {
            compatible_range.get_max_version()
        } else {
            None
        }
    }
}

impl Default for ConflictResolver {
    fn default() -> Self {
        Self::new(ConflictStrategy::LatestWins)
    }
}
