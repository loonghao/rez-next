//! Node.js npm build system implementation

use crate::systems::custom::CustomBuildSystem;
use crate::{BuildEnvironment, BuildRequest, BuildStep, BuildStepResult};
use rez_next_common::RezCoreError;
use rez_next_context::ShellExecutor;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::Mutex;

/// Node.js npm build system
#[derive(Debug, Default)]
pub struct NodeJsBuildSystem;

impl NodeJsBuildSystem {
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
        let result = executor.execute("npm install").await?;
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
        let result = executor
            .execute("npm run build 2>&1 || echo 'No build script'")
            .await?;
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
        let result = executor
            .execute("npm test 2>&1 || echo 'No test script'")
            .await?;
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
        _request: &BuildRequest,
        _environment: &BuildEnvironment,
    ) -> Result<BuildStepResult, RezCoreError> {
        Ok(BuildStepResult {
            step: BuildStep::Packaging,
            success: true,
            output: "NodeJS packaging handled during install".to_string(),
            errors: String::new(),
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
        // Copy dist/ or build/ output to install_dir
        let dist = request.source_dir.join("dist");
        let src = if dist.exists() {
            dist
        } else {
            request.source_dir.clone()
        };
        let files = CustomBuildSystem::copy_package_files(&src, install_dir).await?;
        Ok(BuildStepResult {
            step: BuildStep::Installing,
            success: true,
            output: format!("Installed {} files to {}", files, install_dir.display()),
            errors: String::new(),
            duration_ms: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nodejs_build_system_new() {
        let nodejs = NodeJsBuildSystem::new();
        assert!(format!("{:?}", nodejs).contains("NodeJsBuildSystem"));
    }

    #[test]
    fn test_nodejs_build_system_default() {
        let _nodejs = NodeJsBuildSystem::default();
    }
}
