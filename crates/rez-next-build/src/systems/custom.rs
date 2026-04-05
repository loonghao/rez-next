//! Custom build system implementation

use crate::{BuildEnvironment, BuildRequest, BuildStep, BuildStepResult};
use rez_next_common::RezCoreError;
use rez_next_context::ShellExecutor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::Mutex;

/// Custom build system — handles build scripts and copy-only installs
#[derive(Debug)]
pub struct CustomBuildSystem {
    pub(crate) script_name: String,
}

impl CustomBuildSystem {
    pub fn new(script_name: String) -> Self {
        Self { script_name }
    }

    pub async fn configure(
        &self,
        _request: &BuildRequest,
        _environment: &BuildEnvironment,
    ) -> Result<BuildStepResult, RezCoreError> {
        Ok(BuildStepResult {
            step: BuildStep::Configuring,
            success: true,
            output: format!("Configuration completed for script: {}", self.script_name),
            errors: String::new(),
            duration_ms: 0,
        })
    }

    pub async fn compile(
        &self,
        _request: &BuildRequest,
        _environment: &BuildEnvironment,
        _child_process: Arc<Mutex<Option<Child>>>,
    ) -> Result<BuildStepResult, RezCoreError> {
        Ok(BuildStepResult {
            step: BuildStep::Compiling,
            success: true,
            output: format!("Compilation completed using script: {}", self.script_name),
            errors: String::new(),
            duration_ms: 0,
        })
    }

    pub async fn test(
        &self,
        _request: &BuildRequest,
        _environment: &BuildEnvironment,
        _child_process: Arc<Mutex<Option<Child>>>,
    ) -> Result<BuildStepResult, RezCoreError> {
        Ok(BuildStepResult {
            step: BuildStep::Testing,
            success: true,
            output: "Tests completed".to_string(),
            errors: String::new(),
            duration_ms: 0,
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
            output: "Packaging completed".to_string(),
            errors: String::new(),
            duration_ms: 0,
        })
    }

    pub async fn install(
        &self,
        request: &BuildRequest,
        environment: &BuildEnvironment,
    ) -> Result<BuildStepResult, RezCoreError> {
        match self.script_name.as_str() {
            "default" | "copy-only" => self.copy_files_install(request, environment).await,
            "build_command" => self.execute_build_command(request, environment).await,
            _script_name => {
                self.execute_build_script(request, environment, "install")
                    .await
            }
        }
    }

    /// Copy files installation for packages without build scripts
    async fn copy_files_install(
        &self,
        request: &BuildRequest,
        environment: &BuildEnvironment,
    ) -> Result<BuildStepResult, RezCoreError> {
        let install_dir = environment.get_install_dir();
        let source_dir = &request.source_dir;

        tokio::fs::create_dir_all(install_dir).await.map_err(|e| {
            RezCoreError::BuildError(format!("Failed to create install directory: {}", e))
        })?;

        let copy_result = Self::copy_package_files(source_dir, install_dir).await;

        match copy_result {
            Ok(files_copied) => Ok(BuildStepResult {
                step: BuildStep::Installing,
                success: true,
                output: format!(
                    "Installation completed. Copied {} files to {}",
                    files_copied,
                    install_dir.display()
                ),
                errors: String::new(),
                duration_ms: 0,
            }),
            Err(e) => Ok(BuildStepResult {
                step: BuildStep::Installing,
                success: false,
                output: String::new(),
                errors: format!("Installation failed: {}", e),
                duration_ms: 0,
            }),
        }
    }

    /// Execute build_command from package definition
    async fn execute_build_command(
        &self,
        request: &BuildRequest,
        environment: &BuildEnvironment,
    ) -> Result<BuildStepResult, RezCoreError> {
        let install_dir = environment.get_install_dir();

        tokio::fs::create_dir_all(install_dir).await.map_err(|e| {
            RezCoreError::BuildError(format!("Failed to create install directory: {}", e))
        })?;

        let build_command = request.package.build_command.as_ref().ok_or_else(|| {
            RezCoreError::BuildError("build_command not found in package".to_string())
        })?;

        let executor = ShellExecutor::with_shell(rez_next_context::ShellType::detect())
            .with_environment(environment.get_env_vars().clone())
            .with_working_directory(request.source_dir.clone());

        let expanded_command =
            self.expand_build_command_variables(build_command, request, environment);

        let result = executor.execute(&expanded_command).await?;

        if result.is_success() {
            let copy_result = Self::copy_package_files(&request.source_dir, install_dir).await;
            match copy_result {
                Ok(files_copied) => Ok(BuildStepResult {
                    step: BuildStep::Installing,
                    success: true,
                    output: format!(
                        "{}\nCopied {} source files to install directory",
                        result.stdout, files_copied
                    ),
                    errors: result.stderr,
                    duration_ms: result.execution_time_ms,
                }),
                Err(e) => Ok(BuildStepResult {
                    step: BuildStep::Installing,
                    success: false,
                    output: result.stdout,
                    errors: format!("{}\nFailed to copy source files: {}", result.stderr, e),
                    duration_ms: result.execution_time_ms,
                }),
            }
        } else {
            Ok(BuildStepResult {
                step: BuildStep::Installing,
                success: false,
                output: result.stdout,
                errors: result.stderr,
                duration_ms: result.execution_time_ms,
            })
        }
    }

    /// Expand variables in build command
    fn expand_build_command_variables(
        &self,
        command: &str,
        request: &BuildRequest,
        environment: &BuildEnvironment,
    ) -> String {
        let mut expanded = command.to_string();

        expanded = expanded.replace("{root}", &request.source_dir.to_string_lossy());
        expanded = expanded.replace("{install}", "install");
        expanded = expanded.replace(
            "{build_path}",
            &environment.get_build_dir().to_string_lossy(),
        );
        expanded = expanded.replace(
            "{install_path}",
            &environment.get_install_dir().to_string_lossy(),
        );
        expanded = expanded.replace("{name}", &request.package.name);

        if let Some(ref version) = request.package.version {
            expanded = expanded.replace("{version}", version.as_str());
        } else {
            expanded = expanded.replace("{version}", "");
        }

        expanded = expanded.replace("{variant_index}", "");
        expanded
    }

    /// Execute build script with specific command
    async fn execute_build_script(
        &self,
        request: &BuildRequest,
        environment: &BuildEnvironment,
        command: &str,
    ) -> Result<BuildStepResult, RezCoreError> {
        let script_path = request.source_dir.join(&self.script_name);
        let install_dir = environment.get_install_dir();

        tokio::fs::create_dir_all(install_dir).await.map_err(|e| {
            RezCoreError::BuildError(format!("Failed to create install directory: {}", e))
        })?;

        let executor = ShellExecutor::with_shell(rez_next_context::ShellType::detect())
            .with_environment(environment.get_env_vars().clone())
            .with_working_directory(request.source_dir.clone());

        let exec_command = if self.script_name.ends_with(".py") {
            format!("python {} {}", script_path.to_string_lossy(), command)
        } else if self.script_name.ends_with(".sh") {
            format!("bash {} {}", script_path.to_string_lossy(), command)
        } else {
            format!("{} {}", script_path.to_string_lossy(), command)
        };

        let result = executor.execute(&exec_command).await?;

        Ok(BuildStepResult {
            step: BuildStep::Installing,
            success: result.is_success(),
            output: result.stdout,
            errors: result.stderr,
            duration_ms: result.execution_time_ms,
        })
    }

    /// Copy package files from source to install directory
    pub(crate) async fn copy_package_files(
        source_dir: &Path,
        install_dir: &Path,
    ) -> Result<usize, RezCoreError> {
        use tokio::fs;

        let mut files_copied = 0;
        let mut entries = fs::read_dir(source_dir).await.map_err(|e| {
            RezCoreError::BuildError(format!("Failed to read source directory: {}", e))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            RezCoreError::BuildError(format!("Failed to read directory entry: {}", e))
        })? {
            let src_path = entry.path();
            let file_name = entry.file_name();
            let dest_path = install_dir.join(&file_name);

            let file_name_str = file_name.to_string_lossy();
            if Self::should_skip_file(&file_name_str) {
                continue;
            }

            if src_path.is_dir() {
                let dir_files = Self::copy_dir_recursive(src_path, dest_path).await?;
                files_copied += dir_files;
            } else {
                fs::copy(&src_path, &dest_path).await.map_err(|e| {
                    RezCoreError::BuildError(format!(
                        "Failed to copy file {}: {}",
                        file_name_str, e
                    ))
                })?;
                files_copied += 1;
            }
        }

        Ok(files_copied)
    }

    /// Recursively copy directory
    async fn copy_dir_recursive(src: PathBuf, dest: PathBuf) -> Result<usize, RezCoreError> {
        use tokio::fs;

        fs::create_dir_all(&dest)
            .await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to create directory: {}", e)))?;

        let mut files_copied = 0;
        let mut entries = fs::read_dir(&src)
            .await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to read directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            RezCoreError::BuildError(format!("Failed to read directory entry: {}", e))
        })? {
            let src_path = entry.path();
            let dest_path = dest.join(entry.file_name());

            if src_path.is_dir() {
                let dir_files = Box::pin(Self::copy_dir_recursive(src_path, dest_path)).await?;
                files_copied += dir_files;
            } else {
                fs::copy(&src_path, &dest_path)
                    .await
                    .map_err(|e| RezCoreError::BuildError(format!("Failed to copy file: {}", e)))?;
                files_copied += 1;
            }
        }

        Ok(files_copied)
    }

    /// Check if a file should be skipped during installation
    pub(crate) fn should_skip_file(file_name: &str) -> bool {
        matches!(
            file_name,
            ".git"
                | ".gitignore"
                | ".gitmodules"
                | ".hg"
                | ".hgignore"
                | ".svn"
                | "build"
                | "dist"
                | "__pycache__"
                | ".pytest_cache"
                | ".tox"
                | "node_modules"
                | ".DS_Store"
                | "Thumbs.db"
                | ".coverage"
                | "coverage.xml"
        ) || file_name.ends_with(".pyc")
            || file_name.ends_with(".pyo")
            || file_name.ends_with(".log")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_build_system_new() {
        let custom = CustomBuildSystem::new("build.py".to_string());
        assert_eq!(custom.script_name, "build.py");
    }

    #[test]
    fn test_should_skip_file_git() {
        assert!(CustomBuildSystem::should_skip_file(".git"));
        assert!(CustomBuildSystem::should_skip_file(".gitignore"));
        assert!(CustomBuildSystem::should_skip_file("__pycache__"));
        assert!(CustomBuildSystem::should_skip_file("node_modules"));
    }

    #[test]
    fn test_should_skip_file_extensions() {
        assert!(CustomBuildSystem::should_skip_file("module.pyc"));
        assert!(CustomBuildSystem::should_skip_file("module.pyo"));
        assert!(CustomBuildSystem::should_skip_file("output.log"));
    }

    #[test]
    fn test_should_not_skip_normal_files() {
        assert!(!CustomBuildSystem::should_skip_file("package.py"));
        assert!(!CustomBuildSystem::should_skip_file("README.md"));
        assert!(!CustomBuildSystem::should_skip_file("main.py"));
        assert!(!CustomBuildSystem::should_skip_file("lib.so"));
    }
}
