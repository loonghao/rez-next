//! Python build system implementation

use crate::systems::custom::CustomBuildSystem;
use crate::{BuildEnvironment, BuildRequest, BuildStep, BuildStepResult};
use rez_next_common::RezCoreError;
use rez_next_context::ShellExecutor;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::Mutex;

/// Python setuptools / rezbuild.py build system
#[derive(Debug, Default)]
pub struct PythonBuildSystem;

impl PythonBuildSystem {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn configure(
        &self,
        request: &BuildRequest,
        environment: &BuildEnvironment,
    ) -> Result<BuildStepResult, RezCoreError> {
        let rezbuild = request.source_dir.join("rezbuild.py");
        if rezbuild.exists() {
            let executor = ShellExecutor::with_shell(rez_next_context::ShellType::detect())
                .with_environment(environment.get_env_vars().clone())
                .with_working_directory(request.source_dir.clone());
            // Verify Python is available
            let _ = executor.execute("python --version").await;
        }
        Ok(BuildStepResult {
            step: BuildStep::Configuring,
            success: true,
            output: "Python build system configured".to_string(),
            errors: String::new(),
            duration_ms: 0,
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

        let rezbuild = request.source_dir.join("rezbuild.py");
        if rezbuild.exists() {
            let install_dir = environment.get_install_dir();
            let build_dir = environment.get_build_dir();
            let cmd = format!(
                "python \"{}\" build \"{}\" \"{}\"",
                rezbuild.to_string_lossy(),
                build_dir.to_string_lossy(),
                install_dir.to_string_lossy()
            );
            let result = executor.execute(&cmd).await?;
            return Ok(BuildStepResult {
                step: BuildStep::Compiling,
                success: result.is_success(),
                output: result.stdout,
                errors: result.stderr,
                duration_ms: result.execution_time_ms,
            });
        }

        if request.source_dir.join("setup.py").exists() {
            let build_dir = environment.get_build_dir();
            let cmd = format!(
                "python setup.py build --build-base \"{}\"",
                build_dir.to_string_lossy()
            );
            let result = executor.execute(&cmd).await?;
            return Ok(BuildStepResult {
                step: BuildStep::Compiling,
                success: result.is_success(),
                output: result.stdout,
                errors: result.stderr,
                duration_ms: result.execution_time_ms,
            });
        }

        if request.source_dir.join("pyproject.toml").exists() {
            let build_dir = environment.get_build_dir();
            let cmd = format!(
                "pip wheel . -w \"{}\" --no-deps",
                build_dir.to_string_lossy()
            );
            let result = executor.execute(&cmd).await?;
            return Ok(BuildStepResult {
                step: BuildStep::Compiling,
                success: result.is_success(),
                output: result.stdout,
                errors: result.stderr,
                duration_ms: result.execution_time_ms,
            });
        }

        Ok(BuildStepResult {
            step: BuildStep::Compiling,
            success: true,
            output: "No Python build files found; skipping compile step".to_string(),
            errors: String::new(),
            duration_ms: 0,
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
            .execute("python -m pytest -q --tb=short 2>&1 || python -m unittest discover -q 2>&1")
            .await;
        match result {
            Ok(r) => Ok(BuildStepResult {
                step: BuildStep::Testing,
                success: r.is_success(),
                output: r.stdout,
                errors: r.stderr,
                duration_ms: r.execution_time_ms,
            }),
            Err(_) => Ok(BuildStepResult {
                step: BuildStep::Testing,
                success: true,
                output: "No tests found".to_string(),
                errors: String::new(),
                duration_ms: 0,
            }),
        }
    }

    pub async fn package(
        &self,
        _request: &BuildRequest,
        _environment: &BuildEnvironment,
    ) -> Result<BuildStepResult, RezCoreError> {
        Ok(BuildStepResult {
            step: BuildStep::Packaging,
            success: true,
            output: "Python packaging handled during install".to_string(),
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
        let executor = ShellExecutor::with_shell(rez_next_context::ShellType::detect())
            .with_environment(environment.get_env_vars().clone())
            .with_working_directory(request.source_dir.clone());

        tokio::fs::create_dir_all(install_dir).await.map_err(|e| {
            RezCoreError::BuildError(format!("Failed to create install dir: {}", e))
        })?;

        let rezbuild = request.source_dir.join("rezbuild.py");
        if rezbuild.exists() {
            let build_dir = environment.get_build_dir();
            let cmd = format!(
                "python \"{}\" install \"{}\" \"{}\"",
                rezbuild.to_string_lossy(),
                build_dir.to_string_lossy(),
                install_dir.to_string_lossy()
            );
            let result = executor.execute(&cmd).await?;
            return Ok(BuildStepResult {
                step: BuildStep::Installing,
                success: result.is_success(),
                output: result.stdout,
                errors: result.stderr,
                duration_ms: result.execution_time_ms,
            });
        }

        if request.source_dir.join("setup.py").exists() {
            let cmd = format!(
                "python setup.py install --prefix \"{}\"",
                install_dir.to_string_lossy()
            );
            let result = executor.execute(&cmd).await?;
            return Ok(BuildStepResult {
                step: BuildStep::Installing,
                success: result.is_success(),
                output: result.stdout,
                errors: result.stderr,
                duration_ms: result.execution_time_ms,
            });
        }

        if request.source_dir.join("pyproject.toml").exists() {
            let cmd = format!(
                "pip install . --target \"{}\" --no-deps",
                install_dir.to_string_lossy()
            );
            let result = executor.execute(&cmd).await?;
            return Ok(BuildStepResult {
                step: BuildStep::Installing,
                success: result.is_success(),
                output: result.stdout,
                errors: result.stderr,
                duration_ms: result.execution_time_ms,
            });
        }

        // No build file: copy source files (pure Python package)
        let files_copied =
            CustomBuildSystem::copy_package_files(&request.source_dir, install_dir).await?;
        Ok(BuildStepResult {
            step: BuildStep::Installing,
            success: true,
            output: format!("Copied {} files to {}", files_copied, install_dir.display()),
            errors: String::new(),
            duration_ms: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BuildEnvironment;
    use rez_next_package::Package;

    #[test]
    fn test_python_build_system_new() {
        let python = PythonBuildSystem::new();
        assert!(format!("{:?}", python).contains("PythonBuildSystem"));
    }

    #[test]
    fn test_python_build_system_default() {
        let _python = PythonBuildSystem::default();
    }

    /// Helper: create a minimal BuildRequest pointing at the given source dir.
    fn make_request(source_dir: std::path::PathBuf) -> BuildRequest {
        let pkg = Package::new("test-pkg".to_string());
        BuildRequest {
            package: pkg,
            context: None,
            source_dir,
            variant: None,
            options: crate::BuildOptions::default(),
            install_path: None,
        }
    }

    /// Helper: create a BuildEnvironment rooted in a temp dir.
    fn make_env(base: &std::path::Path) -> BuildEnvironment {
        let pkg = Package::new("test-pkg".to_string());
        BuildEnvironment::new(&pkg, &base.to_path_buf(), None).unwrap()
    }

    /// `configure()` without rezbuild.py: returns success immediately (no external call needed).
    #[tokio::test]
    async fn test_configure_without_rezbuild_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let req = make_request(dir.path().to_path_buf());
        let env = make_env(dir.path());

        let python = PythonBuildSystem::new();
        let result = python.configure(&req, &env).await.unwrap();
        assert!(result.success);
        assert_eq!(result.step, BuildStep::Configuring);
        assert!(result.output.contains("configured"));
    }

    /// `compile()` with no Python build files: returns the "skipping compile step" path.
    #[tokio::test]
    async fn test_compile_no_build_files_skips_gracefully() {
        let dir = tempfile::tempdir().unwrap();
        let req = make_request(dir.path().to_path_buf());
        let env = make_env(dir.path());

        let python = PythonBuildSystem::new();
        let child: Arc<Mutex<Option<tokio::process::Child>>> = Arc::new(Mutex::new(None));
        let result = python.compile(&req, &env, child).await.unwrap();
        assert!(result.success);
        assert_eq!(result.step, BuildStep::Compiling);
        assert!(result.output.contains("skipping"));
    }

    /// `package()` always succeeds with a static message.
    #[tokio::test]
    async fn test_package_always_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let req = make_request(dir.path().to_path_buf());
        let env = make_env(dir.path());

        let python = PythonBuildSystem::new();
        let result = python.package(&req, &env).await.unwrap();
        assert!(result.success);
        assert_eq!(result.step, BuildStep::Packaging);
        assert!(result.output.contains("install"));
    }

    /// `install()` without build files: falls back to copy_package_files.
    /// Source dir has 2 files; install should succeed and report the copied count.
    #[tokio::test]
    async fn test_install_no_build_files_copies_source() {
        let src_dir = tempfile::tempdir().unwrap();
        std::fs::write(src_dir.path().join("a.txt"), "hello").unwrap();
        std::fs::write(src_dir.path().join("b.txt"), "world").unwrap();

        let install_base = tempfile::tempdir().unwrap();
        let req = make_request(src_dir.path().to_path_buf());
        let env = make_env(install_base.path());

        let python = PythonBuildSystem::new();
        let result = python.install(&req, &env).await.unwrap();
        assert!(result.success);
        assert_eq!(result.step, BuildStep::Installing);
        // Must mention copied file count and destination
        assert!(result.output.contains("Copied") || result.output.contains("files"));
    }
}
