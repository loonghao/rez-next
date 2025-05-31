//! Resolved context implementation

use crate::{EnvironmentManager, ShellType};
use rez_core_common::RezCoreError;
use rez_core_package::{Package, PackageRequirement};
use rez_core_solver::{ResolutionResult, DependencySolver, SolverRequest};
use rez_core_version::Version;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Resolved context representing a complete package environment
#[pyclass]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContext {
    /// Unique context identifier
    #[pyo3(get)]
    pub id: String,
    
    /// Context name
    #[pyo3(get)]
    pub name: Option<String>,
    
    /// Original requirements that led to this context
    pub requirements: Vec<PackageRequirement>,
    
    /// Resolved packages in dependency order
    pub resolved_packages: Vec<Package>,
    
    /// Environment variables
    pub environment_vars: HashMap<String, String>,
    
    /// Context metadata
    pub metadata: HashMap<String, String>,
    
    /// Context creation timestamp
    #[pyo3(get)]
    pub created_at: i64,
    
    /// Context suite (if any)
    #[pyo3(get)]
    pub suite: Option<String>,
    
    /// Platform information
    #[pyo3(get)]
    pub platform: Option<String>,
    
    /// Architecture information
    #[pyo3(get)]
    pub arch: Option<String>,
    
    /// Context status
    pub status: ContextStatus,
    
    /// Context configuration
    pub config: ContextConfig,
}

/// Context status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContextStatus {
    /// Context is being resolved
    Resolving,
    /// Context is resolved and ready
    Resolved,
    /// Context resolution failed
    Failed,
    /// Context is cached
    Cached,
}

/// Context configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    /// Whether to include parent environment variables
    pub inherit_parent_env: bool,
    /// Shell type for environment generation
    pub shell_type: ShellType,
    /// Working directory for the context
    pub working_directory: Option<PathBuf>,
    /// Additional environment variables to set
    pub additional_env_vars: HashMap<String, String>,
    /// Variables to unset
    pub unset_vars: Vec<String>,
    /// PATH modification strategy
    pub path_strategy: PathStrategy,
}

/// PATH modification strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PathStrategy {
    /// Prepend to existing PATH
    Prepend,
    /// Append to existing PATH
    Append,
    /// Replace PATH entirely
    Replace,
    /// Don't modify PATH
    NoModify,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            inherit_parent_env: true,
            shell_type: ShellType::Bash,
            working_directory: None,
            additional_env_vars: HashMap::new(),
            unset_vars: Vec::new(),
            path_strategy: PathStrategy::Prepend,
        }
    }
}

#[pymethods]
impl ResolvedContext {
    #[new]
    pub fn new(requirements: Vec<String>) -> PyResult<Self> {
        let parsed_requirements: Result<Vec<PackageRequirement>, _> = requirements
            .iter()
            .map(|req_str| PackageRequirement::parse(req_str))
            .collect();

        let parsed_requirements = parsed_requirements
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        Ok(Self::from_requirements(parsed_requirements))
    }

    /// Get the number of resolved packages
    #[getter]
    pub fn package_count(&self) -> usize {
        self.resolved_packages.len()
    }

    /// Get all package names
    pub fn get_package_names(&self) -> Vec<String> {
        self.resolved_packages.iter().map(|p| p.name.clone()).collect()
    }

    /// Check if a package is included in the context
    pub fn contains_package(&self, name: &str) -> bool {
        self.resolved_packages.iter().any(|p| p.name == name)
    }

    /// Get a package by name
    pub fn get_package(&self, name: &str) -> Option<Package> {
        self.resolved_packages.iter().find(|p| p.name == name).cloned()
    }

    /// Get environment variable by name
    pub fn get_env_var(&self, name: &str) -> Option<String> {
        self.environment_vars.get(name).cloned()
    }

    /// Set an environment variable
    pub fn set_env_var(&mut self, name: String, value: String) {
        self.environment_vars.insert(name, value);
    }

    /// Get context status as string
    #[getter]
    pub fn status_str(&self) -> String {
        format!("{:?}", self.status)
    }

    /// Check if context is resolved
    #[getter]
    pub fn is_resolved(&self) -> bool {
        self.status == ContextStatus::Resolved
    }

    /// Get string representation
    fn __str__(&self) -> String {
        match &self.name {
            Some(name) => format!("ResolvedContext('{}')", name),
            None => format!("ResolvedContext({})", self.id),
        }
    }

    /// Get representation
    fn __repr__(&self) -> String {
        format!("ResolvedContext(id='{}', packages={})", self.id, self.package_count())
    }
}

impl ResolvedContext {
    /// Create a new resolved context from requirements
    pub fn from_requirements(requirements: Vec<PackageRequirement>) -> Self {
        let id = Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().timestamp();

        Self {
            id,
            name: None,
            requirements,
            resolved_packages: Vec::new(),
            environment_vars: HashMap::new(),
            metadata: HashMap::new(),
            created_at,
            suite: None,
            platform: None,
            arch: None,
            status: ContextStatus::Resolving,
            config: ContextConfig::default(),
        }
    }

    /// Create a resolved context from a resolution result
    pub fn from_resolution_result(
        requirements: Vec<PackageRequirement>,
        resolution: ResolutionResult,
    ) -> Self {
        let mut context = Self::from_requirements(requirements);
        context.resolved_packages = resolution.packages;
        context.status = ContextStatus::Resolved;
        
        // Add resolution metadata
        context.metadata.insert(
            "resolution_time_ms".to_string(),
            resolution.resolution_time_ms.to_string(),
        );
        context.metadata.insert(
            "conflicts_resolved".to_string(),
            resolution.conflicts_resolved.to_string(),
        );

        context
    }

    /// Set context name
    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    /// Set context suite
    pub fn set_suite(&mut self, suite: String) {
        self.suite = Some(suite);
    }

    /// Set platform information
    pub fn set_platform(&mut self, platform: String) {
        self.platform = Some(platform);
    }

    /// Set architecture information
    pub fn set_arch(&mut self, arch: String) {
        self.arch = Some(arch);
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Set context configuration
    pub fn set_config(&mut self, config: ContextConfig) {
        self.config = config;
    }

    /// Generate environment variables for this context
    pub async fn generate_environment(&mut self) -> Result<(), RezCoreError> {
        let env_manager = EnvironmentManager::new(self.config.clone());
        self.environment_vars = env_manager.generate_environment(&self.resolved_packages).await?;
        Ok(())
    }

    /// Get all tools from resolved packages
    pub fn get_all_tools(&self) -> Vec<String> {
        let mut all_tools = Vec::new();
        for package in &self.resolved_packages {
            all_tools.extend(package.tools.iter().cloned());
        }
        all_tools.sort();
        all_tools.dedup();
        all_tools
    }

    /// Get package by name and version
    pub fn get_package_by_version(&self, name: &str, version: &Version) -> Option<&Package> {
        self.resolved_packages.iter().find(|p| {
            p.name == name && p.version.as_ref() == Some(version)
        })
    }

    /// Check if context satisfies a requirement
    pub fn satisfies_requirement(&self, requirement: &PackageRequirement) -> bool {
        for package in &self.resolved_packages {
            if package.name == requirement.name {
                if let Some(ref version) = package.version {
                    return requirement.satisfied_by(version);
                } else {
                    // Package without version satisfies any requirement for that package
                    return true;
                }
            }
        }
        false
    }

    /// Get context summary
    pub fn get_summary(&self) -> ContextSummary {
        let mut package_versions = HashMap::new();
        let mut total_tools = 0;

        for package in &self.resolved_packages {
            if let Some(ref version) = package.version {
                package_versions.insert(package.name.clone(), version.as_str().to_string());
            } else {
                package_versions.insert(package.name.clone(), "latest".to_string());
            }
            total_tools += package.tools.len();
        }

        ContextSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            package_count: self.resolved_packages.len(),
            tool_count: total_tools,
            env_var_count: self.environment_vars.len(),
            status: self.status.clone(),
            created_at: self.created_at,
            package_versions,
        }
    }

    /// Validate the context
    pub fn validate(&self) -> Result<(), RezCoreError> {
        // Check that all requirements are satisfied
        for requirement in &self.requirements {
            if !self.satisfies_requirement(requirement) {
                return Err(RezCoreError::ContextError(
                    format!("Requirement not satisfied: {}", requirement.requirement_string)
                ));
            }
        }

        // Validate all packages
        for package in &self.resolved_packages {
            package.validate().map_err(|e| RezCoreError::ContextError(
                format!("Invalid package {}: {}", package.name, e)
            ))?;
        }

        Ok(())
    }

    /// Clone the context with a new ID
    pub fn clone_with_new_id(&self) -> Self {
        let mut cloned = self.clone();
        cloned.id = Uuid::new_v4().to_string();
        cloned.created_at = chrono::Utc::now().timestamp();
        cloned
    }

    /// Get context fingerprint (for caching)
    pub fn get_fingerprint(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        
        // Hash requirements
        for req in &self.requirements {
            req.requirement_string.hash(&mut hasher);
        }
        
        // Hash platform and arch
        self.platform.hash(&mut hasher);
        self.arch.hash(&mut hasher);
        
        // Hash configuration
        self.config.inherit_parent_env.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

/// Context summary for display and caching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSummary {
    /// Context ID
    pub id: String,
    /// Context name
    pub name: Option<String>,
    /// Number of packages
    pub package_count: usize,
    /// Number of tools
    pub tool_count: usize,
    /// Number of environment variables
    pub env_var_count: usize,
    /// Context status
    pub status: ContextStatus,
    /// Creation timestamp
    pub created_at: i64,
    /// Package versions
    pub package_versions: HashMap<String, String>,
}

/// Context builder for creating contexts with fluent API
#[derive(Debug)]
pub struct ContextBuilder {
    requirements: Vec<PackageRequirement>,
    name: Option<String>,
    suite: Option<String>,
    platform: Option<String>,
    arch: Option<String>,
    config: ContextConfig,
    metadata: HashMap<String, String>,
}

impl ContextBuilder {
    /// Create a new context builder
    pub fn new() -> Self {
        Self {
            requirements: Vec::new(),
            name: None,
            suite: None,
            platform: None,
            arch: None,
            config: ContextConfig::default(),
            metadata: HashMap::new(),
        }
    }

    /// Add a requirement
    pub fn with_requirement(mut self, requirement: PackageRequirement) -> Self {
        self.requirements.push(requirement);
        self
    }

    /// Add multiple requirements
    pub fn with_requirements(mut self, requirements: Vec<PackageRequirement>) -> Self {
        self.requirements.extend(requirements);
        self
    }

    /// Set context name
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Set suite
    pub fn with_suite(mut self, suite: String) -> Self {
        self.suite = Some(suite);
        self
    }

    /// Set platform
    pub fn with_platform(mut self, platform: String) -> Self {
        self.platform = Some(platform);
        self
    }

    /// Set architecture
    pub fn with_arch(mut self, arch: String) -> Self {
        self.arch = Some(arch);
        self
    }

    /// Set configuration
    pub fn with_config(mut self, config: ContextConfig) -> Self {
        self.config = config;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Build the context (without resolving)
    pub fn build(self) -> ResolvedContext {
        let mut context = ResolvedContext::from_requirements(self.requirements);
        context.name = self.name;
        context.suite = self.suite;
        context.platform = self.platform;
        context.arch = self.arch;
        context.config = self.config;
        context.metadata = self.metadata;
        context
    }

    /// Build and resolve the context
    pub async fn build_and_resolve(
        self,
        solver: &DependencySolver,
    ) -> Result<ResolvedContext, RezCoreError> {
        let context = self.build();
        let request = SolverRequest::new(context.requirements.clone());
        let resolution = solver.resolve(request)?;
        Ok(ResolvedContext::from_resolution_result(context.requirements, resolution))
    }
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}
