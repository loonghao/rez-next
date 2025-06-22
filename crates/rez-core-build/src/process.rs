//! Build process management

use crate::{BuildRequest, BuildResult, BuildConfig, BuildStatus, BuildEnvironment, BuildArtifacts, BuildSystem};
use rez_core_common::RezCoreError;
use rez_core_context::ShellExecutor;
#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use tokio::process::Child;

/// Build process for managing individual package builds
#[cfg_attr(feature = "python-bindings", pyclass)]
#[derive(Debug)]
pub struct BuildProcess {
    /// Build ID
    build_id: String,
    /// Build request
    request: BuildRequest,
    /// Build environment
    environment: BuildEnvironment,
    /// Build configuration
    config: BuildConfig,
    /// Build status
    status: Arc<RwLock<BuildStatus>>,
    /// Build output
    output: Arc<Mutex<String>>,
    /// Build errors
    errors: Arc<Mutex<String>>,
    /// Start time
    start_time: Option<std::time::Instant>,
    /// Child process (if running)
    child_process: Arc<Mutex<Option<Child>>>,
}

/// Build step enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildStep {
    /// Preparing build environment
    Preparing,
    /// Configuring build
    Configuring,
    /// Compiling source code
    Compiling,
    /// Running tests
    Testing,
    /// Packaging artifacts
    Packaging,
    /// Installing artifacts
    Installing,
    /// Cleaning up
    Cleanup,
}

/// Build step result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildStepResult {
    /// Build step
    pub step: BuildStep,
    /// Whether step was successful
    pub success: bool,
    /// Step output
    pub output: String,
    /// Step errors
    pub errors: String,
    /// Step duration in milliseconds
    pub duration_ms: u64,
}

#[cfg(feature = "python-bindings")]
#[pymethods]
impl BuildProcess {
    /// Get build ID
    #[getter]
    pub fn build_id(&self) -> String {
        self.build_id.clone()
    }

    /// Get build status
    #[getter]
    pub fn status(&self) -> String {
        // This is a simplified sync version for Python binding
        format!("{:?}", BuildStatus::Running) // TODO: Implement proper sync access
    }

    /// Get package name
    #[getter]
    pub fn package_name(&self) -> String {
        self.request.package.name.clone()
    }
}

impl BuildProcess {
    /// Create a new build process
    pub fn new(
        build_id: String,
        request: BuildRequest,
        environment: BuildEnvironment,
        config: BuildConfig,
    ) -> Self {
        Self {
            build_id,
            request,
            environment,
            config,
            status: Arc::new(RwLock::new(BuildStatus::Queued)),
            output: Arc::new(Mutex::new(String::new())),
            errors: Arc::new(Mutex::new(String::new())),
            start_time: None,
            child_process: Arc::new(Mutex::new(None)),
        }
    }

    /// Start the build process
    pub async fn start(&mut self) -> Result<(), RezCoreError> {
        {
            let mut status = self.status.write().await;
            *status = BuildStatus::Running;
        }

        self.start_time = Some(std::time::Instant::now());

        // Start build in background
        let build_id = self.build_id.clone();
        let request = self.request.clone();
        let environment = self.environment.clone();
        let config = self.config.clone();
        let status = self.status.clone();
        let output = self.output.clone();
        let errors = self.errors.clone();
        let child_process = self.child_process.clone();

        tokio::spawn(async move {
            let result = Self::run_build_steps(
                &build_id,
                &request,
                &environment,
                &config,
                output.clone(),
                errors.clone(),
                child_process.clone(),
            ).await;

            let mut status_guard = status.write().await;
            *status_guard = match result {
                Ok(_) => BuildStatus::Success,
                Err(_) => BuildStatus::Failed,
            };
        });

        Ok(())
    }

    /// Wait for the build to complete
    pub async fn wait(&mut self) -> Result<BuildResult, RezCoreError> {
        // Wait for build to complete
        loop {
            let status = {
                let status_guard = self.status.read().await;
                status_guard.clone()
            };

            match status {
                BuildStatus::Success | BuildStatus::Failed | BuildStatus::Cancelled => break,
                _ => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }

        // Collect results
        let duration_ms = self.start_time
            .map(|start| start.elapsed().as_millis() as u64)
            .unwrap_or(0);

        let output = {
            let output_guard = self.output.lock().await;
            output_guard.clone()
        };

        let errors = {
            let errors_guard = self.errors.lock().await;
            errors_guard.clone()
        };

        let status = {
            let status_guard = self.status.read().await;
            status_guard.clone()
        };

        let success = status == BuildStatus::Success;

        // Create artifacts (simplified)
        let artifacts = if success {
            BuildArtifacts::new(self.environment.get_install_dir().clone())
        } else {
            BuildArtifacts::default()
        };

        let result = if success {
            BuildResult::success(self.build_id.clone(), artifacts, duration_ms)
        } else {
            BuildResult::failure(self.build_id.clone(), errors, duration_ms)
        };

        Ok(result.with_output(output))
    }

    /// Cancel the build process
    pub async fn cancel(&mut self) -> Result<(), RezCoreError> {
        {
            let mut status = self.status.write().await;
            *status = BuildStatus::Cancelled;
        }

        // Kill child process if running
        let mut child_guard = self.child_process.lock().await;
        if let Some(ref mut child) = child_guard.as_mut() {
            let _ = child.kill().await;
        }

        Ok(())
    }

    /// Get current build status
    pub fn get_status(&self) -> BuildStatus {
        // Use try_read to avoid blocking
        match self.status.try_read() {
            Ok(status) => status.clone(),
            Err(_) => BuildStatus::Running, // If we can't read, assume still running
        }
    }

    /// Run build steps
    async fn run_build_steps(
        build_id: &str,
        request: &BuildRequest,
        environment: &BuildEnvironment,
        config: &BuildConfig,
        output: Arc<Mutex<String>>,
        errors: Arc<Mutex<String>>,
        child_process: Arc<Mutex<Option<Child>>>,
    ) -> Result<(), RezCoreError> {
        let steps = vec![
            BuildStep::Preparing,
            BuildStep::Configuring,
            BuildStep::Compiling,
            BuildStep::Testing,
            BuildStep::Packaging,
            BuildStep::Installing,
            BuildStep::Cleanup,
        ];

        for step in steps {
            let step_result = Self::execute_build_step(
                &step,
                request,
                environment,
                config,
                child_process.clone(),
            ).await?;

            // Append output
            {
                let mut output_guard = output.lock().await;
                output_guard.push_str(&format!("=== {:?} ===\n", step));
                output_guard.push_str(&step_result.output);
                output_guard.push('\n');
            }

            // Append errors if any
            if !step_result.errors.is_empty() {
                let mut errors_guard = errors.lock().await;
                errors_guard.push_str(&format!("=== {:?} Errors ===\n", step));
                errors_guard.push_str(&step_result.errors);
                errors_guard.push('\n');
            }

            // Stop on failure
            if !step_result.success {
                return Err(RezCoreError::BuildError(
                    format!("Build step {:?} failed: {}", step, step_result.errors)
                ));
            }
        }

        Ok(())
    }

    /// Execute a single build step
    async fn execute_build_step(
        step: &BuildStep,
        request: &BuildRequest,
        environment: &BuildEnvironment,
        config: &BuildConfig,
        child_process: Arc<Mutex<Option<Child>>>,
    ) -> Result<BuildStepResult, RezCoreError> {
        let start_time = std::time::Instant::now();

        let (success, output, errors) = match step {
            BuildStep::Preparing => {
                Self::execute_prepare_step(request, environment, config).await?
            }
            BuildStep::Configuring => {
                Self::execute_configure_step(request, environment, config).await?
            }
            BuildStep::Compiling => {
                Self::execute_compile_step(request, environment, config, child_process).await?
            }
            BuildStep::Testing => {
                if request.options.skip_tests {
                    (true, "Tests skipped".to_string(), String::new())
                } else {
                    Self::execute_test_step(request, environment, config, child_process).await?
                }
            }
            BuildStep::Packaging => {
                Self::execute_package_step(request, environment, config).await?
            }
            BuildStep::Installing => {
                Self::execute_install_step(request, environment, config).await?
            }
            BuildStep::Cleanup => {
                Self::execute_cleanup_step(request, environment, config).await?
            }
        };

        let duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(BuildStepResult {
            step: step.clone(),
            success,
            output,
            errors,
            duration_ms,
        })
    }

    /// Execute prepare step
    async fn execute_prepare_step(
        request: &BuildRequest,
        environment: &BuildEnvironment,
        config: &BuildConfig,
    ) -> Result<(bool, String, String), RezCoreError> {
        // Create build directories
        let build_dir = environment.get_build_dir();
        let install_dir = environment.get_install_dir();

        tokio::fs::create_dir_all(build_dir).await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to create build dir: {}", e)))?;

        tokio::fs::create_dir_all(install_dir).await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to create install dir: {}", e)))?;

        let output = format!(
            "Created build directory: {}\nCreated install directory: {}",
            build_dir.display(),
            install_dir.display()
        );

        Ok((true, output, String::new()))
    }

    /// Execute configure step
    async fn execute_configure_step(
        request: &BuildRequest,
        environment: &BuildEnvironment,
        config: &BuildConfig,
    ) -> Result<(bool, String, String), RezCoreError> {
        // Detect and configure build system
        let build_system = BuildSystem::detect_with_package(&request.source_dir, &request.package)?;
        let configure_result = build_system.configure(request, environment).await?;

        Ok((configure_result.success, configure_result.output, configure_result.errors))
    }

    /// Execute compile step
    async fn execute_compile_step(
        request: &BuildRequest,
        environment: &BuildEnvironment,
        config: &BuildConfig,
        child_process: Arc<Mutex<Option<Child>>>,
    ) -> Result<(bool, String, String), RezCoreError> {
        let build_system = BuildSystem::detect_with_package(&request.source_dir, &request.package)?;
        let compile_result = build_system.compile(request, environment, child_process).await?;

        Ok((compile_result.success, compile_result.output, compile_result.errors))
    }

    /// Execute test step
    async fn execute_test_step(
        request: &BuildRequest,
        environment: &BuildEnvironment,
        config: &BuildConfig,
        child_process: Arc<Mutex<Option<Child>>>,
    ) -> Result<(bool, String, String), RezCoreError> {
        let build_system = BuildSystem::detect_with_package(&request.source_dir, &request.package)?;
        let test_result = build_system.test(request, environment, child_process).await?;

        Ok((test_result.success, test_result.output, test_result.errors))
    }

    /// Execute package step
    async fn execute_package_step(
        request: &BuildRequest,
        environment: &BuildEnvironment,
        config: &BuildConfig,
    ) -> Result<(bool, String, String), RezCoreError> {
        let build_system = BuildSystem::detect_with_package(&request.source_dir, &request.package)?;
        let package_result = build_system.package(request, environment).await?;

        Ok((package_result.success, package_result.output, package_result.errors))
    }

    /// Execute install step
    async fn execute_install_step(
        request: &BuildRequest,
        environment: &BuildEnvironment,
        config: &BuildConfig,
    ) -> Result<(bool, String, String), RezCoreError> {
        let build_system = BuildSystem::detect_with_package(&request.source_dir, &request.package)?;
        let install_result = build_system.install(request, environment).await?;

        Ok((install_result.success, install_result.output, install_result.errors))
    }

    /// Execute cleanup step
    async fn execute_cleanup_step(
        request: &BuildRequest,
        environment: &BuildEnvironment,
        config: &BuildConfig,
    ) -> Result<(bool, String, String), RezCoreError> {
        // Clean temporary files if configured
        if !config.keep_artifacts {
            let temp_dir = environment.get_temp_dir();
            if temp_dir.exists() {
                tokio::fs::remove_dir_all(temp_dir).await
                    .map_err(|e| RezCoreError::BuildError(format!("Failed to clean temp dir: {}", e)))?;
            }
        }

        Ok((true, "Cleanup completed".to_string(), String::new()))
    }
}
