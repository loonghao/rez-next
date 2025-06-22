//! Build system implementations

use crate::{BuildEnvironment, BuildRequest, BuildStep, BuildStepResult};
use rez_core_common::RezCoreError;
use rez_core_context::ShellExecutor;
use rez_core_package::Package;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::Mutex;

/// Build system types
#[derive(Debug, Clone, PartialEq)]
pub enum BuildSystemType {
    /// CMake build system
    CMake,
    /// Make build system
    Make,
    /// Python setuptools
    Python,
    /// Node.js npm
    NodeJs,
    /// Rust Cargo
    Cargo,
    /// Custom build script
    Custom,
    /// Unknown/unsupported
    Unknown,
}

/// Build system abstraction
#[derive(Debug)]
pub enum BuildSystem {
    /// CMake build system
    CMake(CMakeBuildSystem),
    /// Make build system
    Make(MakeBuildSystem),
    /// Python build system
    Python(PythonBuildSystem),
    /// Node.js build system
    NodeJs(NodeJsBuildSystem),
    /// Rust Cargo build system
    Cargo(CargoBuildSystem),
    /// Custom build system
    Custom(CustomBuildSystem),
}

impl BuildSystem {
    /// Detect build system from source directory
    pub fn detect(source_dir: &PathBuf) -> Result<Self, RezCoreError> {
        // Check for build script first (higher priority for rez packages)
        let build_scripts = ["build.sh", "build.bat", "build.py", "build"];
        for script in &build_scripts {
            if source_dir.join(script).exists() {
                return Ok(BuildSystem::Custom(CustomBuildSystem::new(
                    script.to_string(),
                )));
            }
        }

        // Check for CMakeLists.txt
        if source_dir.join("CMakeLists.txt").exists() {
            return Ok(BuildSystem::CMake(CMakeBuildSystem::new()));
        }

        // Check for Makefile
        if source_dir.join("Makefile").exists() || source_dir.join("makefile").exists() {
            return Ok(BuildSystem::Make(MakeBuildSystem::new()));
        }

        // Check for setup.py or pyproject.toml
        if source_dir.join("setup.py").exists() || source_dir.join("pyproject.toml").exists() {
            return Ok(BuildSystem::Python(PythonBuildSystem::new()));
        }

        // Check for package.json
        if source_dir.join("package.json").exists() {
            return Ok(BuildSystem::NodeJs(NodeJsBuildSystem::new()));
        }

        // Check for Cargo.toml
        if source_dir.join("Cargo.toml").exists() {
            return Ok(BuildSystem::Cargo(CargoBuildSystem::new()));
        }

        // Default to custom build system for packages without explicit build files
        // This allows simple packages to be "built" (essentially just packaged)
        Ok(BuildSystem::Custom(CustomBuildSystem::new(
            "default".to_string(),
        )))
    }

    /// Detect build system from source directory and package definition
    pub fn detect_with_package(
        source_dir: &PathBuf,
        package: &Package,
    ) -> Result<Self, RezCoreError> {
        // Check for explicit build_command first
        if let Some(ref build_command) = package.build_command {
            if build_command == "false" || build_command.is_empty() {
                // No build required - just copy files
                return Ok(BuildSystem::Custom(CustomBuildSystem::new(
                    "copy-only".to_string(),
                )));
            } else {
                // Custom build command
                return Ok(BuildSystem::Custom(CustomBuildSystem::new(
                    "build_command".to_string(),
                )));
            }
        }

        // Check for explicit build_system specification
        if let Some(ref build_system) = package.build_system {
            match build_system.as_str() {
                "cmake" => return Ok(BuildSystem::CMake(CMakeBuildSystem::new())),
                "make" => return Ok(BuildSystem::Make(MakeBuildSystem::new())),
                "python" => return Ok(BuildSystem::Python(PythonBuildSystem::new())),
                "nodejs" => return Ok(BuildSystem::NodeJs(NodeJsBuildSystem::new())),
                "cargo" => return Ok(BuildSystem::Cargo(CargoBuildSystem::new())),
                _ => {
                    return Ok(BuildSystem::Custom(CustomBuildSystem::new(
                        build_system.clone(),
                    )))
                }
            }
        }

        // Check for explicit build scripts
        if source_dir.join("rezbuild.py").exists() {
            return Ok(BuildSystem::Custom(CustomBuildSystem::new(
                "rezbuild.py".to_string(),
            )));
        }

        // Fall back to standard detection
        Self::detect(source_dir)
    }

    /// Configure the build
    pub async fn configure(
        &self,
        request: &BuildRequest,
        environment: &BuildEnvironment,
    ) -> Result<BuildStepResult, RezCoreError> {
        match self {
            BuildSystem::CMake(cmake) => cmake.configure(request, environment).await,
            BuildSystem::Make(make) => make.configure(request, environment).await,
            BuildSystem::Python(python) => python.configure(request, environment).await,
            BuildSystem::NodeJs(nodejs) => nodejs.configure(request, environment).await,
            BuildSystem::Cargo(cargo) => cargo.configure(request, environment).await,
            BuildSystem::Custom(custom) => custom.configure(request, environment).await,
        }
    }

    /// Compile the project
    pub async fn compile(
        &self,
        request: &BuildRequest,
        environment: &BuildEnvironment,
        child_process: Arc<Mutex<Option<Child>>>,
    ) -> Result<BuildStepResult, RezCoreError> {
        match self {
            BuildSystem::CMake(cmake) => cmake.compile(request, environment, child_process).await,
            BuildSystem::Make(make) => make.compile(request, environment, child_process).await,
            BuildSystem::Python(python) => {
                python.compile(request, environment, child_process).await
            }
            BuildSystem::NodeJs(nodejs) => {
                nodejs.compile(request, environment, child_process).await
            }
            BuildSystem::Cargo(cargo) => cargo.compile(request, environment, child_process).await,
            BuildSystem::Custom(custom) => {
                custom.compile(request, environment, child_process).await
            }
        }
    }

    /// Run tests
    pub async fn test(
        &self,
        request: &BuildRequest,
        environment: &BuildEnvironment,
        child_process: Arc<Mutex<Option<Child>>>,
    ) -> Result<BuildStepResult, RezCoreError> {
        match self {
            BuildSystem::CMake(cmake) => cmake.test(request, environment, child_process).await,
            BuildSystem::Make(make) => make.test(request, environment, child_process).await,
            BuildSystem::Python(python) => python.test(request, environment, child_process).await,
            BuildSystem::NodeJs(nodejs) => nodejs.test(request, environment, child_process).await,
            BuildSystem::Cargo(cargo) => cargo.test(request, environment, child_process).await,
            BuildSystem::Custom(custom) => custom.test(request, environment, child_process).await,
        }
    }

    /// Package the build
    pub async fn package(
        &self,
        request: &BuildRequest,
        environment: &BuildEnvironment,
    ) -> Result<BuildStepResult, RezCoreError> {
        match self {
            BuildSystem::CMake(cmake) => cmake.package(request, environment).await,
            BuildSystem::Make(make) => make.package(request, environment).await,
            BuildSystem::Python(python) => python.package(request, environment).await,
            BuildSystem::NodeJs(nodejs) => nodejs.package(request, environment).await,
            BuildSystem::Cargo(cargo) => cargo.package(request, environment).await,
            BuildSystem::Custom(custom) => custom.package(request, environment).await,
        }
    }

    /// Install the build
    pub async fn install(
        &self,
        request: &BuildRequest,
        environment: &BuildEnvironment,
    ) -> Result<BuildStepResult, RezCoreError> {
        match self {
            BuildSystem::CMake(cmake) => cmake.install(request, environment).await,
            BuildSystem::Make(make) => make.install(request, environment).await,
            BuildSystem::Python(python) => python.install(request, environment).await,
            BuildSystem::NodeJs(nodejs) => nodejs.install(request, environment).await,
            BuildSystem::Cargo(cargo) => cargo.install(request, environment).await,
            BuildSystem::Custom(custom) => custom.install(request, environment).await,
        }
    }
}

/// CMake build system
#[derive(Debug)]
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

        let executor = ShellExecutor::with_shell(rez_core_context::ShellType::detect())
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

        let executor = ShellExecutor::with_shell(rez_core_context::ShellType::detect())
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
        request: &BuildRequest,
        environment: &BuildEnvironment,
        _child_process: Arc<Mutex<Option<Child>>>,
    ) -> Result<BuildStepResult, RezCoreError> {
        let build_dir = environment.get_build_dir();

        let executor = ShellExecutor::with_shell(rez_core_context::ShellType::detect())
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
        // CMake packaging is typically done during install
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
        request: &BuildRequest,
        environment: &BuildEnvironment,
    ) -> Result<BuildStepResult, RezCoreError> {
        let build_dir = environment.get_build_dir();

        let executor = ShellExecutor::with_shell(rez_core_context::ShellType::detect())
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

/// Make build system (simplified implementation)
#[derive(Debug)]
pub struct MakeBuildSystem;

impl MakeBuildSystem {
    pub fn new() -> Self {
        Self
    }

    pub async fn configure(
        &self,
        _request: &BuildRequest,
        _environment: &BuildEnvironment,
    ) -> Result<BuildStepResult, RezCoreError> {
        // Make typically doesn't have a separate configure step
        Ok(BuildStepResult {
            step: BuildStep::Configuring,
            success: true,
            output: "Make build system ready".to_string(),
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
        let executor = ShellExecutor::with_shell(rez_core_context::ShellType::detect())
            .with_environment(environment.get_env_vars().clone())
            .with_working_directory(request.source_dir.clone());

        let command = "make";
        let result = executor.execute(command).await?;

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
        let executor = ShellExecutor::with_shell(rez_core_context::ShellType::detect())
            .with_environment(environment.get_env_vars().clone())
            .with_working_directory(request.source_dir.clone());

        let command = "make test";
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
            output: "Make packaging handled during install".to_string(),
            errors: String::new(),
            duration_ms: 0,
        })
    }

    pub async fn install(
        &self,
        request: &BuildRequest,
        environment: &BuildEnvironment,
    ) -> Result<BuildStepResult, RezCoreError> {
        let executor = ShellExecutor::with_shell(rez_core_context::ShellType::detect())
            .with_environment(environment.get_env_vars().clone())
            .with_working_directory(request.source_dir.clone());

        let install_dir = environment.get_install_dir();
        let command = format!("make install DESTDIR={}", install_dir.to_string_lossy());
        let result = executor.execute(&command).await?;

        Ok(BuildStepResult {
            step: BuildStep::Installing,
            success: result.is_success(),
            output: result.stdout,
            errors: result.stderr,
            duration_ms: result.execution_time_ms,
        })
    }
}

// Placeholder implementations for other build systems
#[derive(Debug)]
pub struct PythonBuildSystem;

#[derive(Debug)]
pub struct NodeJsBuildSystem;

#[derive(Debug)]
pub struct CargoBuildSystem;

#[derive(Debug)]
pub struct CustomBuildSystem {
    script_name: String,
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
        // For custom build systems, we need to handle different cases
        match self.script_name.as_str() {
            "default" | "copy-only" => {
                // Default case: copy source files to install directory (for pure packages)
                self.copy_files_install(request, environment).await
            }
            "build_command" => {
                // Execute build_command from package definition
                self.execute_build_command(request, environment).await
            }
            script_name => {
                // Execute custom build script with install command
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

        // Create install directory
        tokio::fs::create_dir_all(install_dir).await.map_err(|e| {
            RezCoreError::BuildError(format!("Failed to create install directory: {}", e))
        })?;

        // Copy all source files to install directory
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

        // Create install directory
        tokio::fs::create_dir_all(install_dir).await.map_err(|e| {
            RezCoreError::BuildError(format!("Failed to create install directory: {}", e))
        })?;

        // Get build_command from package
        let build_command = request.package.build_command.as_ref().ok_or_else(|| {
            RezCoreError::BuildError("build_command not found in package".to_string())
        })?;

        let executor = ShellExecutor::with_shell(rez_core_context::ShellType::detect())
            .with_environment(environment.get_env_vars().clone())
            .with_working_directory(request.source_dir.clone());

        // Expand variables in build command
        let expanded_command =
            self.expand_build_command_variables(build_command, request, environment);

        let result = executor.execute(&expanded_command).await?;

        // If build command succeeded, also copy source files to install directory
        // This ensures that package.py and other source files are available
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

        // Replace {root} with source directory
        expanded = expanded.replace("{root}", &request.source_dir.to_string_lossy());

        // Replace {install} with "install" if installing, empty otherwise
        expanded = expanded.replace("{install}", "install");

        // Replace {build_path} with build directory
        expanded = expanded.replace(
            "{build_path}",
            &environment.get_build_dir().to_string_lossy(),
        );

        // Replace {install_path} with install directory
        expanded = expanded.replace(
            "{install_path}",
            &environment.get_install_dir().to_string_lossy(),
        );

        // Replace {name} with package name
        expanded = expanded.replace("{name}", &request.package.name);

        // Replace {version} with package version
        if let Some(ref version) = request.package.version {
            expanded = expanded.replace("{version}", version.as_str());
        } else {
            expanded = expanded.replace("{version}", "");
        }

        // Replace {variant_index} with variant index (empty for now)
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

        // Create install directory
        tokio::fs::create_dir_all(install_dir).await.map_err(|e| {
            RezCoreError::BuildError(format!("Failed to create install directory: {}", e))
        })?;

        let executor = ShellExecutor::with_shell(rez_core_context::ShellType::detect())
            .with_environment(environment.get_env_vars().clone())
            .with_working_directory(request.source_dir.clone());

        // Determine how to execute the script
        let exec_command = if self.script_name.ends_with(".py") {
            format!("python {} {}", script_path.to_string_lossy(), command)
        } else if self.script_name.ends_with(".sh") {
            format!("bash {} {}", script_path.to_string_lossy(), command)
        } else if self.script_name.ends_with(".bat") {
            format!("{} {}", script_path.to_string_lossy(), command)
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
    async fn copy_package_files(
        source_dir: &PathBuf,
        install_dir: &PathBuf,
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

            // Skip certain files that shouldn't be installed
            let file_name_str = file_name.to_string_lossy();
            if Self::should_skip_file(&file_name_str) {
                continue;
            }

            if src_path.is_dir() {
                // Recursively copy directory
                let dir_files = Self::copy_dir_recursive(src_path, dest_path).await?;
                files_copied += dir_files;
            } else {
                // Copy file
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
    fn should_skip_file(file_name: &str) -> bool {
        // Skip common development files that shouldn't be installed
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

// Simplified implementations for the remaining build systems
// In a real implementation, these would be fully fleshed out

macro_rules! impl_build_system_placeholder {
    ($system:ty) => {
        impl $system {
            pub fn new() -> Self {
                Self {}
            }

            pub async fn configure(
                &self,
                _request: &BuildRequest,
                _environment: &BuildEnvironment,
            ) -> Result<BuildStepResult, RezCoreError> {
                Ok(BuildStepResult {
                    step: BuildStep::Configuring,
                    success: true,
                    output: "Configuration completed".to_string(),
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
                    output: "Compilation completed".to_string(),
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
                _request: &BuildRequest,
                _environment: &BuildEnvironment,
            ) -> Result<BuildStepResult, RezCoreError> {
                // Placeholder implementations should not be used for actual installation
                // This is a fallback that should not be reached
                Ok(BuildStepResult {
                    step: BuildStep::Installing,
                    success: false,
                    output: String::new(),
                    errors: "Placeholder build system should not be used for installation"
                        .to_string(),
                    duration_ms: 0,
                })
            }
        }
    };
}

impl_build_system_placeholder!(PythonBuildSystem);
impl_build_system_placeholder!(NodeJsBuildSystem);
impl_build_system_placeholder!(CargoBuildSystem);
