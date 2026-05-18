//! Custom build system implementation

use crate::{BuildEnvironment, BuildEventKind, BuildRequest, BuildStep, BuildStepResult};
use rez_next_common::RezCoreError;
use rez_next_context::ShellExecutor;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Child;
use tokio::sync::Mutex;

/// Custom build system — handles build scripts and copy-only installs
#[derive(Debug, Clone)]
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
        request: &BuildRequest,
        environment: &BuildEnvironment,
        _child_process: Arc<Mutex<Option<Child>>>,
    ) -> Result<BuildStepResult, RezCoreError> {
        match self.script_name.as_str() {
            "default" | "copy-only" | "build_command" => {}
            _ => {
                return self
                    .execute_build_script(request, environment, "build")
                    .await;
            }
        }

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
            .with_environment(Self::script_env(environment))
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

        let exec_command = if self.script_name.ends_with(".py") {
            return Self::execute_python_build_script(request, environment, &script_path, command)
                .await;
        } else if self.script_name.ends_with(".sh") {
            format!("bash {} {}", script_path.to_string_lossy(), command)
        } else {
            format!("{} {}", script_path.to_string_lossy(), command)
        };

        let executor = ShellExecutor::with_shell(rez_next_context::ShellType::detect())
            .with_environment(Self::script_env(environment))
            .with_working_directory(request.source_dir.clone());
        let result = executor.execute(&exec_command).await?;
        let success = result.is_success();
        let output = format!(
            "Invoking custom build system...\nRunning build command: {}\n{}",
            exec_command, result.stdout
        );

        let errors = if success {
            result.stderr.clone()
        } else {
            Self::format_command_failure(&exec_command, &result)
        };

        Ok(BuildStepResult {
            step: BuildStep::Installing,
            success,
            output,
            errors,
            duration_ms: result.execution_time_ms,
        })
    }

    async fn execute_python_build_script(
        request: &BuildRequest,
        environment: &BuildEnvironment,
        script_path: &Path,
        command: &str,
    ) -> Result<BuildStepResult, RezCoreError> {
        let python = Self::find_host_python()?;
        let started_at = Instant::now();
        let mut process = tokio::process::Command::new(&python);
        process
            .arg(script_path)
            .arg(command)
            .env_clear()
            .envs(Self::script_env(environment))
            .current_dir(&request.source_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let exec_command = format!("{} {} {}", python.display(), script_path.display(), command);
        environment.emit_event(
            step_for_script_command(command),
            BuildEventKind::StepOutput,
            "Invoking custom build system...".to_string(),
        );
        environment.emit_event(
            step_for_script_command(command),
            BuildEventKind::StepOutput,
            format!("Running build command: {}", exec_command),
        );

        let mut child = process.spawn().map_err(|err| {
            RezCoreError::ExecutionError(format!(
                "Failed to run Python build script with {}: {}",
                python.display(),
                err
            ))
        })?;

        let step = step_for_script_command(command);
        let stdout_task = child.stdout.take().map(|stdout| {
            tokio::spawn(read_script_stream(
                stdout,
                environment.clone(),
                step.clone(),
                BuildEventKind::StepOutput,
            ))
        });
        let stderr_task = child.stderr.take().map(|stderr| {
            tokio::spawn(read_script_stream(
                stderr,
                environment.clone(),
                step.clone(),
                BuildEventKind::StepError,
            ))
        });
        let status = child.wait().await.map_err(|err| {
            RezCoreError::ExecutionError(format!(
                "Failed to wait for Python build script with {}: {}",
                python.display(),
                err
            ))
        })?;

        let raw_stdout = match stdout_task {
            Some(task) => task.await.unwrap_or_default(),
            None => String::new(),
        };
        let stderr = match stderr_task {
            Some(task) => task.await.unwrap_or_default(),
            None => String::new(),
        };
        let success = status.success();
        let exit_code = status.code().unwrap_or(-1);
        let stdout = format!(
            "Invoking custom build system...\nRunning build command: {}\n{}",
            exec_command, raw_stdout
        );

        let errors = if success {
            stderr
        } else {
            Self::format_raw_command_failure(&exec_command, exit_code, &raw_stdout, &stderr)
        };

        Ok(BuildStepResult {
            step: BuildStep::Installing,
            success,
            output: stdout,
            errors,
            duration_ms: started_at.elapsed().as_millis() as u64,
        })
    }

    fn find_host_python() -> Result<PathBuf, RezCoreError> {
        if let Ok(path) =
            std::env::var("REZ_NEXT_BUILD_PYTHON").or_else(|_| std::env::var("PYTHON"))
        {
            let path = PathBuf::from(path);
            if path.is_file() {
                return Ok(path);
            }
        }

        let candidates: &[&str] = if cfg!(windows) {
            &["python", "py"]
        } else {
            &["python3", "python"]
        };
        for candidate in candidates {
            let mut command = std::process::Command::new(candidate);
            if *candidate == "py" {
                command.arg("-3");
            }
            let output = command
                .args(["-c", "import sys; print(sys.executable)"])
                .output();
            if let Ok(output) = output {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path.is_empty() {
                        return Ok(PathBuf::from(path));
                    }
                }
            }
        }

        Err(RezCoreError::ExecutionError(
            "No host Python found for rezbuild.py. Set REZ_NEXT_BUILD_PYTHON to a Python executable."
                .to_string(),
        ))
    }

    fn format_command_failure(command: &str, result: &rez_next_context::CommandResult) -> String {
        Self::format_raw_command_failure(command, result.exit_code, &result.stdout, &result.stderr)
    }

    fn format_raw_command_failure(
        command: &str,
        exit_code: i32,
        stdout: &str,
        stderr: &str,
    ) -> String {
        let mut message = format!("Command failed with exit code {}: {}", exit_code, command);

        if !stderr.trim().is_empty() {
            message.push_str("\nstderr:\n");
            message.push_str(stderr.trim_end());
        }

        if !stdout.trim().is_empty() {
            message.push_str("\nstdout:\n");
            message.push_str(stdout.trim_end());
        }

        message
    }

    fn script_env(environment: &BuildEnvironment) -> std::collections::HashMap<String, String> {
        let mut env = environment.get_env_vars().clone();
        Self::add_rez_next_pythonpath(&mut env);
        Self::add_windows_runtime_env(&mut env);
        env
    }

    fn add_rez_next_pythonpath(env: &mut std::collections::HashMap<String, String>) {
        let mut paths = Vec::new();
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .parent()
            .and_then(|path| path.parent())
            .map(Path::to_path_buf)
            .unwrap_or(manifest_dir);

        for candidate in [
            workspace_root.join("crates/rez-next-python/python"),
            workspace_root.join("python"),
        ] {
            if candidate.is_dir() {
                paths.push(candidate.to_string_lossy().to_string());
            }
        }

        if paths.is_empty() {
            return;
        }

        let separator = if cfg!(windows) { ";" } else { ":" };
        if let Some(existing) = env.get("PYTHONPATH").filter(|value| !value.is_empty()) {
            paths.push(existing.clone());
        }
        env.insert("PYTHONPATH".to_string(), paths.join(separator));
    }

    fn add_windows_runtime_env(env: &mut std::collections::HashMap<String, String>) {
        if !cfg!(windows) {
            return;
        }

        for key in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
            if !env.contains_key(key) {
                if let Ok(value) = std::env::var(key) {
                    env.insert(key.to_string(), value);
                }
            }
        }
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

fn step_for_script_command(command: &str) -> BuildStep {
    match command {
        "build" => BuildStep::Compiling,
        "install" => BuildStep::Installing,
        "test" => BuildStep::Testing,
        _ => BuildStep::Installing,
    }
}

async fn read_script_stream<R>(
    reader: R,
    environment: BuildEnvironment,
    step: BuildStep,
    kind: BuildEventKind,
) -> String
where
    R: AsyncRead + Unpin,
{
    let mut lines = BufReader::new(reader).lines();
    let mut output = String::new();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                environment.emit_event(step.clone(), kind.clone(), line.clone());
                output.push_str(&line);
                output.push('\n');
            }
            Ok(None) => break,
            Err(err) => {
                let message = format!("Failed to read build output: {}", err);
                environment.emit_event(step.clone(), BuildEventKind::StepError, message.clone());
                output.push_str(&message);
                output.push('\n');
                break;
            }
        }
    }
    output
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
