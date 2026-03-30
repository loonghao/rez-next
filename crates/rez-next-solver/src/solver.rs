//! Core solver implementation

#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;
use crate::dependency_resolver::DependencyResolver;
use rez_next_common::RezCoreError;
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_repository::simple_repository::RepositoryManager;
use rez_next_version::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Resolution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionResult {
    /// Resolved packages
    pub packages: Vec<Package>,
    /// Whether conflicts were resolved
    pub conflicts_resolved: bool,
    /// Resolution time in milliseconds
    pub resolution_time_ms: u64,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

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
#[cfg_attr(feature = "python-bindings", pyclass)]
pub struct DependencySolver {
    /// Solver configuration
    config: SolverConfig,
    /// Solver statistics
    stats: SolverStats,
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

// Python methods - conditionally compiled
#[cfg(feature = "python-bindings")]
#[pymethods]
impl DependencySolver {
    #[new]
    pub fn new_py() -> Self {
        let config = SolverConfig::default();
        Self {
            config,
            stats: SolverStats::default(),
        }
    }

    /// Get solver statistics
    #[getter]
    pub fn stats(&self) -> String {
        serde_json::to_string(&self.stats).unwrap_or_else(|_| "{}".to_string())
    }
}

impl DependencySolver {
    /// Create a new solver with default configuration
    pub fn new() -> Self {
        let config = SolverConfig::default();
        Self {
            config,
            stats: SolverStats::default(),
            repository_manager: None,
        }
    }

    /// Create a new solver with custom configuration
    pub fn with_config(config: SolverConfig) -> Self {
        Self {
            config,
            stats: SolverStats::default(),
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
                    req_str.parse::<Requirement>().unwrap_or_else(|_| {
                        Requirement::new(pr.name.clone())
                    })
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
