//! Build system implementations
//!
//! This module is split into sub-modules by build system type:
//! - `cmake`       — CMake build system
//! - `make`        — Make build system
//! - `python`      — Python setuptools / rezbuild.py
//! - `nodejs`      — Node.js npm
//! - `cargo_build` — Rust Cargo
//! - `custom`      — Custom build scripts / copy-only
//! - `cmd_builder` — Shared command-runner helpers (no shell-specific strings)

mod cargo_build;
mod cmake;
pub(crate) mod cmd_builder;
mod custom;
mod make;
mod nodejs;
mod python;

pub use cargo_build::CargoBuildSystem;
pub use cmake::CMakeBuildSystem;
pub use custom::CustomBuildSystem;
pub use make::MakeBuildSystem;
pub use nodejs::NodeJsBuildSystem;
pub use python::PythonBuildSystem;

use crate::{BuildEnvironment, BuildRequest, BuildStepResult};
use rez_next_common::RezCoreError;
use rez_next_package::Package;
use std::path::Path;
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
    pub fn detect(source_dir: &Path) -> Result<Self, RezCoreError> {
        // Check for build script first (higher priority for rez packages)
        let build_scripts = ["build.sh", "build.bat", "build.py", "build"];
        for script in &build_scripts {
            if source_dir.join(script).exists() {
                return Ok(BuildSystem::Custom(CustomBuildSystem::new(
                    script.to_string(),
                )));
            }
        }

        if source_dir.join("CMakeLists.txt").exists() {
            return Ok(BuildSystem::CMake(CMakeBuildSystem::new()));
        }

        if source_dir.join("Makefile").exists() || source_dir.join("makefile").exists() {
            return Ok(BuildSystem::Make(MakeBuildSystem::new()));
        }

        if source_dir.join("setup.py").exists() || source_dir.join("pyproject.toml").exists() {
            return Ok(BuildSystem::Python(PythonBuildSystem::new()));
        }

        if source_dir.join("package.json").exists() {
            return Ok(BuildSystem::NodeJs(NodeJsBuildSystem::new()));
        }

        if source_dir.join("Cargo.toml").exists() {
            return Ok(BuildSystem::Cargo(CargoBuildSystem::new()));
        }

        // Default to custom build system for packages without explicit build files
        Ok(BuildSystem::Custom(CustomBuildSystem::new(
            "default".to_string(),
        )))
    }

    /// Detect build system from source directory and package definition
    pub fn detect_with_package(source_dir: &Path, package: &Package) -> Result<Self, RezCoreError> {
        // Check for explicit build_command first
        if let Some(ref build_command) = package.build_command {
            if build_command == "false" || build_command.is_empty() {
                return Ok(BuildSystem::Custom(CustomBuildSystem::new(
                    "copy-only".to_string(),
                )));
            } else {
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

        if source_dir.join("rezbuild.py").exists() {
            return Ok(BuildSystem::Custom(CustomBuildSystem::new(
                "rezbuild.py".to_string(),
            )));
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_build_system_type_eq() {
        assert_eq!(BuildSystemType::CMake, BuildSystemType::CMake);
        assert_ne!(BuildSystemType::CMake, BuildSystemType::Make);
        assert_ne!(BuildSystemType::Python, BuildSystemType::NodeJs);
    }

    #[test]
    fn test_build_system_type_clone() {
        let t = BuildSystemType::Cargo;
        let t2 = t.clone();
        assert_eq!(t, t2);
    }

    #[test]
    fn test_detect_nonexistent_dir_returns_custom_default() {
        let dir = PathBuf::from("/nonexistent/path/xyz");
        // Non-existent dir: no files match → returns Custom("default")
        let result = BuildSystem::detect(&dir);
        assert!(result.is_ok());
        match result.unwrap() {
            BuildSystem::Custom(c) => assert_eq!(c.script_name, "default"),
            other => panic!("Expected Custom, got {:?}", other),
        }
    }

    #[test]
    fn test_detect_with_package_copy_only() {
        let dir = PathBuf::from("/nonexistent");
        let mut pkg = rez_next_package::Package::new("test-pkg".to_string());
        pkg.build_command = Some("false".to_string());
        let result = BuildSystem::detect_with_package(&dir, &pkg);
        assert!(result.is_ok());
        match result.unwrap() {
            BuildSystem::Custom(c) => assert_eq!(c.script_name, "copy-only"),
            other => panic!("Expected Custom(copy-only), got {:?}", other),
        }
    }

    #[test]
    fn test_detect_with_package_explicit_cmake() {
        let dir = PathBuf::from("/nonexistent");
        let mut pkg = rez_next_package::Package::new("test-pkg".to_string());
        pkg.build_system = Some("cmake".to_string());
        let result = BuildSystem::detect_with_package(&dir, &pkg);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), BuildSystem::CMake(_)));
    }

    #[test]
    fn test_detect_with_package_explicit_cargo() {
        let dir = PathBuf::from("/nonexistent");
        let mut pkg = rez_next_package::Package::new("test-pkg".to_string());
        pkg.build_system = Some("cargo".to_string());
        let result = BuildSystem::detect_with_package(&dir, &pkg);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), BuildSystem::Cargo(_)));
    }

    // ----- detect() integration tests using real temp directories -----

    #[test]
    fn test_detect_cmake_from_cmakelists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("CMakeLists.txt"),
            "cmake_minimum_required(VERSION 3.10)",
        )
        .unwrap();
        let result = BuildSystem::detect(dir.path()).unwrap();
        assert!(matches!(result, BuildSystem::CMake(_)));
    }

    #[test]
    fn test_detect_make_from_makefile() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Makefile"), "all:\n\techo done").unwrap();
        let result = BuildSystem::detect(dir.path()).unwrap();
        assert!(matches!(result, BuildSystem::Make(_)));
    }

    #[test]
    fn test_detect_python_from_setup_py() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("setup.py"),
            "from setuptools import setup; setup()",
        )
        .unwrap();
        let result = BuildSystem::detect(dir.path()).unwrap();
        assert!(matches!(result, BuildSystem::Python(_)));
    }

    #[test]
    fn test_detect_python_from_pyproject_toml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[build-system]\nrequires = []",
        )
        .unwrap();
        let result = BuildSystem::detect(dir.path()).unwrap();
        assert!(matches!(result, BuildSystem::Python(_)));
    }

    #[test]
    fn test_detect_nodejs_from_package_json() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name":"test","version":"1.0.0"}"#,
        )
        .unwrap();
        let result = BuildSystem::detect(dir.path()).unwrap();
        assert!(matches!(result, BuildSystem::NodeJs(_)));
    }

    #[test]
    fn test_detect_cargo_from_cargo_toml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"",
        )
        .unwrap();
        let result = BuildSystem::detect(dir.path()).unwrap();
        assert!(matches!(result, BuildSystem::Cargo(_)));
    }

    /// build.sh takes priority over CMakeLists.txt (custom build script wins).
    #[test]
    fn test_detect_custom_build_script_wins_over_cmake() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CMakeLists.txt"), "").unwrap();
        std::fs::write(dir.path().join("build.sh"), "#!/bin/sh\necho build").unwrap();
        let result = BuildSystem::detect(dir.path()).unwrap();
        match result {
            BuildSystem::Custom(c) => assert_eq!(c.script_name, "build.sh"),
            other => panic!("Expected Custom(build.sh), got {:?}", other),
        }
    }

    /// detect_with_package: rezbuild.py takes priority over generic auto-detect.
    #[test]
    fn test_detect_with_package_rezbuild_py_wins() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("rezbuild.py"), "def build(dst): pass").unwrap();
        let pkg = rez_next_package::Package::new("rez-pkg".to_string());
        let result = BuildSystem::detect_with_package(dir.path(), &pkg).unwrap();
        match result {
            BuildSystem::Custom(c) => assert_eq!(c.script_name, "rezbuild.py"),
            other => panic!("Expected Custom(rezbuild.py), got {:?}", other),
        }
    }

    /// detect_with_package: explicit build_system = "nodejs" overrides file-based detection.
    #[test]
    fn test_detect_with_package_explicit_nodejs() {
        let dir = PathBuf::from("/nonexistent");
        let mut pkg = rez_next_package::Package::new("test-pkg".to_string());
        pkg.build_system = Some("nodejs".to_string());
        let result = BuildSystem::detect_with_package(&dir, &pkg).unwrap();
        assert!(matches!(result, BuildSystem::NodeJs(_)));
    }

    /// detect_with_package: explicit build_system = "python" overrides file-based detection.
    #[test]
    fn test_detect_with_package_explicit_python() {
        let dir = PathBuf::from("/nonexistent");
        let mut pkg = rez_next_package::Package::new("test-pkg".to_string());
        pkg.build_system = Some("python".to_string());
        let result = BuildSystem::detect_with_package(&dir, &pkg).unwrap();
        assert!(matches!(result, BuildSystem::Python(_)));
    }

    /// detect_with_package: explicit build_system = "make" overrides file-based detection.
    #[test]
    fn test_detect_with_package_explicit_make() {
        let dir = PathBuf::from("/nonexistent");
        let mut pkg = rez_next_package::Package::new("test-pkg".to_string());
        pkg.build_system = Some("make".to_string());
        let result = BuildSystem::detect_with_package(&dir, &pkg).unwrap();
        assert!(matches!(result, BuildSystem::Make(_)));
    }

    /// detect_with_package: non-empty build_command maps to Custom("build_command").
    #[test]
    fn test_detect_with_package_build_command_nonempty() {
        let dir = PathBuf::from("/nonexistent");
        let mut pkg = rez_next_package::Package::new("test-pkg".to_string());
        pkg.build_command = Some("make install".to_string());
        let result = BuildSystem::detect_with_package(&dir, &pkg).unwrap();
        match result {
            BuildSystem::Custom(c) => assert_eq!(c.script_name, "build_command"),
            other => panic!("Expected Custom(build_command), got {:?}", other),
        }
    }
}
