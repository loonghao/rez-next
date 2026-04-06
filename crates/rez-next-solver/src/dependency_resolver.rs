//! Dependency resolution implementation - equivalent to Python's solver

use crate::resolution_state::ResolutionState;
use crate::SolverConfig;
use rez_next_common::RezCoreError;
use rez_next_package::{Package, Requirement};
use rez_next_repository::simple_repository::RepositoryManager;
use std::collections::HashMap;
use std::sync::Arc;

/// A dependency resolver that finds compatible package combinations
pub struct DependencyResolver {
    /// Repository manager for package discovery
    repository_manager: Arc<RepositoryManager>,

    /// Solver configuration
    config: SolverConfig,

    /// Cache of resolved packages
    package_cache: HashMap<String, Vec<Arc<Package>>>,
}

/// Detailed resolution result containing resolved packages, failures, conflicts and stats
#[derive(Debug, Clone)]
pub struct DetailedResolutionResult {
    /// Successfully resolved packages
    pub resolved_packages: Vec<ResolvedPackageInfo>,

    /// Requirements that couldn't be satisfied
    pub failed_requirements: Vec<Requirement>,

    /// Conflicts encountered during resolution
    pub conflicts: Vec<ResolutionConflict>,

    /// Resolution statistics
    pub stats: ResolutionStats,
}

/// Information about a resolved package
#[derive(Debug, Clone)]
pub struct ResolvedPackageInfo {
    /// The resolved package
    pub package: Arc<Package>,

    /// The variant index if this package has variants
    pub variant_index: Option<usize>,

    /// Whether this package was explicitly requested
    pub requested: bool,

    /// The requirements that led to this package being included
    pub required_by: Vec<String>,

    /// The specific requirement that was satisfied
    pub satisfying_requirement: Option<Requirement>,
}

/// A conflict encountered during resolution
#[derive(Debug, Clone, serde::Serialize)]
pub struct ResolutionConflict {
    /// The package name that has conflicting requirements
    pub package_name: String,

    /// The conflicting requirements
    pub conflicting_requirements: Vec<Requirement>,

    /// The packages that introduced these requirements
    pub source_packages: Vec<String>,
}

/// Statistics about the resolution process
#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct ResolutionStats {
    /// Number of packages considered
    pub packages_considered: usize,

    /// Number of variants evaluated
    pub variants_evaluated: usize,

    /// Time spent resolving (in milliseconds)
    pub resolution_time_ms: u64,

    /// Number of conflicts encountered
    pub conflicts_encountered: usize,

    /// Number of backtracking steps
    pub backtrack_steps: usize,
}

impl DependencyResolver {
    /// Create a new dependency resolver
    pub fn new(repository_manager: Arc<RepositoryManager>, config: SolverConfig) -> Self {
        Self {
            repository_manager,
            config,
            package_cache: HashMap::new(),
        }
    }

    /// Resolve a set of requirements into a consistent package set
    pub async fn resolve(
        &mut self,
        requirements: Vec<Requirement>,
    ) -> Result<DetailedResolutionResult, RezCoreError> {
        let start_time = std::time::Instant::now();

        // Initialize resolution state
        let mut resolution_state = ResolutionState::new(requirements.clone());

        // Perform resolution
        let result = self.resolve_recursive(&mut resolution_state).await?;

        // Check for cyclic dependencies after full resolution
        if let Some(cycle) = resolution_state.detect_cycle() {
            return Err(RezCoreError::Solver(format!(
                "Cyclic dependency detected: {}",
                cycle.join(" -> ")
            )));
        }

        // Calculate statistics
        let resolution_time = start_time.elapsed().as_millis() as u64;
        let stats = ResolutionStats {
            packages_considered: resolution_state.packages_considered,
            variants_evaluated: resolution_state.variants_evaluated,
            resolution_time_ms: resolution_time,
            conflicts_encountered: resolution_state.conflicts.len(),
            backtrack_steps: resolution_state.backtrack_steps,
        };

        let failed = resolution_state.failed_requirements;

        // Strict mode: any unsatisfied requirement is a hard error
        if self.config.strict_mode && !failed.is_empty() {
            let names: Vec<String> = failed.iter().map(|r| r.to_string()).collect();
            return Err(RezCoreError::Solver(format!(
                "Strict mode: failed to satisfy requirements: {}",
                names.join(", ")
            )));
        }

        Ok(DetailedResolutionResult {
            resolved_packages: result,
            failed_requirements: failed,
            conflicts: resolution_state.conflicts,
            stats,
        })
    }

    /// Recursive resolution implementation
    async fn resolve_recursive(
        &mut self,
        state: &mut ResolutionState,
    ) -> Result<Vec<ResolvedPackageInfo>, RezCoreError> {
        // Get next requirement to resolve
        while let Some(requirement) = state.get_next_requirement() {
            // Check if we already have a package that satisfies this requirement
            if let Some(existing) = state.find_satisfying_package(&requirement) {
                // Mark this requirement as satisfied
                state.mark_requirement_satisfied(&requirement, existing.package.name.clone());
                continue;
            }

            // Find candidate packages for this requirement
            let candidates = self.find_candidate_packages(&requirement).await?;
            state.packages_considered += candidates.len();

            if candidates.is_empty() {
                state.failed_requirements.push(requirement.clone());
                continue;
            }

            // Try each candidate
            let mut resolved = false;
            for candidate in candidates {
                // Check for conflicts with existing packages
                if let Some(conflict) = state.check_conflicts(&candidate, &requirement) {
                    state.conflicts.push(conflict);
                    continue;
                }

                // Try to resolve with this candidate
                if let Ok(resolved_info) = self
                    .try_resolve_with_candidate(state, &candidate, &requirement)
                    .await
                {
                    state.add_resolved_package(resolved_info);
                    resolved = true;
                    break;
                }
            }

            if !resolved {
                state.failed_requirements.push(requirement);
            }
        }

        Ok(state.resolved_packages.clone())
    }

    /// Find candidate packages that could satisfy a requirement
    async fn find_candidate_packages(
        &mut self,
        requirement: &Requirement,
    ) -> Result<Vec<Arc<Package>>, RezCoreError> {
        // Check cache first
        if let Some(cached) = self.package_cache.get(&requirement.name) {
            return Ok(self.filter_and_sort_candidates(cached, requirement));
        }

        // Search repositories
        let packages = self
            .repository_manager
            .find_packages(&requirement.name)
            .await?;

        // Cache the results
        self.package_cache
            .insert(requirement.name.clone(), packages.clone());

        Ok(self.filter_and_sort_candidates(&packages, requirement))
    }

    /// Filter candidate packages based on version constraints and sort them
    fn filter_and_sort_candidates(
        &self,
        packages: &[Arc<Package>],
        requirement: &Requirement,
    ) -> Vec<Arc<Package>> {
        let mut candidates: Vec<Arc<Package>> = packages
            .iter()
            .filter(|pkg| {
                if let Some(ref version) = pkg.version {
                    // Respect allow_prerelease flag
                    if !self.config.allow_prerelease && version.is_prerelease() {
                        return false;
                    }
                    requirement.is_satisfied_by(version)
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        // Sort by version: latest first (prefer latest behavior)
        if self.config.prefer_latest {
            candidates.sort_by(|a, b| match (&b.version, &a.version) {
                (Some(bv), Some(av)) => bv.cmp(av),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            });
        } else {
            // oldest first
            candidates.sort_by(|a, b| match (&a.version, &b.version) {
                (Some(av), Some(bv)) => av.cmp(bv),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            });
        }

        candidates
    }

    /// Try to resolve using a specific candidate package
    async fn try_resolve_with_candidate(
        &mut self,
        state: &mut ResolutionState,
        candidate: &Arc<Package>,
        requirement: &Requirement,
    ) -> Result<ResolvedPackageInfo, RezCoreError> {
        // Determine which variant to use (if package has variants)
        let variant_index = self.select_variant(candidate, state);
        state.variants_evaluated += candidate.variants.len().max(1);

        // Get the effective requires list (base + variant-specific)
        let effective_requires = self.get_effective_requires(candidate, variant_index);

        // Create resolved package info
        let resolved_info = ResolvedPackageInfo {
            package: candidate.clone(),
            variant_index,
            requested: state.is_original_requirement(requirement),
            required_by: vec![requirement.name.clone()],
            satisfying_requirement: Some(requirement.clone()),
        };

        // Add transitive dependencies to resolution queue
        for dep_req_str in &effective_requires {
            let dep_requirement: Requirement = dep_req_str.parse().map_err(|e| {
                RezCoreError::RequirementParse(format!(
                    "Invalid requirement '{}': {}",
                    dep_req_str, e
                ))
            })?;
            // Record the dependency edge for cycle detection
            state.record_dependency(&candidate.name, &dep_requirement.name);
            state.add_requirement(dep_requirement);
        }

        Ok(resolved_info)
    }

    /// Select the best variant for a package given current resolution state
    fn select_variant(&self, package: &Package, state: &ResolutionState) -> Option<usize> {
        if package.variants.is_empty() {
            return None;
        }

        // Try each variant, pick the first one that doesn't conflict with resolved packages
        for (i, variant_requires) in package.variants.iter().enumerate() {
            let mut compatible = true;
            for req_str in variant_requires {
                if let Ok(req) = req_str.parse::<Requirement>() {
                    // Check if this variant requirement conflicts with already resolved packages
                    for resolved in &state.resolved_packages {
                        if resolved.package.name == req.name {
                            if let Some(ref version) = resolved.package.version {
                                if !req.is_satisfied_by(version) {
                                    compatible = false;
                                    break;
                                }
                            }
                        }
                    }
                }
                if !compatible {
                    break;
                }
            }
            if compatible {
                return Some(i);
            }
        }

        // Fall back to first variant
        Some(0)
    }

    /// Get the effective requires list, merging base requires with variant requires
    fn get_effective_requires(
        &self,
        package: &Package,
        variant_index: Option<usize>,
    ) -> Vec<String> {
        let mut requires = package.requires.clone();

        if let Some(idx) = variant_index {
            if let Some(variant_reqs) = package.variants.get(idx) {
                // Variant requires are appended to base requires
                for vreq in variant_reqs {
                    if !requires.contains(vreq) {
                        requires.push(vreq.clone());
                    }
                }
            }
        }

        requires
    }
}

