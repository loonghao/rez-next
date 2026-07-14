//! Dependency resolution implementation - equivalent to Python's solver

use crate::SolverConfig;
use crate::resolution_state::ResolutionState;
use rez_next_common::RezCoreError;
use rez_next_package::{Package, Requirement, VersionConstraint};
use rez_next_repository::simple_repository::RepositoryManager;
use rez_next_version::Version;
use std::collections::HashMap;
use std::path::PathBuf;
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

impl ResolvedPackageInfo {
    /// Return the package descriptor rooted at its selected variant payload.
    pub fn materialized_package(&self) -> Package {
        let mut package = (*self.package).clone();
        if package.hashed_variants != Some(true)
            && let Some(index) = self.variant_index
            && let (Some(root), Some(requirements)) = (package.root(), package.variants.get(index))
        {
            let variant_root = requirements
                .iter()
                .fold(PathBuf::from(root), |path, requirement| {
                    path.join(requirement)
                });
            package.filepath = Some(
                variant_root
                    .join("package.py")
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        package
    }
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

        let failed_requirements = std::mem::take(&mut resolution_state.failed_requirements);
        let failed: Vec<_> = failed_requirements
            .into_iter()
            .filter(|requirement| {
                resolution_state.is_requirement_active(requirement)
                    && resolution_state
                        .find_satisfying_package(requirement)
                        .is_none()
            })
            .collect();

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

            // Weak requirements constrain a package when it is otherwise part
            // of the solve, but never introduce that package on their own.
            if requirement.weak
                && !state
                    .resolved_packages
                    .iter()
                    .any(|resolved| resolved.package.name == requirement.name)
            {
                continue;
            }

            // Find candidate packages for this requirement
            let candidates = self.find_candidate_packages(&requirement).await?;
            state.packages_considered += candidates.len();
            let has_resolved_package = state
                .resolved_packages
                .iter()
                .any(|resolved| resolved.package.name == requirement.name);

            if candidates.is_empty() {
                state.failed_requirements.push(requirement.clone());
                continue;
            }

            // Try each candidate
            let mut resolved = false;
            let mut best_effort_candidate = None;
            for candidate in candidates {
                if state.is_package_rejected(&candidate) {
                    continue;
                }
                if best_effort_candidate.is_none()
                    && state.check_explicit_conflicts(&candidate).is_none()
                {
                    best_effort_candidate = Some(candidate.clone());
                }
                // Check for conflicts with existing packages
                if let Some(conflict) = state.check_conflicts(&candidate, &requirement) {
                    state.conflicts.push(conflict);
                    continue;
                }

                // Try to resolve with this candidate
                if let Ok((resolved_info, dependencies, conflicts)) = self
                    .try_resolve_with_candidate(state, &candidate, &requirement)
                    .await
                {
                    let package_name = resolved_info.package.name.clone();
                    state.add_resolved_package(resolved_info);
                    state.set_package_requirements(package_name, dependencies, conflicts);
                    resolved = true;
                    break;
                }
            }

            if !resolved
                && !self.config.strict_mode
                && !has_resolved_package
                && let Some(candidate) = best_effort_candidate
                && let Ok((resolved_info, dependencies, conflicts)) = self
                    .try_resolve_with_candidate(state, &candidate, &requirement)
                    .await
            {
                let package_name = resolved_info.package.name.clone();
                state.add_resolved_package(resolved_info);
                state.set_package_requirements(package_name, dependencies, conflicts);
                resolved = true;
            }

            if !resolved
                && self.config.strict_mode
                && self
                    .backtrack_dependency_source(state, &requirement)
                    .await?
            {
                continue;
            }

            if !resolved {
                state.failed_requirements.push(requirement);
            }
        }

        Ok(state.resolved_packages.clone())
    }

    async fn backtrack_dependency_source(
        &mut self,
        state: &mut ResolutionState,
        failed_requirement: &Requirement,
    ) -> Result<bool, RezCoreError> {
        let target_candidates = self.find_candidate_packages(failed_requirement).await?;
        let mut exact_sources = Vec::new();
        let mut incompatible_sources = Vec::new();
        let mut other_sources = Vec::new();

        for resolved in &state.resolved_packages {
            let source = &resolved.package.name;
            let Some(requirements) = state.package_requirements.get(source) else {
                continue;
            };
            let target_requirements: Vec<_> = requirements
                .iter()
                .filter(|requirement| requirement.name == failed_requirement.name)
                .collect();
            if target_requirements.is_empty() {
                continue;
            }

            if target_requirements.contains(&failed_requirement) {
                exact_sources.push(source.clone());
            } else if target_requirements.iter().any(|requirement| {
                !target_candidates.iter().any(|candidate| {
                    candidate
                        .version
                        .as_ref()
                        .is_some_and(|version| requirement.is_satisfied_by(version))
                })
            }) {
                incompatible_sources.push(source.clone());
            } else {
                other_sources.push(source.clone());
            }
        }

        exact_sources.extend(incompatible_sources);
        exact_sources.extend(other_sources);
        for source in exact_sources {
            if self.backtrack_package(state, &source).await? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn backtrack_package(
        &mut self,
        state: &mut ResolutionState,
        package_name: &str,
    ) -> Result<bool, RezCoreError> {
        let Some(current) = state
            .resolved_packages
            .iter()
            .find(|resolved| resolved.package.name == package_name)
            .cloned()
        else {
            return Ok(false);
        };
        let Some(requirements) = state.active_requirements.get(package_name).cloned() else {
            return Ok(false);
        };
        let Some(primary_requirement) = requirements.first() else {
            return Ok(false);
        };

        state.reject_package(&current.package);
        let candidates = self.find_candidate_packages(primary_requirement).await?;
        for candidate in candidates {
            if state.is_package_rejected(&candidate)
                || candidate.version == current.package.version
                || candidate.version.as_ref().is_some_and(|version| {
                    requirements
                        .iter()
                        .any(|requirement| !requirement.is_satisfied_by(version))
                })
                || state.check_explicit_conflicts(&candidate).is_some()
            {
                continue;
            }

            let requirement = current
                .satisfying_requirement
                .as_ref()
                .unwrap_or(primary_requirement);
            if let Ok((resolved_info, dependencies, conflicts)) = self
                .try_resolve_with_candidate(state, &candidate, requirement)
                .await
            {
                state.add_resolved_package(resolved_info);
                state.set_package_requirements(package_name.to_string(), dependencies, conflicts);
                state.backtrack_steps += 1;
                return Ok(true);
            }
        }

        Ok(false)
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
    ) -> Result<(ResolvedPackageInfo, Vec<Requirement>, Vec<Requirement>), RezCoreError> {
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

        let mut dependencies = Vec::new();
        let mut conflicts = Vec::new();
        for dep_req_str in &effective_requires {
            let (conflict, dep_requirement) = Self::parse_variant_requirement(dep_req_str)
                .map_err(|e| {
                    RezCoreError::RequirementParse(format!(
                        "Invalid requirement '{}': {}",
                        dep_req_str, e
                    ))
                })?;
            if conflict {
                conflicts.push(dep_requirement);
                continue;
            }
            dependencies.push(dep_requirement);
        }

        Ok((resolved_info, dependencies, conflicts))
    }

    /// Select the best variant for a package given current resolution state
    fn select_variant(&self, package: &Package, state: &ResolutionState) -> Option<usize> {
        if package.variants.is_empty() {
            return None;
        }

        package
            .variants
            .iter()
            .enumerate()
            .filter(|(_, requirements)| self.variant_is_compatible(requirements, state))
            .max_by_key(|(index, requirements)| {
                let positive: Vec<_> = requirements
                    .iter()
                    .filter_map(|raw| Self::parse_variant_requirement(raw).ok())
                    .filter(|(conflict, _)| !conflict)
                    .map(|(_, requirement)| requirement)
                    .collect();
                let resolved_matches = positive
                    .iter()
                    .filter(|requirement| {
                        state.resolved_packages.iter().any(|resolved| {
                            resolved.package.name == requirement.name
                                && resolved
                                    .package
                                    .version
                                    .as_ref()
                                    .is_some_and(|version| requirement.is_satisfied_by(version))
                        })
                    })
                    .count();
                let shared = positive
                    .iter()
                    .filter(|requirement| {
                        state
                            .original_requirements
                            .iter()
                            .any(|original| original.name == requirement.name)
                    })
                    .count();
                let resolved_version_priority: Vec<_> = positive
                    .iter()
                    .filter(|requirement| {
                        state
                            .resolved_packages
                            .iter()
                            .any(|resolved| resolved.package.name == requirement.name)
                    })
                    .map(|requirement| {
                        (
                            requirement
                                .version_constraint
                                .as_ref()
                                .and_then(Self::constraint_version_floor),
                            requirement.name.clone(),
                        )
                    })
                    .collect();
                (
                    resolved_matches,
                    shared,
                    std::cmp::Reverse(positive.len()),
                    resolved_version_priority,
                    std::cmp::Reverse(*index),
                )
            })
            .map(|(index, _)| index)
    }

    fn constraint_version_floor(constraint: &VersionConstraint) -> Option<Version> {
        match constraint {
            VersionConstraint::Exact(version)
            | VersionConstraint::GreaterThan(version)
            | VersionConstraint::GreaterThanOrEqual(version)
            | VersionConstraint::Compatible(version)
            | VersionConstraint::Range(version, _)
            | VersionConstraint::Prefix(version) => Some(version.clone()),
            VersionConstraint::Multiple(constraints)
            | VersionConstraint::Alternative(constraints) => constraints
                .iter()
                .filter_map(Self::constraint_version_floor)
                .max(),
            VersionConstraint::LessThan(_)
            | VersionConstraint::LessThanOrEqual(_)
            | VersionConstraint::Exclude(_)
            | VersionConstraint::Wildcard(_)
            | VersionConstraint::Any => None,
        }
    }

    fn variant_is_compatible(&self, requirements: &[String], state: &ResolutionState) -> bool {
        requirements.iter().all(|raw| {
            let Ok((conflict, requirement)) = Self::parse_variant_requirement(raw) else {
                return false;
            };
            let resolved = state
                .resolved_packages
                .iter()
                .find(|resolved| resolved.package.name == requirement.name);
            match (conflict, resolved) {
                (true, Some(resolved)) => resolved
                    .package
                    .version
                    .as_ref()
                    .is_some_and(|version| !requirement.is_satisfied_by(version)),
                // A later positive constraint may legitimately narrow an already
                // selected package. The queue will replace it with a candidate
                // satisfying every active constraint.
                (false, Some(_)) => true,
                _ => true,
            }
        })
    }

    fn parse_variant_requirement(raw: &str) -> Result<(bool, Requirement), String> {
        let (conflict, requirement) = raw
            .strip_prefix('!')
            .map_or((false, raw), |requirement| (true, requirement));
        requirement
            .parse()
            .map(|requirement| (conflict, requirement))
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
