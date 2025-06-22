//! Build manager and coordination

use crate::{BuildProcess, BuildEnvironment, BuildArtifacts, BuildSystem};
use rez_core_common::RezCoreError;
use rez_core_package::Package;
use rez_core_context::ResolvedContext;
#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Build manager for coordinating package builds
#[cfg_attr(feature = "python-bindings", pyclass)]
#[derive(Debug)]
pub struct BuildManager {
    /// Build configuration
    config: BuildConfig,
    /// Active build processes
    active_builds: HashMap<String, BuildProcess>,
    /// Build statistics
    stats: BuildStats,
}

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Build output directory
    pub build_dir: PathBuf,
    /// Temporary build directory
    pub temp_dir: PathBuf,
    /// Maximum concurrent builds
    pub max_concurrent_builds: usize,
    /// Build timeout in seconds
    pub build_timeout_seconds: u64,
    /// Whether to clean build directory before building
    pub clean_before_build: bool,
    /// Whether to keep build artifacts after successful build
    pub keep_artifacts: bool,
    /// Build verbosity level
    pub verbosity: BuildVerbosity,
    /// Environment variables to pass to build processes
    pub build_env_vars: HashMap<String, String>,
}

/// Build verbosity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BuildVerbosity {
    /// Silent (errors only)
    Silent,
    /// Normal output
    Normal,
    /// Verbose output
    Verbose,
    /// Debug output
    Debug,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            build_dir: PathBuf::from("build"),
            temp_dir: PathBuf::from("tmp"),
            max_concurrent_builds: 4,
            build_timeout_seconds: 3600, // 1 hour
            clean_before_build: false,
            keep_artifacts: true,
            verbosity: BuildVerbosity::Normal,
            build_env_vars: HashMap::new(),
        }
    }
}

/// Build statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildStats {
    /// Total builds started
    pub builds_started: usize,
    /// Successful builds
    pub builds_successful: usize,
    /// Failed builds
    pub builds_failed: usize,
    /// Currently running builds
    pub builds_running: usize,
    /// Total build time in milliseconds
    pub total_build_time_ms: u64,
    /// Average build time in milliseconds
    pub avg_build_time_ms: f64,
}

impl Default for BuildStats {
    fn default() -> Self {
        Self {
            builds_started: 0,
            builds_successful: 0,
            builds_failed: 0,
            builds_running: 0,
            total_build_time_ms: 0,
            avg_build_time_ms: 0.0,
        }
    }
}

/// Build request
#[derive(Debug, Clone)]
pub struct BuildRequest {
    /// Package to build
    pub package: Package,
    /// Build context (resolved dependencies)
    pub context: Option<ResolvedContext>,
    /// Source directory
    pub source_dir: PathBuf,
    /// Build variant (if any)
    pub variant: Option<String>,
    /// Build options
    pub options: BuildOptions,
    /// Installation path (if installing)
    pub install_path: Option<PathBuf>,
}

/// Build options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildOptions {
    /// Force rebuild even if artifacts exist
    pub force_rebuild: bool,
    /// Skip tests during build
    pub skip_tests: bool,
    /// Build in release mode
    pub release_mode: bool,
    /// Additional build arguments
    pub build_args: Vec<String>,
    /// Custom environment variables
    pub env_vars: HashMap<String, String>,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            force_rebuild: false,
            skip_tests: false,
            release_mode: false,
            build_args: Vec::new(),
            env_vars: HashMap::new(),
        }
    }
}

/// Build result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildResult {
    /// Build ID
    pub build_id: String,
    /// Whether build was successful
    pub success: bool,
    /// Build output
    pub output: String,
    /// Build errors
    pub errors: String,
    /// Build artifacts
    pub artifacts: BuildArtifacts,
    /// Build duration in milliseconds
    pub duration_ms: u64,
    /// Build metadata
    pub metadata: HashMap<String, String>,
}

impl BuildResult {
    /// Create a successful build result
    pub fn success(build_id: String, artifacts: BuildArtifacts, duration_ms: u64) -> Self {
        Self {
            build_id,
            success: true,
            output: String::new(),
            errors: String::new(),
            artifacts,
            duration_ms,
            metadata: HashMap::new(),
        }
    }

    /// Create a failed build result
    pub fn failure(build_id: String, errors: String, duration_ms: u64) -> Self {
        Self {
            build_id,
            success: false,
            output: String::new(),
            errors,
            artifacts: BuildArtifacts::default(),
            duration_ms,
            metadata: HashMap::new(),
        }
    }

    /// Add output
    pub fn with_output(mut self, output: String) -> Self {
        self.output = output;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

#[cfg(feature = "python-bindings")]
#[pymethods]
impl BuildManager {
    #[new]
    pub fn new() -> Self {
        Self::with_config(BuildConfig::default())
    }

    /// Start a build
    #[cfg(feature = "python-bindings")]
    pub fn build_package_py(&mut self, package: Package, source_dir: String) -> PyResult<String> {
        let request = BuildRequest {
            package,
            context: None,
            source_dir: PathBuf::from(source_dir),
            variant: None,
            options: BuildOptions::default(),
        };

        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(self.start_build(request));
        
        match result {
            Ok(build_id) => Ok(build_id),
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())),
        }
    }

    /// Get build status
    pub fn get_build_status_py(&self, build_id: &str) -> PyResult<String> {
        match self.get_build_status(build_id) {
            Some(status) => Ok(format!("{:?}", status)),
            None => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("Build not found")),
        }
    }

    /// Get build statistics
    #[getter]
    pub fn stats(&self) -> String {
        serde_json::to_string_pretty(&self.stats).unwrap_or_default()
    }
}

impl BuildManager {
    /// Create a new build manager with default configuration
    pub fn new() -> Self {
        Self::with_config(BuildConfig::default())
    }

    /// Create a new build manager with configuration
    pub fn with_config(config: BuildConfig) -> Self {
        Self {
            config,
            active_builds: HashMap::new(),
            stats: BuildStats::default(),
        }
    }

    /// Start a build process
    pub async fn start_build(&mut self, request: BuildRequest) -> Result<String, RezCoreError> {
        // Check concurrent build limit
        if self.active_builds.len() >= self.config.max_concurrent_builds {
            return Err(RezCoreError::BuildError(
                "Maximum concurrent builds reached".to_string()
            ));
        }

        // Generate build ID
        let build_id = Uuid::new_v4().to_string();

        // Create build environment
        let build_env = BuildEnvironment::with_install_path(
            &request.package,
            &self.config.build_dir,
            request.context.as_ref(),
            request.install_path.as_ref(),
        )?;

        // Create build process
        let mut build_process = BuildProcess::new(
            build_id.clone(),
            request,
            build_env,
            self.config.clone(),
        );

        // Start the build
        build_process.start().await?;

        // Track the build
        self.active_builds.insert(build_id.clone(), build_process);
        self.stats.builds_started += 1;
        self.stats.builds_running += 1;

        Ok(build_id)
    }

    /// Wait for a build to complete
    pub async fn wait_for_build(&mut self, build_id: &str) -> Result<BuildResult, RezCoreError> {
        if let Some(mut build_process) = self.active_builds.remove(build_id) {
            let result = build_process.wait().await?;
            
            // Update statistics
            self.stats.builds_running -= 1;
            if result.success {
                self.stats.builds_successful += 1;
            } else {
                self.stats.builds_failed += 1;
            }
            
            self.stats.total_build_time_ms += result.duration_ms;
            if self.stats.builds_started > 0 {
                self.stats.avg_build_time_ms = 
                    self.stats.total_build_time_ms as f64 / self.stats.builds_started as f64;
            }

            Ok(result)
        } else {
            Err(RezCoreError::BuildError(
                format!("Build {} not found", build_id)
            ))
        }
    }

    /// Cancel a build
    pub async fn cancel_build(&mut self, build_id: &str) -> Result<(), RezCoreError> {
        if let Some(mut build_process) = self.active_builds.remove(build_id) {
            build_process.cancel().await?;
            self.stats.builds_running -= 1;
            self.stats.builds_failed += 1;
            Ok(())
        } else {
            Err(RezCoreError::BuildError(
                format!("Build {} not found", build_id)
            ))
        }
    }

    /// Get build status
    pub fn get_build_status(&self, build_id: &str) -> Option<BuildStatus> {
        self.active_builds.get(build_id).map(|process| process.get_status())
    }

    /// Get all active builds
    pub fn get_active_builds(&self) -> Vec<String> {
        self.active_builds.keys().cloned().collect()
    }

    /// Clean build directory
    pub async fn clean_build_dir(&self) -> Result<(), RezCoreError> {
        if self.config.build_dir.exists() {
            tokio::fs::remove_dir_all(&self.config.build_dir).await
                .map_err(|e| RezCoreError::BuildError(
                    format!("Failed to clean build directory: {}", e)
                ))?;
        }
        
        tokio::fs::create_dir_all(&self.config.build_dir).await
            .map_err(|e| RezCoreError::BuildError(
                format!("Failed to create build directory: {}", e)
            ))?;

        Ok(())
    }

    /// Get build configuration
    pub fn get_config(&self) -> &BuildConfig {
        &self.config
    }

    /// Update build configuration
    pub fn set_config(&mut self, config: BuildConfig) {
        self.config = config;
    }

    /// Get build statistics
    pub fn get_stats(&self) -> &BuildStats {
        &self.stats
    }

    /// Reset build statistics
    pub fn reset_stats(&mut self) {
        self.stats = BuildStats::default();
    }
}

/// Build status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BuildStatus {
    /// Build is queued
    Queued,
    /// Build is running
    Running,
    /// Build completed successfully
    Success,
    /// Build failed
    Failed,
    /// Build was cancelled
    Cancelled,
}

impl Default for BuildManager {
    fn default() -> Self {
        Self::new()
    }
}
