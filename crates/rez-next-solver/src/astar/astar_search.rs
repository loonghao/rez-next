//! A* Search Algorithm Implementation for Dependency Resolution
//!
//! Implements the core A* search algorithm optimized for dependency resolution.
//! Uses heuristic-guided search to find optimal dependency solutions efficiently.

use super::search_state::{ConflictType, DependencyConflict, SearchState, StatePool};
use crate::SolverConfig;
use rez_next_common::RezCoreError;
use rez_next_package::{Package, PackageRequirement};
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use rez_next_version::VersionRange;
use std::collections::{BinaryHeap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A* search algorithm for dependency resolution
pub struct AStarSearch {
    /// Open set: states to be evaluated (priority queue / min-heap via reversed Ord)
    open_set: BinaryHeap<SearchState>,

    /// Closed set: hashes of states already evaluated
    closed_set: HashSet<u64>,

    /// State pool for memory management
    state_pool: StatePool,

    /// Repository manager for package lookup
    repository_manager: Arc<RepositoryManager>,

    /// Solver configuration
    config: SolverConfig,

    /// Search statistics
    stats: SearchStats,

    /// Maximum search time
    max_search_time: Duration,

    /// Maximum number of states to explore
    max_states: usize,
}

/// Search statistics for monitoring and debugging
#[derive(Debug, Clone, Default)]
pub struct SearchStats {
    /// Total states explored
    pub states_explored: usize,

    /// States in open set
    pub open_set_size: usize,

    /// States in closed set
    pub closed_set_size: usize,

    /// Search time elapsed
    pub search_time_ms: u64,

    /// Number of goal states found
    pub goal_states_found: usize,

    /// Number of invalid states pruned
    pub invalid_states_pruned: usize,

    /// Average branching factor
    pub avg_branching_factor: f64,
}

impl AStarSearch {
    /// Create a new A* search instance
    pub fn new(
        repository_manager: Arc<RepositoryManager>,
        config: SolverConfig,
        max_search_time: Duration,
        max_states: usize,
    ) -> Self {
        Self {
            open_set: BinaryHeap::new(),
            closed_set: HashSet::new(),
            state_pool: StatePool::new(1000),
            repository_manager,
            config,
            stats: SearchStats::default(),
            max_search_time,
            max_states,
        }
    }

    /// Build a repository manager from a list of paths
    pub fn from_paths(
        paths: Vec<PathBuf>,
        config: SolverConfig,
        max_search_time: Duration,
        max_states: usize,
    ) -> Self {
        let mut repo_manager = RepositoryManager::new();
        for (i, path) in paths.into_iter().filter(|p| p.exists()).enumerate() {
            repo_manager.add_repository(Box::new(SimpleRepository::new(
                path,
                format!("repo_{}", i),
            )));
        }
        Self::new(Arc::new(repo_manager), config, max_search_time, max_states)
    }

    /// Perform A* search to find optimal dependency resolution
    pub async fn search(
        &mut self,
        initial_requirements: Vec<PackageRequirement>,
        heuristic_fn: impl Fn(&SearchState) -> f64,
    ) -> Result<Option<SearchState>, RezCoreError> {
        let start_time = Instant::now();

        let mut initial_state = SearchState::new_initial(initial_requirements);
        initial_state.estimated_total_cost = heuristic_fn(&initial_state);

        self.open_set.push(initial_state);
        self.stats = SearchStats::default();

        while let Some(current_state) = self.open_set.pop() {
            if start_time.elapsed() > self.max_search_time {
                return Err(RezCoreError::Solver(
                    "A* search time limit exceeded".to_string(),
                ));
            }

            if self.stats.states_explored >= self.max_states {
                return Err(RezCoreError::Solver(
                    "A* maximum states limit exceeded".to_string(),
                ));
            }

            if self.closed_set.contains(&current_state.get_hash()) {
                continue;
            }

            self.closed_set.insert(current_state.get_hash());
            self.stats.states_explored += 1;

            if current_state.is_goal() {
                self.stats.goal_states_found += 1;
                self.stats.search_time_ms = start_time.elapsed().as_millis() as u64;
                return Ok(Some(current_state));
            }

            if !current_state.is_valid() {
                self.stats.invalid_states_pruned += 1;
                continue;
            }

            let successors = self.generate_successors(&current_state).await?;
            let branching = successors.len();

            for mut successor in successors {
                let successor_hash = successor.get_hash();

                if self.closed_set.contains(&successor_hash) {
                    continue;
                }

                let h_value = heuristic_fn(&successor);
                successor.estimated_total_cost = successor.cost_so_far + h_value;
                self.open_set.push(successor);
            }

            // Update running average branching factor
            let n = self.stats.states_explored as f64;
            self.stats.avg_branching_factor =
                (self.stats.avg_branching_factor * (n - 1.0) + branching as f64) / n;

            self.stats.open_set_size = self.open_set.len();
            self.stats.closed_set_size = self.closed_set.len();
        }

        self.stats.search_time_ms = start_time.elapsed().as_millis() as u64;
        Ok(None)
    }

    /// Generate successor states from current state
    async fn generate_successors(
        &self,
        current_state: &SearchState,
    ) -> Result<Vec<SearchState>, RezCoreError> {
        let mut successors = Vec::new();

        let requirement = match current_state.get_next_requirement() {
            Some(r) => r.clone(),
            None => return Ok(successors),
        };

        // Find all packages satisfying the requirement
        let candidates = self
            .repository_manager
            .find_packages(&requirement.name)
            .await?;

        for pkg in candidates {
            // Filter by version if specified
            if let Some(ref version_spec) = requirement.version_spec {
                if let Some(ref pkg_ver) = pkg.version {
                    if let Ok(range) = VersionRange::parse(version_spec) {
                        if !range.contains(pkg_ver) {
                            continue;
                        }
                    }
                }
            }

            // Filter pre-release unless allowed
            if !self.config.allow_prerelease {
                if let Some(ref ver) = pkg.version {
                    if ver.is_prerelease() {
                        continue;
                    }
                }
            }

            if let Ok(successor) = self
                .create_successor_state(current_state, (*pkg).clone(), &requirement)
                .await
            {
                successors.push(successor);
            }
        }

        Ok(successors)
    }

    /// Create a successor state by resolving a requirement with a package
    async fn create_successor_state(
        &self,
        parent_state: &SearchState,
        package: Package,
        resolved_requirement: &PackageRequirement,
    ) -> Result<SearchState, RezCoreError> {
        let package_cost = self.calculate_package_cost(&package);

        // Translate package.requires (Vec<String>) into pending requirements (skip already resolved)
        let new_requirements: Vec<PackageRequirement> = package
            .requires
            .iter()
            .filter_map(|req_str| {
                let req = PackageRequirement::parse(req_str).ok()?;
                if parent_state.resolved_packages.contains_key(&req.name) {
                    None
                } else {
                    Some(req)
                }
            })
            .collect();

        let mut successor =
            SearchState::new_from_parent(parent_state, package, new_requirements, package_cost);

        successor.remove_requirement(resolved_requirement);

        self.detect_conflicts(&mut successor).await?;

        Ok(successor)
    }

    /// Calculate the cost of adding a package to the resolution
    fn calculate_package_cost(&self, package: &Package) -> f64 {
        let base = 1.0;
        let dep_cost = package.requires.len() as f64 * 0.1;

        // Prefer later versions slightly (lower cost = more preferred)
        let version_discount = if let Some(ref ver) = package.version {
            // Simple heuristic: newer versions have slightly lower cost
            let tokens: Vec<u64> = ver
                .as_str()
                .split('.')
                .filter_map(|t| t.parse().ok())
                .collect();
            let major = tokens.first().copied().unwrap_or(0);
            -(major.min(100) as f64) * 0.001
        } else {
            0.0
        };

        (base + dep_cost + version_discount).max(0.001)
    }

    /// Detect conflicts in the current state
    async fn detect_conflicts(&self, state: &mut SearchState) -> Result<(), RezCoreError> {
        let mut conflicts_to_add: Vec<DependencyConflict> = Vec::new();

        // Check for version conflicts
        for requirement in &state.pending_requirements {
            if let Some(resolved_pkg) = state.resolved_packages.get(&requirement.name) {
                if let Some(ref version_spec) = requirement.version_spec {
                    if let Some(ref resolved_ver) = resolved_pkg.version {
                        if let Ok(range) = VersionRange::parse(version_spec) {
                            if !range.contains(resolved_ver) {
                                conflicts_to_add.push(DependencyConflict::new(
                                    requirement.name.clone(),
                                    vec![
                                        requirement.to_string(),
                                        format!("resolved={}", resolved_ver.as_str()),
                                    ],
                                    1.0,
                                    ConflictType::VersionConflict,
                                ));
                            }
                        }
                    }
                }
            }
        }

        // Check for circular dependencies
        let resolved_names: HashSet<String> = state.resolved_packages.keys().cloned().collect();

        for requirement in &state.pending_requirements {
            if resolved_names.contains(&requirement.name) {
                if let Some(pkg) = state.resolved_packages.get(&requirement.name) {
                    for dep_str in pkg.requires.iter() {
                        let dep_name = dep_str
                            .split('-')
                            .next()
                            .unwrap_or(dep_str.as_str())
                            .to_string();
                        if let Some(resolved_dep) = state.resolved_packages.get(&dep_name) {
                            let dep_requires_parent = resolved_dep.requires.iter().any(|r| {
                                r.split('-').next().unwrap_or(r.as_str()) == requirement.name
                            });
                            if dep_requires_parent {
                                conflicts_to_add.push(DependencyConflict::new(
                                    requirement.name.clone(),
                                    vec![requirement.name.clone(), dep_name.clone()],
                                    1.0,
                                    ConflictType::CircularDependency,
                                ));
                            }
                        }
                    }
                }
            }
        }

        for conflict in conflicts_to_add {
            state.add_conflict(conflict);
        }

        Ok(())
    }

    /// Get current search statistics
    pub fn get_stats(&self) -> &SearchStats {
        &self.stats
    }

    /// Clear search state for reuse
    pub fn clear(&mut self) {
        self.open_set.clear();
        self.closed_set.clear();
        self.stats = SearchStats::default();
    }

    /// Reconstruct solution packages from goal state (sorted by dependency order)
    pub fn reconstruct_path(&self, goal_state: &SearchState) -> Vec<Package> {
        let mut packages: Vec<Package> =
            goal_state.resolved_packages.values().cloned().collect();
        // Sort by version descending for determinism
        packages.sort_by(|a, b| match (&b.version, &a.version) {
            (Some(v1), Some(v2)) => v1.cmp(v2),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.name.cmp(&b.name),
        });
        packages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SolverConfig;
    use std::time::Duration;
    use tempfile::TempDir;
    use tokio::fs;

    async fn setup_repo_with_package(dir: &std::path::Path, name: &str, version: &str) {
        let pkg_dir = dir.join(name).join(version);
        fs::create_dir_all(&pkg_dir).await.unwrap();
        let content = format!(
            "name = \"{}\"\nversion = \"{}\"\ndescription = \"Test\"\n",
            name, version
        );
        fs::write(pkg_dir.join("package.py"), content).await.unwrap();
    }

    #[tokio::test]
    async fn test_astar_search_creation() {
        let repo_manager = Arc::new(RepositoryManager::new());
        let config = SolverConfig::default();
        let search = AStarSearch::new(repo_manager, config, Duration::from_secs(30), 1000);

        assert_eq!(search.stats.states_explored, 0);
        assert_eq!(search.open_set.len(), 0);
        assert_eq!(search.closed_set.len(), 0);
    }

    #[tokio::test]
    async fn test_astar_empty_requirements_returns_goal() {
        let repo_manager = Arc::new(RepositoryManager::new());
        let config = SolverConfig::default();
        let mut search =
            AStarSearch::new(repo_manager, config, Duration::from_secs(30), 1000);

        let result = search
            .search(vec![], |_| 0.0)
            .await
            .expect("search should not fail");

        assert!(result.is_some(), "Empty requirements => immediate goal state");
        assert!(result.unwrap().is_goal());
    }

    #[tokio::test]
    async fn test_astar_resolves_single_package() {
        let temp_dir = TempDir::new().unwrap();
        setup_repo_with_package(temp_dir.path(), "python", "3.9.0").await;

        let mut repo_manager = RepositoryManager::new();
        repo_manager.add_repository(Box::new(SimpleRepository::new(
            temp_dir.path(),
            "test_repo".to_string(),
        )));

        let config = SolverConfig::default();
        let mut search = AStarSearch::new(
            Arc::new(repo_manager),
            config,
            Duration::from_secs(10),
            500,
        );

        let req = PackageRequirement::new("python".to_string());
        let result = search
            .search(vec![req], |state| state.pending_requirements.len() as f64)
            .await
            .expect("search should succeed");

        assert!(result.is_some(), "Should find a resolution for 'python'");
        let goal = result.unwrap();
        assert!(goal.resolved_packages.contains_key("python"));
    }

    #[test]
    fn test_package_cost_base_value() {
        let repo_manager = Arc::new(RepositoryManager::new());
        let config = SolverConfig::default();
        let search = AStarSearch::new(repo_manager, config, Duration::from_secs(30), 1000);

        let package = Package::new("test_package".to_string());
        let cost = search.calculate_package_cost(&package);
        assert!(cost > 0.0, "Package cost should be positive");
    }

    #[test]
    fn test_package_cost_increases_with_deps() {
        let repo_manager = Arc::new(RepositoryManager::new());
        let config = SolverConfig::default();
        let search = AStarSearch::new(repo_manager, config, Duration::from_secs(30), 1000);

        let mut pkg_no_deps = Package::new("no_deps".to_string());
        let mut pkg_with_deps = Package::new("with_deps".to_string());
        pkg_with_deps.requires.push("dep1".to_string());
        pkg_with_deps.requires.push("dep2".to_string());

        let cost_no = search.calculate_package_cost(&pkg_no_deps);
        let cost_with = search.calculate_package_cost(&pkg_with_deps);
        assert!(cost_with > cost_no, "Packages with more deps should cost more");
    }

    #[tokio::test]
    async fn test_astar_stats_updated_after_search() {
        let repo_manager = Arc::new(RepositoryManager::new());
        let config = SolverConfig::default();
        let mut search =
            AStarSearch::new(repo_manager, config, Duration::from_secs(30), 1000);

        search.search(vec![], |_| 0.0).await.unwrap();
        assert!(search.stats.states_explored >= 1, "At least 1 state explored");
        assert!(search.stats.search_time_ms < 5000, "Should complete quickly");
    }

    #[tokio::test]
    async fn test_astar_clear_resets_state() {
        let repo_manager = Arc::new(RepositoryManager::new());
        let config = SolverConfig::default();
        let mut search =
            AStarSearch::new(repo_manager, config, Duration::from_secs(30), 1000);

        search.search(vec![], |_| 0.0).await.unwrap();
        search.clear();

        assert_eq!(search.open_set.len(), 0);
        assert_eq!(search.closed_set.len(), 0);
        assert_eq!(search.stats.states_explored, 0);
    }
}
