//! CMake build system implementation

use crate::{BuildEnvironment, BuildRequest, BuildStep, BuildStepResult};
use rez_next_common::RezCoreError;
use rez_next_context::ShellExecutor;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::Mutex;

/// CMake build system
#[derive(Debug, Default)]
pub struct CMakeBuildSystem;

impl CMakeBuildSystem {
    pub fn new() -> Self {
        Self
    }

    pub async fn configure(
        &self,
        request: &BuildRequest,
        environment: &BuildEnvironment,
    ) -> Result<BuildStepResult, RezCoreError> {
        let build_dir = environment.get_build_dir();
        let install_dir = environment.get_install_dir();

        let mut args = vec![
            "-S".to_string(),
            request.source_dir.to_string_lossy().to_string(),
            "-B".to_string(),
            build_dir.to_string_lossy().to_string(),
            format!("-DCMAKE_INSTALL_PREFIX={}", install_dir.to_string_lossy()),
        ];

        if request.options.release_mode {
            args.push("-DCMAKE_BUILD_TYPE=Release".to_string());
        } else {
            args.push("-DCMAKE_BUILD_TYPE=Debug".to_string());
        }

        args.extend(request.options.build_args.clone());

        let executor = ShellExecutor::with_shell(rez_next_context::ShellType::detect())
            .with_environment(environment.get_env_vars().clone())
            .with_working_directory(request.source_dir.clone());

        let command = format!("cmake {}", args.join(" "));
        let result = executor.execute(&command).await?;

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
        let build_dir = environment.get_build_dir();

        let mut args = vec![
            "--build".to_string(),
            build_dir.to_string_lossy().to_string(),
        ];

        if request.options.release_mode {
            args.extend(vec!["--config".to_string(), "Release".to_string()]);
        }

        let executor = ShellExecutor::with_shell(rez_next_context::ShellType::detect())
            .with_environment(environment.get_env_vars().clone())
            .with_working_directory(request.source_dir.clone());

        let command = format!("cmake {}", args.join(" "));
        let result = executor.execute(&command).await?;

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
        _request: &BuildRequest,
        environment: &BuildEnvironment,
        _child_process: Arc<Mutex<Option<Child>>>,
    ) -> Result<BuildStepResult, RezCoreError> {
        let build_dir = environment.get_build_dir();

        let executor = ShellExecutor::with_shell(rez_next_context::ShellType::detect())
            .with_environment(environment.get_env_vars().clone())
            .with_working_directory(build_dir.clone());

        let command = "ctest --output-on-failure";
        let result = executor.execute(command).await?;

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
            output: "CMake packaging handled during install".to_string(),
            errors: String::new(),
            duration_ms: 0,
        })
    }

    pub async fn install(
        &self,
        _request: &BuildRequest,
        environment: &BuildEnvironment,
    ) -> Result<BuildStepResult, RezCoreError> {
        let build_dir = environment.get_build_dir();

        let executor = ShellExecutor::with_shell(rez_next_context::ShellType::detect())
            .with_environment(environment.get_env_vars().clone())
            .with_working_directory(build_dir.clone());

        let command = "cmake --install .";
        let result = executor.execute(command).await?;

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
    fn test_cmake_build_system_new() {
        let cmake = CMakeBuildSystem::new();
        assert!(format!("{:?}", cmake).contains("CMakeBuildSystem"));
    }

    #[test]
    fn test_cmake_build_system_default() {
        let _cmake = CMakeBuildSystem::default();
    }
}
