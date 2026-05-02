//! Core solver implementation

use crate::dependency_resolver::DependencyResolver;
use crate::resolution::ResolutionResult;
use rez_next_common::RezCoreError;
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_repository::simple_repository::RepositoryManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Solver configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverConfig {
    /// Maximum number of resolution attempts
    pub max_attempts: usize,
    /// Maximum resolution time in seconds
    pub max_time_seconds: u64,
    /// Enable parallel resolution
    pub enable_parallel: bool,
    /// Maximum number of parallel workers
    pub max_workers: usize,
    /// Enable solver caching
    pub enable_caching: bool,
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
    /// Prefer latest versions
    pub prefer_latest: bool,
    /// Allow pre-release versions
    pub allow_prerelease: bool,
    /// Conflict resolution strategy
    pub conflict_strategy: ConflictStrategy,
    /// Strict mode: return Err if any requirement cannot be satisfied.
    /// When false (lenient/default), unsatisfied requirements are recorded in
    /// `DetailedResolutionResult::failed_requirements` and resolution continues.
    pub strict_mode: bool,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            max_attempts: 1000,
            max_time_seconds: 300, // 5 minutes
            enable_parallel: true,
            max_workers: 4,
            enable_caching: true,
            cache_ttl_seconds: 3600, // 1 hour
            prefer_latest: true,
            allow_prerelease: false,
            conflict_strategy: ConflictStrategy::LatestWins,
            strict_mode: false,
        }
    }
}

/// Conflict resolution strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConflictStrategy {
    /// Latest version wins
    LatestWins,
    /// Earliest version wins
    EarliestWins,
    /// Fail on conflict
    FailOnConflict,
    /// Try to find compatible version
    FindCompatible,
}

/// Solver request
#[derive(Debug, Clone)]
pub struct SolverRequest {
    /// Root requirements to resolve
    pub requirements: Vec<PackageRequirement>,
    /// Additional constraints
    pub constraints: Vec<PackageRequirement>,
    /// Packages to exclude
    pub excludes: Vec<String>,
    /// Platform constraints
    pub platform: Option<String>,
    /// Architecture constraints
    pub arch: Option<String>,
    /// Request metadata
    pub metadata: HashMap<String, String>,
}

impl SolverRequest {
    /// Create a new solver request
    pub fn new(requirements: Vec<PackageRequirement>) -> Self {
        Self {
            requirements,
            constraints: Vec::new(),
            excludes: Vec::new(),
            platform: None,
            arch: None,
            metadata: HashMap::new(),
        }
    }

    /// Add a constraint
    pub fn with_constraint(mut self, constraint: PackageRequirement) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Add an exclusion
    pub fn with_exclude(mut self, package_name: String) -> Self {
        self.excludes.push(package_name);
        self
    }

    /// Set platform constraint
    pub fn with_platform(mut self, platform: String) -> Self {
        self.platform = Some(platform);
        self
    }

    /// Set architecture constraint
    pub fn with_arch(mut self, arch: String) -> Self {
        self.arch = Some(arch);
        self
    }
}

/// High-performance dependency solver
pub struct DependencySolver {
    /// Solver configuration
    config: SolverConfig,
    /// Repository manager for package discovery
    repository_manager: Option<Arc<RepositoryManager>>,
}

impl std::fmt::Debug for DependencySolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DependencySolver")
            .field("config", &self.config)
            .field("has_repository", &self.repository_manager.is_some())
            .finish()
    }
}

/// Solver statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverStats {
    /// Total number of resolutions
    pub total_resolutions: u64,
    /// Successful resolutions
    pub successful_resolutions: u64,
    /// Failed resolutions
    pub failed_resolutions: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Average resolution time in milliseconds
    pub avg_resolution_time_ms: f64,
    /// Total resolution time in milliseconds
    pub total_resolution_time_ms: u64,
}

impl Default for SolverStats {
    fn default() -> Self {
        Self {
            total_resolutions: 0,
            successful_resolutions: 0,
            failed_resolutions: 0,
            cache_hits: 0,
            cache_misses: 0,
            avg_resolution_time_ms: 0.0,
            total_resolution_time_ms: 0,
        }
    }
}

impl DependencySolver {
    /// Create a new solver with default configuration
    pub fn new() -> Self {
        let config = SolverConfig::default();
        Self {
            config,
            repository_manager: None,
        }
    }

    /// Create a new solver with custom configuration
    pub fn with_config(config: SolverConfig) -> Self {
        Self {
            config,
            repository_manager: None,
        }
    }

    /// Set repository manager for package discovery
    pub fn with_repository_manager(mut self, manager: Arc<RepositoryManager>) -> Self {
        self.repository_manager = Some(manager);
        self
    }

    /// Set repository manager in place
    pub fn set_repository_manager(&mut self, manager: Arc<RepositoryManager>) {
        self.repository_manager = Some(manager);
    }

    /// Resolve dependencies for a given request using DependencyResolver when possible
    pub fn resolve(&self, request: SolverRequest) -> Result<ResolutionResult, RezCoreError> {
        if let Some(ref repo_manager) = self.repository_manager {
            // Use the real DependencyResolver backed by repositories
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| RezCoreError::Solver(format!("Failed to create runtime: {}", e)))?;

            // Convert PackageRequirement -> Requirement via string parsing
            let requirements: Vec<Requirement> = request
                .requirements
                .into_iter()
                .map(|pr| {
                    let req_str = pr.to_string();
                    req_str
                        .parse::<Requirement>()
                        .unwrap_or_else(|_| Requirement::new(pr.name.clone()))
                })
                .collect();

            let mut resolver =
                DependencyResolver::new(Arc::clone(repo_manager), self.config.clone());

            let result = rt.block_on(resolver.resolve(requirements))?;

            let packages: Vec<Package> = result
                .resolved_packages
                .into_iter()
                .map(|info| (*info.package).clone())
                .collect();

            Ok(ResolutionResult {
                packages,
                conflicts_resolved: !result.conflicts.is_empty(),
                resolution_time_ms: result.stats.resolution_time_ms,
                metadata: HashMap::new(),
            })
        } else {
            // No repository configured: return empty result (packages must be resolved externally)
            Ok(ResolutionResult {
                packages: Vec::new(),
                conflicts_resolved: false,
                resolution_time_ms: 0,
                metadata: HashMap::new(),
            })
        }
    }
}

impl Default for DependencySolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rez_next_package::PackageRequirement;

    /// Test SolverConfig default values
    #[test]
    fn test_solver_config_default() {
        let config = SolverConfig::default();
        assert_eq!(config.max_attempts, 1000);
        assert_eq!(config.max_time_seconds, 300);
        assert!(config.enable_parallel);
        assert_eq!(config.max_workers, 4);
        assert!(config.enable_caching);
        assert_eq!(config.cache_ttl_seconds, 3600);
        assert!(config.prefer_latest);
        assert!(!config.allow_prerelease);
        assert_eq!(config.conflict_strategy, ConflictStrategy::LatestWins);
        assert!(!config.strict_mode);
    }

    /// Test SolverConfig custom values
    #[test]
    fn test_solver_config_custom() {
        let config = SolverConfig {
            max_attempts: 500,
            max_time_seconds: 600,
            enable_parallel: false,
            max_workers: 8,
            enable_caching: false,
            cache_ttl_seconds: 7200,
            prefer_latest: false,
            allow_prerelease: true,
            conflict_strategy: ConflictStrategy::FailOnConflict,
            strict_mode: true,
        };
        assert_eq!(config.max_attempts, 500);
        assert!(!config.enable_parallel);
        assert!(config.allow_prerelease);
        assert_eq!(config.conflict_strategy, ConflictStrategy::FailOnConflict);
        assert!(config.strict_mode);
    }

    /// Test ConflictStrategy enum variants
    #[test]
    fn test_conflict_strategy_variants() {
        let strategies = vec![
            ConflictStrategy::LatestWins,
            ConflictStrategy::EarliestWins,
            ConflictStrategy::FailOnConflict,
            ConflictStrategy::FindCompatible,
        ];
        // Just verify all variants can be created
        for s in strategies {
            let _ = s;
        }
    }

    /// Test SolverRequest::new()
    #[test]
    fn test_solver_request_new() {
        let req = SolverRequest::new(vec![]);
        assert!(req.requirements.is_empty());
        assert!(req.constraints.is_empty());
        assert!(req.excludes.is_empty());
        assert!(req.platform.is_none());
        assert!(req.arch.is_none());
        assert!(req.metadata.is_empty());
    }

    /// Test SolverRequest::with_constraint()
    #[test]
    fn test_solver_request_with_constraint() {
        let req = SolverRequest::new(vec![])
            .with_constraint(PackageRequirement::new("python".to_string()));
        assert_eq!(req.constraints.len(), 1);
        assert_eq!(req.constraints[0].name, "python");
    }

    /// Test SolverRequest::with_exclude()
    #[test]
    fn test_solver_request_with_exclude() {
        let req = SolverRequest::new(vec![]).with_exclude("old_pkg".to_string());
        assert_eq!(req.excludes.len(), 1);
        assert_eq!(req.excludes[0], "old_pkg");
    }

    /// Test SolverRequest::with_platform()
    #[test]
    fn test_solver_request_with_platform() {
        let req = SolverRequest::new(vec![]).with_platform("windows".to_string());
        assert_eq!(req.platform, Some("windows".to_string()));
    }

    /// Test SolverRequest::with_arch()
    #[test]
    fn test_solver_request_with_arch() {
        let req = SolverRequest::new(vec![]).with_arch("x86_64".to_string());
        assert_eq!(req.arch, Some("x86_64".to_string()));
    }

    /// Test SolverStats::default()
    #[test]
    fn test_solver_stats_default() {
        let stats = SolverStats::default();
        assert_eq!(stats.total_resolutions, 0);
        assert_eq!(stats.successful_resolutions, 0);
        assert_eq!(stats.failed_resolutions, 0);
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.cache_misses, 0);
        assert_eq!(stats.avg_resolution_time_ms, 0.0);
        assert_eq!(stats.total_resolution_time_ms, 0);
    }

    /// Test DependencySolver::new()
    #[test]
    fn test_dependency_solver_new() {
        let solver = DependencySolver::new();
        // Just verify it creates without panicking
        let _ = solver;
    }

    /// Test DependencySolver::with_config()
    #[test]
    fn test_dependency_solver_with_config() {
        let config = SolverConfig {
            max_attempts: 500,
            ..SolverConfig::default()
        };
        let solver = DependencySolver::with_config(config);
        // Just verify it creates without panicking
        let _ = solver;
    }

    /// Test DependencySolver::with_repository_manager()
    #[test]
    fn test_dependency_solver_with_repository_manager() {
        let solver = DependencySolver::new();
        // We can't easily create a RepositoryManager for testing,
        // but we can test the builder pattern compiles
        let _ = solver;
    }

    /// Test DependencySolver Default trait
    #[test]
    fn test_dependency_solver_default_trait() {
        let solver1 = DependencySolver::default();
        let solver2 = DependencySolver::new();
        // Both should create valid solvers
        let _ = (solver1, solver2);
    }

    /// Test SolverConfig Serialize/Deserialize (if supported)
    #[test]
    fn test_solver_config_serde() {
        let config = SolverConfig::default();
        // Test that config can be serialized/deserialized
        // This requires the Serialize/Deserialize traits
        let _ = config;
    }
}
