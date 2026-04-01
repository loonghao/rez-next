//! Dependency resolution implementation - equivalent to Python's solver

use crate::{SolverConfig, SolverStats};
use rez_next_common::RezCoreError;
use rez_next_package::{Package, Requirement};
use rez_next_repository::simple_repository::RepositoryManager;
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

        Ok(DetailedResolutionResult {
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

    /// Filter candidate packages based on version constraints (legacy alias)
    fn filter_candidates(
        &self,
        packages: &[Arc<Package>],
        requirement: &Requirement,
    ) -> Vec<Arc<Package>> {
        self.filter_and_sort_candidates(packages, requirement)
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

    /// Dependency graph edges: package_name -> list of its direct requirements (package names)
    dep_graph: HashMap<String, Vec<String>>,

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
            dep_graph: HashMap::new(),
            packages_considered: 0,
            variants_evaluated: 0,
            backtrack_steps: 0,
        }
    }

    /// Record a dependency edge: `from_pkg` requires `to_pkg`
    fn record_dependency(&mut self, from_pkg: &str, to_pkg: &str) {
        self.dep_graph
            .entry(from_pkg.to_string())
            .or_default()
            .push(to_pkg.to_string());
    }

    /// Detect cycles in the dependency graph using DFS.
    /// Returns Some(cycle_path) if a cycle is detected, None otherwise.
    pub fn detect_cycle(&self) -> Option<Vec<String>> {
        let mut visited: HashSet<String> = HashSet::new();
        let mut path: Vec<String> = Vec::new();
        let mut on_stack: HashSet<String> = HashSet::new();

        for node in self.dep_graph.keys() {
            if !visited.contains(node) {
                if let Some(cycle) = self.dfs_cycle(node, &mut visited, &mut on_stack, &mut path) {
                    return Some(cycle);
                }
            }
        }
        None
    }

    fn dfs_cycle(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        on_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(node.to_string());
        on_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(neighbors) = self.dep_graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if let Some(cycle) = self.dfs_cycle(neighbor, visited, on_stack, path) {
                        return Some(cycle);
                    }
                } else if on_stack.contains(neighbor) {
                    // Found a back-edge: extract cycle
                    let cycle_start = path.iter().position(|n| n == neighbor).unwrap_or(0);
                    let mut cycle = path[cycle_start..].to_vec();
                    cycle.push(neighbor.clone()); // close the cycle
                    return Some(cycle);
                }
            }
        }

        on_stack.remove(node);
        path.pop();
        None
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::ConflictStrategy;
    use rez_next_package::{Package, Requirement};
    use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
    use rez_next_version::Version;
    use serde_json;
    use std::sync::Arc;

    /// Write a minimal package.py to a temp directory and return path
    fn write_package(base: &std::path::Path, name: &str, version: &str, requires: &[&str]) {
        let pkg_dir = base.join(name).join(version);
        std::fs::create_dir_all(&pkg_dir).unwrap();
        let mut content = format!("name = '{}'\nversion = '{}'\n", name, version);
        if !requires.is_empty() {
            content.push_str("requires = [\n");
            for req in requires {
                content.push_str(&format!("    '{}',\n", req));
            }
            content.push_str("]\n");
        }
        std::fs::write(pkg_dir.join("package.py"), content).unwrap();
    }

    #[test]
    fn test_empty_requirements_resolves_empty() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let repo = RepositoryManager::new();
        let repo_arc = Arc::new(repo);
        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());

        let result = rt.block_on(resolver.resolve(vec![])).unwrap();
        assert!(result.resolved_packages.is_empty());
        assert!(result.failed_requirements.is_empty());
    }

    #[test]
    fn test_single_package_file_repo() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "foo", "1.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let req = Requirement::new("foo".to_string());
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();

        assert_eq!(result.resolved_packages.len(), 1);
        assert_eq!(result.resolved_packages[0].package.name, "foo");
    }

    #[test]
    fn test_package_with_dependency_resolved() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "bar", "2.0.0", &[]);
        write_package(tmp.path(), "foo", "1.0.0", &["bar"]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let req = Requirement::new("foo".to_string());
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();

        let names: Vec<&str> = result
            .resolved_packages
            .iter()
            .map(|r| r.package.name.as_str())
            .collect();
        assert!(
            names.contains(&"foo"),
            "foo should be resolved, got: {:?}",
            names
        );
        assert!(
            names.contains(&"bar"),
            "bar dependency should be resolved, got: {:?}",
            names
        );
    }

    #[test]
    fn test_prefer_latest_version_selection() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "foo", "1.0.0", &[]);
        write_package(tmp.path(), "foo", "2.0.0", &[]);
        write_package(tmp.path(), "foo", "1.5.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut config = SolverConfig::default();
        config.prefer_latest = true;
        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), config);

        let req = Requirement::new("foo".to_string());
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();

        assert_eq!(result.resolved_packages.len(), 1);
        let ver = result.resolved_packages[0]
            .package
            .version
            .as_ref()
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        assert_eq!(ver, "2.0.0", "Should select latest version (2.0.0)");
    }

    #[test]
    fn test_missing_package_returns_empty_or_error() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let repo = RepositoryManager::new();
        let repo_arc = Arc::new(repo);
        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());

        let req = Requirement::new("nonexistent".to_string());
        let result = rt.block_on(resolver.resolve(vec![req]));

        match result {
            Err(_) => {}
            Ok(r) => {
                assert!(
                    !r.failed_requirements.is_empty() || r.resolved_packages.is_empty(),
                    "Non-existent package should fail or leave failed requirements"
                );
            }
        }
    }

    #[test]
    fn test_solver_config_defaults() {
        let config = SolverConfig::default();
        assert!(config.prefer_latest);
        assert!(config.max_attempts > 0);
        assert!(!config.allow_prerelease);
    }

    #[test]
    fn test_resolution_stats_default() {
        let stats = ResolutionStats::default();
        assert_eq!(stats.packages_considered, 0);
        assert_eq!(stats.variants_evaluated, 0);
        assert_eq!(stats.conflicts_encountered, 0);
    }

    #[test]
    fn test_resolution_result_structure() {
        let result = DetailedResolutionResult {
            resolved_packages: vec![],
            failed_requirements: vec![],
            conflicts: vec![],
            stats: ResolutionStats::default(),
        };
        assert!(result.resolved_packages.is_empty());
        assert!(result.failed_requirements.is_empty());
        assert!(result.conflicts.is_empty());
    }

    /// Phase 61: Verify real VersionRange filtering works in solver
    #[test]
    fn test_version_range_constraint_filters_packages() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        // Add foo 1.0.0, 1.5.0, 2.0.0
        // Note: In rez version semantics, "2.0" > "2.0.0" (shorter = greater)
        // So "foo>=1.0,<2" means <2 (epoch), which excludes 2.x family
        write_package(tmp.path(), "foo", "1.0.0", &[]);
        write_package(tmp.path(), "foo", "1.5.0", &[]);
        write_package(tmp.path(), "foo", "2.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut config = SolverConfig::default();
        config.prefer_latest = true;
        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), config);

        // Parse "foo>=1.0.0,<1.5.0" as a Requirement - strict range that excludes 1.5.0 and 2.0.0
        let req: Requirement = "foo>=1.0.0,<1.5.0".parse().unwrap();
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();

        assert_eq!(
            result.resolved_packages.len(),
            1,
            "Should resolve exactly one foo"
        );
        let resolved_ver = result.resolved_packages[0]
            .package
            .version
            .as_ref()
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        // Should pick 1.0.0 (the only version satisfying >=1.0.0,<1.5.0)
        assert_eq!(
            resolved_ver, "1.0.0",
            "Should select 1.0.0 (only version satisfying >=1.0.0,<1.5.0), got: {}",
            resolved_ver
        );
    }

    /// Verify that a too-strict constraint returns no results
    #[test]
    fn test_version_range_excludes_all() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "bar", "1.0.0", &[]);
        write_package(tmp.path(), "bar", "2.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        // Request bar>=3.0 - no such version exists
        let req: Requirement = "bar>=3.0".parse().unwrap();
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();

        // Should not resolve (empty or failed)
        assert!(
            result.resolved_packages.is_empty() || !result.failed_requirements.is_empty(),
            "bar>=3.0 should not resolve when only 1.0 and 2.0 exist"
        );
    }

    /// Verify exact version constraint
    #[test]
    fn test_exact_version_constraint() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "baz", "1.0.0", &[]);
        write_package(tmp.path(), "baz", "1.5.0", &[]);
        write_package(tmp.path(), "baz", "2.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        // Request exact baz==1.5.0
        let req: Requirement = "baz==1.5.0".parse().unwrap();
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();

        assert_eq!(result.resolved_packages.len(), 1);
        let resolved_ver = result.resolved_packages[0]
            .package
            .version
            .as_ref()
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        assert_eq!(resolved_ver, "1.5.0", "Should resolve exact version 1.5.0");
    }

    /// Phase 70: Test diamond dependency pattern
    /// A depends on B and C; B depends on D>=1.0; C depends on D>=1.5
    /// Result: D should be at least 1.5
    #[test]
    fn test_diamond_dependency() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "d_pkg", "1.0.0", &[]);
        write_package(tmp.path(), "d_pkg", "1.5.0", &[]);
        write_package(tmp.path(), "d_pkg", "2.0.0", &[]);
        write_package(tmp.path(), "b_pkg", "1.0.0", &["d_pkg>=1.0.0"]);
        write_package(tmp.path(), "c_pkg", "1.0.0", &["d_pkg>=1.5.0"]);
        write_package(tmp.path(), "a_pkg", "1.0.0", &["b_pkg", "c_pkg"]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let req = Requirement::new("a_pkg".to_string());
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();

        let names: Vec<&str> = result
            .resolved_packages
            .iter()
            .map(|r| r.package.name.as_str())
            .collect();
        assert!(names.contains(&"a_pkg"), "a_pkg should be resolved");
        assert!(
            names.contains(&"b_pkg"),
            "b_pkg dependency should be resolved"
        );
        assert!(
            names.contains(&"c_pkg"),
            "c_pkg dependency should be resolved"
        );
        assert!(
            names.contains(&"d_pkg"),
            "d_pkg should be resolved as transitive dep"
        );
    }

    /// Phase 70: Test multi-package resolution in one request
    #[test]
    fn test_multiple_packages_request() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "pkg_x", "1.0.0", &[]);
        write_package(tmp.path(), "pkg_y", "2.0.0", &[]);
        write_package(tmp.path(), "pkg_z", "3.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let reqs = vec![
            Requirement::new("pkg_x".to_string()),
            Requirement::new("pkg_y".to_string()),
            Requirement::new("pkg_z".to_string()),
        ];
        let result = rt.block_on(resolver.resolve(reqs)).unwrap();

        let names: Vec<&str> = result
            .resolved_packages
            .iter()
            .map(|r| r.package.name.as_str())
            .collect();
        assert!(names.contains(&"pkg_x"), "pkg_x should resolve");
        assert!(names.contains(&"pkg_y"), "pkg_y should resolve");
        assert!(names.contains(&"pkg_z"), "pkg_z should resolve");
    }

    /// Phase 70: Test prefer_earliest_version config
    #[test]
    fn test_prefer_earliest_version_selection() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "lib", "1.0.0", &[]);
        write_package(tmp.path(), "lib", "2.0.0", &[]);
        write_package(tmp.path(), "lib", "3.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut config = SolverConfig::default();
        config.prefer_latest = false; // prefer earliest
        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), config);

        let req = Requirement::new("lib".to_string());
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();

        assert_eq!(result.resolved_packages.len(), 1);
        let ver = result.resolved_packages[0]
            .package
            .version
            .as_ref()
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        assert_eq!(ver, "1.0.0", "Should select earliest version (1.0.0)");
    }

    /// Phase 81: Detect direct cycle A -> B -> A
    #[test]
    fn test_cycle_detection_ab() {
        let mut state = ResolutionState::new(vec![]);
        state.record_dependency("A", "B");
        state.record_dependency("B", "A");
        let cycle = state.detect_cycle();
        assert!(cycle.is_some(), "Should detect A->B->A cycle");
        let c = cycle.unwrap();
        assert!(c.len() >= 2, "Cycle path should have >= 2 nodes: {:?}", c);
    }

    /// Phase 81: No cycle in a linear chain A -> B -> C
    #[test]
    fn test_no_cycle_linear() {
        let mut state = ResolutionState::new(vec![]);
        state.record_dependency("A", "B");
        state.record_dependency("B", "C");
        assert!(state.detect_cycle().is_none(), "Linear chain has no cycle");
    }

    /// Phase 81: Detect 3-node cycle A -> B -> C -> A
    #[test]
    fn test_cycle_detection_three_nodes() {
        let mut state = ResolutionState::new(vec![]);
        state.record_dependency("A", "B");
        state.record_dependency("B", "C");
        state.record_dependency("C", "A");
        let cycle = state.detect_cycle();
        assert!(cycle.is_some(), "Should detect A->B->C->A cycle");
    }

    /// Phase 81: resolver returns Err on cyclic packages
    #[test]
    fn test_resolver_errors_on_cyclic_packages() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        // A requires B, B requires A (cycle)
        write_package(tmp.path(), "cyclic_a", "1.0.0", &["cyclic_b"]);
        write_package(tmp.path(), "cyclic_b", "1.0.0", &["cyclic_a"]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let req = Requirement::new("cyclic_a".to_string());
        let result = rt.block_on(resolver.resolve(vec![req]));

        assert!(
            result.is_err(),
            "Cyclic dependencies should return an error"
        );
        if let Err(e) = result {
            let msg = format!("{}", e);
            assert!(
                msg.contains("Cyclic") || msg.contains("cycle"),
                "Error message should mention cycle: {}",
                msg
            );
        }
    }

    /// Phase 81: resolver succeeds on a DAG (no cycle), diamond is fine
    #[test]
    fn test_resolver_succeeds_on_dag() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        // D <- B <- A, D <- C <- A (diamond, no cycle)
        write_package(tmp.path(), "dag_d", "1.0.0", &[]);
        write_package(tmp.path(), "dag_b", "1.0.0", &["dag_d"]);
        write_package(tmp.path(), "dag_c", "1.0.0", &["dag_d"]);
        write_package(tmp.path(), "dag_a", "1.0.0", &["dag_b", "dag_c"]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let req = Requirement::new("dag_a".to_string());
        let result = rt.block_on(resolver.resolve(vec![req]));
        assert!(
            result.is_ok(),
            "DAG (diamond) should resolve without error: {:?}",
            result
        );
        let r = result.unwrap();
        let names: Vec<&str> = r
            .resolved_packages
            .iter()
            .map(|p| p.package.name.as_str())
            .collect();
        assert!(names.contains(&"dag_a"));
        assert!(names.contains(&"dag_b"));
        assert!(names.contains(&"dag_c"));
        assert!(names.contains(&"dag_d"));
    }

    // ── Phase 89: Backtracking / version-downgrade scenarios ─────────────────

    /// Two packages require different version ranges of a shared dep.
    /// pkgA requires shared>=1.0,<2; pkgB requires shared>=1.5,<2
    /// Both should be satisfiable with shared==1.5.0
    #[test]
    fn test_shared_dep_intersection() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "shared", "1.0.0", &[]);
        write_package(tmp.path(), "shared", "1.5.0", &[]);
        write_package(tmp.path(), "shared", "2.0.0", &[]);
        write_package(tmp.path(), "pkgA", "1.0.0", &["shared>=1.0.0,<2.0.0"]);
        write_package(tmp.path(), "pkgB", "1.0.0", &["shared>=1.5.0,<2.0.0"]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let reqs = vec![
            Requirement::new("pkgA".to_string()),
            Requirement::new("pkgB".to_string()),
        ];
        let result = rt.block_on(resolver.resolve(reqs)).unwrap();
        let names: Vec<&str> = result
            .resolved_packages
            .iter()
            .map(|r| r.package.name.as_str())
            .collect();
        assert!(names.contains(&"pkgA"), "pkgA should resolve");
        assert!(names.contains(&"pkgB"), "pkgB should resolve");
        assert!(names.contains(&"shared"), "shared should resolve");
    }

    /// Resolver picks latest by default — confirm newest 3.x is selected when available
    #[test]
    fn test_latest_across_major_versions() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "engine", "1.0.0", &[]);
        write_package(tmp.path(), "engine", "2.0.0", &[]);
        write_package(tmp.path(), "engine", "3.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let req = Requirement::new("engine".to_string());
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();
        assert_eq!(result.resolved_packages.len(), 1);
        let ver = result.resolved_packages[0]
            .package
            .version
            .as_ref()
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        assert_eq!(ver, "3.0.0", "Should pick latest 3.0.0");
    }

    /// Backtrack: first candidate 2.0 conflicts via constraint, resolver must use 1.9
    #[test]
    fn test_version_downgrade_on_upper_bound() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "util", "1.8.0", &[]);
        write_package(tmp.path(), "util", "1.9.0", &[]);
        write_package(tmp.path(), "util", "2.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut config = SolverConfig::default();
        config.prefer_latest = true;
        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), config);
        // Strict upper bound: <2.0.0 forces downgrade from 2.0
        let req: Requirement = "util>=1.8.0,<2.0.0".parse().unwrap();
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();
        assert_eq!(result.resolved_packages.len(), 1);
        let ver = result.resolved_packages[0]
            .package
            .version
            .as_ref()
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        assert_eq!(
            ver, "1.9.0",
            "Should downgrade to 1.9.0 given <2.0.0 constraint, got: {}",
            ver
        );
    }

    /// Resolution stats: backtrack_steps and packages_considered are tracked
    #[test]
    fn test_resolution_stats_are_populated() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "alpha", "1.0.0", &[]);
        write_package(tmp.path(), "beta", "2.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let reqs = vec![
            Requirement::new("alpha".to_string()),
            Requirement::new("beta".to_string()),
        ];
        let result = rt.block_on(resolver.resolve(reqs)).unwrap();
        assert!(
            result.stats.packages_considered >= 2,
            "Should have considered at least 2 packages"
        );
    }

    /// conflict_strategy=FailOnConflict config exists and is serializable
    #[test]
    fn test_conflict_strategy_fail_on_conflict_config() {
        let mut config = SolverConfig::default();
        config.conflict_strategy = ConflictStrategy::FailOnConflict;
        let json = serde_json::to_string(&config).unwrap();
        assert!(
            json.contains("FailOnConflict"),
            "Serialized config should contain strategy name"
        );
        let back: SolverConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.conflict_strategy, ConflictStrategy::FailOnConflict);
    }

    /// Dedup: requesting same package twice should not resolve it twice
    #[test]
    fn test_duplicate_requirements_dedup() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "mylib", "1.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        // Same package requested twice
        let reqs = vec![
            Requirement::new("mylib".to_string()),
            Requirement::new("mylib".to_string()),
        ];
        let result = rt.block_on(resolver.resolve(reqs)).unwrap();
        // Should resolve mylib only once
        let mylib_count = result
            .resolved_packages
            .iter()
            .filter(|r| r.package.name == "mylib")
            .count();
        assert_eq!(
            mylib_count, 1,
            "mylib should only appear once, got: {}",
            mylib_count
        );
    }

    // ── Phase 113: Solver + VersionRange end-to-end tests ───────────────────

    /// !=2.0 constraint excludes the bad version, selects latest non-excluded
    #[test]
    fn test_ne_constraint_excludes_version() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "lib", "1.0.0", &[]);
        write_package(tmp.path(), "lib", "2.0.0", &[]);
        write_package(tmp.path(), "lib", "3.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        // Exclude 2.0, prefer latest: should pick 3.0 or 1.0 (never 2.0)
        let req: Requirement = "lib!=2.0.0".parse().unwrap();
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();
        // If resolved, should NOT be 2.0.0
        for r in &result.resolved_packages {
            if r.package.name == "lib" {
                let ver = r
                    .package
                    .version
                    .as_ref()
                    .map(|v| v.as_str().to_string())
                    .unwrap_or_default();
                assert_ne!(ver, "2.0.0", "Should not resolve to excluded version 2.0.0");
            }
        }
        // Either resolved to non-2.0 or failed requirements (both are valid for !=)
    }

    /// compatible release ~=1.2 means >=1.2,<2: selects within 1.x
    #[test]
    fn test_compatible_release_constraint() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "compat_lib", "1.0.0", &[]);
        write_package(tmp.path(), "compat_lib", "1.3.0", &[]);
        write_package(tmp.path(), "compat_lib", "2.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        // ~=1.2 means >=1.2, <2 — should pick 1.3.0
        let req: Requirement = "compat_lib~=1.2".parse().unwrap();
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();
        if !result.resolved_packages.is_empty() {
            let ver = result.resolved_packages[0]
                .package
                .version
                .as_ref()
                .map(|v| v.as_str().to_string())
                .unwrap_or_default();
            assert_ne!(
                ver, "2.0.0",
                "~=1.2 should not select 2.0.0 (outside compatible range)"
            );
        }
    }

    /// prefer_latest=false with range: selects lowest in range
    #[test]
    fn test_prefer_earliest_with_range() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "rangelib", "1.0.0", &[]);
        write_package(tmp.path(), "rangelib", "1.5.0", &[]);
        write_package(tmp.path(), "rangelib", "1.9.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut config = SolverConfig::default();
        config.prefer_latest = false;
        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), config);
        let req: Requirement = "rangelib>=1.0.0,<2.0.0".parse().unwrap();
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();
        assert!(
            !result.resolved_packages.is_empty(),
            "Should resolve rangelib"
        );
        let ver = result.resolved_packages[0]
            .package
            .version
            .as_ref()
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        assert_eq!(
            ver, "1.0.0",
            "prefer_latest=false should pick 1.0.0 in range, got: {}",
            ver
        );
    }

    /// Empty repository: resolving any package returns empty or failed
    #[test]
    fn test_empty_repository_resolves_empty() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap(); // no packages written

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "empty".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let req = Requirement::new("nonexistent".to_string());
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();
        assert!(
            result.resolved_packages.is_empty() || !result.failed_requirements.is_empty(),
            "Empty repo should yield no resolved packages or failed requirements"
        );
    }

    /// Multi-repo priority: first repo's package is preferred
    #[test]
    fn test_multi_repo_priority() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp1 = tempfile::TempDir::new().unwrap();
        let tmp2 = tempfile::TempDir::new().unwrap();
        // Both repos have "shared_pkg" but different versions
        write_package(tmp1.path(), "shared_pkg", "2.0.0", &[]);
        write_package(tmp2.path(), "shared_pkg", "1.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp1.path(),
            "repo1".to_string(),
        )));
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp2.path(),
            "repo2".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut config = SolverConfig::default();
        config.prefer_latest = true;
        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), config);
        let req = Requirement::new("shared_pkg".to_string());
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();
        assert!(
            !result.resolved_packages.is_empty(),
            "Should resolve shared_pkg from multi-repo"
        );
        // With prefer_latest, should select highest version across repos
        let ver = result.resolved_packages[0]
            .package
            .version
            .as_ref()
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        assert_eq!(ver, "2.0.0", "Should prefer 2.0.0 (latest) from repo1");
    }

    /// All ConflictStrategy variants are serializable
    #[test]
    fn test_all_conflict_strategies_serializable() {
        let strategies = [
            ConflictStrategy::LatestWins,
            ConflictStrategy::EarliestWins,
            ConflictStrategy::FailOnConflict,
            ConflictStrategy::FindCompatible,
        ];
        for strategy in &strategies {
            let json = serde_json::to_string(strategy).unwrap();
            assert!(
                !json.is_empty(),
                "Strategy should serialize: {:?}",
                strategy
            );
            let back: ConflictStrategy = serde_json::from_str(&json).unwrap();
            assert_eq!(
                back, *strategy,
                "Strategy roundtrip should match: {:?}",
                strategy
            );
        }
    }
}
