//! Dependency resolution implementation - equivalent to Python's solver

use crate::{SolverConfig, SolverStats};
use rez_core_common::RezCoreError;
use rez_core_package::{Package, Requirement, VersionConstraint};
use rez_core_repository::simple_repository::{PackageRepository, RepositoryManager};
use rez_core_version::Version;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

/// A dependency resolver that finds compatible package combinations
pub struct DependencyResolver {
    /// Repository manager for package discovery
    repository_manager: Arc<RepositoryManager>,

    /// Solver configuration
    config: SolverConfig,

    /// Solver statistics
    stats: SolverStats,

    /// Cache of resolved packages
    package_cache: HashMap<String, Vec<Arc<Package>>>,
}

/// Resolution result containing resolved packages and metadata
#[derive(Debug, Clone)]
pub struct ResolutionResult {
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
#[derive(Debug, Clone, serde::Serialize)]
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
            stats: SolverStats::default(),
            package_cache: HashMap::new(),
        }
    }

    /// Resolve a set of requirements into a consistent package set
    pub async fn resolve(
        &mut self,
        requirements: Vec<Requirement>,
    ) -> Result<ResolutionResult, RezCoreError> {
        let start_time = std::time::Instant::now();

        // Initialize resolution state
        let mut resolution_state = ResolutionState::new(requirements.clone());

        // Perform resolution
        let result = self.resolve_recursive(&mut resolution_state).await?;

        // Calculate statistics
        let resolution_time = start_time.elapsed().as_millis() as u64;
        let stats = ResolutionStats {
            packages_considered: resolution_state.packages_considered,
            variants_evaluated: resolution_state.variants_evaluated,
            resolution_time_ms: resolution_time,
            conflicts_encountered: resolution_state.conflicts.len(),
            backtrack_steps: resolution_state.backtrack_steps,
        };

        Ok(ResolutionResult {
            resolved_packages: result,
            failed_requirements: resolution_state.failed_requirements,
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
            return Ok(self.filter_candidates(cached, requirement));
        }

        // Search repositories
        let packages = self
            .repository_manager
            .find_packages(&requirement.name)
            .await?;

        // Cache the results
        self.package_cache
            .insert(requirement.name.clone(), packages.clone());

        Ok(self.filter_candidates(&packages, requirement))
    }

    /// Filter candidate packages based on version constraints
    fn filter_candidates(
        &self,
        packages: &[Arc<Package>],
        requirement: &Requirement,
    ) -> Vec<Arc<Package>> {
        packages
            .iter()
            .filter(|pkg| {
                if let Some(ref version) = pkg.version {
                    requirement.is_satisfied_by(version)
                } else {
                    // Package without version satisfies any requirement
                    true
                }
            })
            .cloned()
            .collect()
    }

    /// Try to resolve using a specific candidate package
    async fn try_resolve_with_candidate(
        &mut self,
        state: &mut ResolutionState,
        candidate: &Arc<Package>,
        requirement: &Requirement,
    ) -> Result<ResolvedPackageInfo, RezCoreError> {
        // Create resolved package info
        let resolved_info = ResolvedPackageInfo {
            package: candidate.clone(),
            variant_index: None, // TODO: Handle variants
            requested: state.is_original_requirement(requirement),
            required_by: vec![requirement.name.clone()],
            satisfying_requirement: Some(requirement.clone()),
        };

        // Add transitive dependencies to resolution queue
        for dep_req_str in &candidate.requires {
            let dep_requirement: Requirement = dep_req_str.parse().map_err(|e| {
                RezCoreError::RequirementParse(format!(
                    "Invalid requirement '{}': {}",
                    dep_req_str, e
                ))
            })?;
            state.add_requirement(dep_requirement);
        }

        Ok(resolved_info)
    }
}

/// Internal state for the resolution process
#[derive(Debug)]
struct ResolutionState {
    /// Original requirements to resolve
    original_requirements: Vec<Requirement>,

    /// Queue of requirements to process
    requirement_queue: VecDeque<Requirement>,

    /// Successfully resolved packages
    resolved_packages: Vec<ResolvedPackageInfo>,

    /// Requirements that couldn't be satisfied
    failed_requirements: Vec<Requirement>,

    /// Conflicts encountered
    conflicts: Vec<ResolutionConflict>,

    /// Satisfied requirements (to avoid duplicates)
    satisfied_requirements: HashSet<String>,

    /// Statistics
    packages_considered: usize,
    variants_evaluated: usize,
    backtrack_steps: usize,
}

impl ResolutionState {
    fn new(requirements: Vec<Requirement>) -> Self {
        let mut queue = VecDeque::new();
        for req in &requirements {
            queue.push_back(req.clone());
        }

        Self {
            original_requirements: requirements,
            requirement_queue: queue,
            resolved_packages: Vec::new(),
            failed_requirements: Vec::new(),
            conflicts: Vec::new(),
            satisfied_requirements: HashSet::new(),
            packages_considered: 0,
            variants_evaluated: 0,
            backtrack_steps: 0,
        }
    }

    fn get_next_requirement(&mut self) -> Option<Requirement> {
        self.requirement_queue.pop_front()
    }

    fn add_requirement(&mut self, requirement: Requirement) {
        let req_key = format!(
            "{}:{}",
            requirement.name,
            requirement
                .version_constraint
                .as_ref()
                .map(|v| format!("{:?}", v))
                .unwrap_or_default()
        );
        if !self.satisfied_requirements.contains(&req_key) {
            self.requirement_queue.push_back(requirement);
        }
    }

    fn mark_requirement_satisfied(&mut self, requirement: &Requirement, package_name: String) {
        let req_key = format!(
            "{}:{}",
            requirement.name,
            requirement
                .version_constraint
                .as_ref()
                .map(|v| format!("{:?}", v))
                .unwrap_or_default()
        );
        self.satisfied_requirements.insert(req_key);
    }

    fn find_satisfying_package(&self, requirement: &Requirement) -> Option<&ResolvedPackageInfo> {
        self.resolved_packages.iter().find(|pkg| {
            pkg.package.name == requirement.name
                && pkg
                    .package
                    .version
                    .as_ref()
                    .map_or(true, |v| requirement.is_satisfied_by(v))
        })
    }

    fn check_conflicts(
        &self,
        candidate: &Arc<Package>,
        requirement: &Requirement,
    ) -> Option<ResolutionConflict> {
        // Check for version conflicts with existing packages
        for existing in &self.resolved_packages {
            if existing.package.name == candidate.name {
                if let (Some(existing_version), Some(candidate_version)) =
                    (&existing.package.version, &candidate.version)
                {
                    if existing_version != candidate_version {
                        return Some(ResolutionConflict {
                            package_name: candidate.name.clone(),
                            conflicting_requirements: vec![requirement.clone()],
                            source_packages: vec![existing.package.name.clone()],
                        });
                    }
                }
            }
        }

        None
    }

    fn add_resolved_package(&mut self, package: ResolvedPackageInfo) {
        self.resolved_packages.push(package);
    }

    fn is_original_requirement(&self, requirement: &Requirement) -> bool {
        self.original_requirements
            .iter()
            .any(|orig| orig.name == requirement.name)
    }
}

impl Default for ResolutionStats {
    fn default() -> Self {
        Self {
            packages_considered: 0,
            variants_evaluated: 0,
            resolution_time_ms: 0,
            conflicts_encountered: 0,
            backtrack_steps: 0,
        }
    }
}
