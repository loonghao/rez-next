//! Rust Cargo build system implementation

use crate::{BuildEnvironment, BuildRequest, BuildStep, BuildStepResult};
use rez_next_common::RezCoreError;
use rez_next_context::ShellExecutor;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::Mutex;

/// Rust Cargo build system
#[derive(Debug, Default)]
pub struct CargoBuildSystem;

impl CargoBuildSystem {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn configure(
        &self,
        request: &BuildRequest,
        environment: &BuildEnvironment,
    ) -> Result<BuildStepResult, RezCoreError> {
        let executor = ShellExecutor::with_shell(rez_next_context::ShellType::detect())
            .with_environment(environment.get_env_vars().clone())
            .with_working_directory(request.source_dir.clone());
        let result = executor.execute("cargo check 2>&1").await?;
        Ok(BuildStepResult {
            step: BuildStep::Configuring,
            success: result.is_success(),
            output: result.stdout,
            errors: result.stderr,
            duration_ms: result.execution_time_ms,
        })
    }

    pub async fn compile(
        &self,
        request: &BuildRequest,
        environment: &BuildEnvironment,
        _child_process: Arc<Mutex<Option<Child>>>,
    ) -> Result<BuildStepResult, RezCoreError> {
        let executor = ShellExecutor::with_shell(rez_next_context::ShellType::detect())
            .with_environment(environment.get_env_vars().clone())
            .with_working_directory(request.source_dir.clone());
        let mode = if request.options.release_mode {
            "--release"
        } else {
            ""
        };
        let cmd = format!("cargo build {}", mode).trim().to_string();
        let result = executor.execute(&cmd).await?;
        Ok(BuildStepResult {
            step: BuildStep::Compiling,
            success: result.is_success(),
            output: result.stdout,
            errors: result.stderr,
            duration_ms: result.execution_time_ms,
        })
    }

    pub async fn test(
        &self,
        request: &BuildRequest,
        environment: &BuildEnvironment,
        _child_process: Arc<Mutex<Option<Child>>>,
    ) -> Result<BuildStepResult, RezCoreError> {
        let executor = ShellExecutor::with_shell(rez_next_context::ShellType::detect())
            .with_environment(environment.get_env_vars().clone())
            .with_working_directory(request.source_dir.clone());
        let result = executor.execute("cargo test 2>&1").await?;
        Ok(BuildStepResult {
            step: BuildStep::Testing,
            success: result.is_success(),
            output: result.stdout,
            errors: result.stderr,
            duration_ms: result.execution_time_ms,
        })
    }

    pub async fn package(
        &self,
        request: &BuildRequest,
        environment: &BuildEnvironment,
    ) -> Result<BuildStepResult, RezCoreError> {
        let executor = ShellExecutor::with_shell(rez_next_context::ShellType::detect())
            .with_environment(environment.get_env_vars().clone())
            .with_working_directory(request.source_dir.clone());
        let result = executor.execute("cargo package --no-verify 2>&1").await;
        let (success, output, errors) = match result {
            Ok(r) => (r.is_success(), r.stdout, r.stderr),
            Err(_) => (true, "cargo package skipped".to_string(), String::new()),
        };
        Ok(BuildStepResult {
            step: BuildStep::Packaging,
            success,
            output,
            errors,
            duration_ms: 0,
        })
    }

    pub async fn install(
        &self,
        request: &BuildRequest,
        environment: &BuildEnvironment,
    ) -> Result<BuildStepResult, RezCoreError> {
        let install_dir = environment.get_install_dir();
        tokio::fs::create_dir_all(install_dir).await.map_err(|e| {
            RezCoreError::BuildError(format!("Failed to create install dir: {}", e))
        })?;
        let executor = ShellExecutor::with_shell(rez_next_context::ShellType::detect())
            .with_environment(environment.get_env_vars().clone())
            .with_working_directory(request.source_dir.clone());
        let mode = if request.options.release_mode {
            "--release"
        } else {
            ""
        };
        let cmd = format!(
            "cargo install --path . {} --root \"{}\"",
            mode,
            install_dir.to_string_lossy()
        )
        .trim()
        .to_string();
        let result = executor.execute(&cmd).await?;
        Ok(BuildStepResult {
            step: BuildStep::Installing,
            success: result.is_success(),
            output: result.stdout,
            errors: result.stderr,
            duration_ms: result.execution_time_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cargo_build_system_new() {
        let cargo = CargoBuildSystem::new();
        assert!(format!("{:?}", cargo).contains("CargoBuildSystem"));
    }

    #[test]
    fn test_cargo_build_system_default() {
        let _cargo = CargoBuildSystem::default();
    }
}
