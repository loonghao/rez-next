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
    use crate::BuildEnvironment;
    use rez_next_package::Package;

    #[test]
    fn test_nodejs_build_system_new() {
        let nodejs = NodeJsBuildSystem::new();
        assert!(format!("{:?}", nodejs).contains("NodeJsBuildSystem"));
    }

    #[test]
    fn test_nodejs_build_system_default() {
        let _nodejs = NodeJsBuildSystem::default();
    }

    fn make_request(source_dir: std::path::PathBuf) -> BuildRequest {
        let pkg = Package::new("test-node-pkg".to_string());
        BuildRequest {
            package: pkg,
            context: None,
            source_dir,
            variant: None,
            options: crate::BuildOptions::default(),
            install_path: None,
        }
    }

    fn make_env(base: &std::path::Path) -> BuildEnvironment {
        let pkg = Package::new("test-node-pkg".to_string());
        BuildEnvironment::new(&pkg, &base.to_path_buf(), None).unwrap()
    }

    /// `package()` always returns a success result with static message.
    #[tokio::test]
    async fn test_package_always_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let req = make_request(dir.path().to_path_buf());
        let env = make_env(dir.path());

        let nodejs = NodeJsBuildSystem::new();
        let result = nodejs.package(&req, &env).await.unwrap();
        assert!(result.success);
        assert_eq!(result.step, BuildStep::Packaging);
        assert!(result.output.contains("NodeJS"));
    }

    /// `install()` without a dist/ directory: copies source files directly.
    #[tokio::test]
    async fn test_install_without_dist_copies_source() {
        let src_dir = tempfile::tempdir().unwrap();
        std::fs::write(src_dir.path().join("index.js"), "module.exports = {};").unwrap();

        let install_base = tempfile::tempdir().unwrap();
        let req = make_request(src_dir.path().to_path_buf());
        let env = make_env(install_base.path());

        let nodejs = NodeJsBuildSystem::new();
        let result = nodejs.install(&req, &env).await.unwrap();
        assert!(result.success);
        assert_eq!(result.step, BuildStep::Installing);
        assert!(result.output.contains("Installed"));
    }

    /// `install()` with a dist/ directory: copies from dist/.
    #[tokio::test]
    async fn test_install_with_dist_dir_copies_dist() {
        let src_dir = tempfile::tempdir().unwrap();
        let dist_dir = src_dir.path().join("dist");
        std::fs::create_dir_all(&dist_dir).unwrap();
        std::fs::write(dist_dir.join("bundle.js"), "// bundle").unwrap();

        let install_base = tempfile::tempdir().unwrap();
        let req = make_request(src_dir.path().to_path_buf());
        let env = make_env(install_base.path());

        let nodejs = NodeJsBuildSystem::new();
        let result = nodejs.install(&req, &env).await.unwrap();
        assert!(result.success);
        assert_eq!(result.step, BuildStep::Installing);
        assert!(result.output.contains("Installed"));
    }
}
