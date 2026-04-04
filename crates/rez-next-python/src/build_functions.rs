//! Build system functions exposed to Python.
//!
//! Covers: build_package, get_build_system.

use pyo3::prelude::*;

use crate::package_functions::expand_home;

/// Build a package from source.
/// Equivalent to running `rez build` or `rez.build_.build_package()`
#[pyfunction]
#[pyo3(signature = (source_dir=None, install=false, clean=false, install_path=None))]
pub fn build_package(
    source_dir: Option<&str>,
    install: bool,
    clean: bool,
    install_path: Option<&str>,
) -> PyResult<String> {
    use rez_next_build::{BuildManager, BuildOptions, BuildRequest};
    use rez_next_common::config::RezCoreConfig;
    use rez_next_package::serialization::PackageSerializer;
    use std::path::PathBuf;

    let cwd = std::env::current_dir()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    let source = PathBuf::from(source_dir.unwrap_or("."));
    let source = if source.is_relative() {
        cwd.join(source)
    } else {
        source
    };

    // Load package definition
    let pkg_py = source.join("package.py");
    let pkg_yaml = source.join("package.yaml");
    let package = if pkg_py.exists() {
        PackageSerializer::load_from_file(&pkg_py)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?
    } else if pkg_yaml.exists() {
        PackageSerializer::load_from_file(&pkg_yaml)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?
    } else {
        return Err(pyo3::exceptions::PyFileNotFoundError::new_err(
            "No package.py or package.yaml found",
        ));
    };

    let config = RezCoreConfig::load();
    let dest = install_path
        .map(PathBuf::from)
        .or_else(|| Some(PathBuf::from(expand_home(&config.local_packages_path))));

    let options = BuildOptions {
        force_rebuild: clean,
        skip_tests: false,
        release_mode: false,
        build_args: Vec::new(),
        env_vars: std::collections::HashMap::new(),
    };

    let request = BuildRequest {
        package: package.clone(),
        context: None,
        source_dir: source,
        variant: None,
        options,
        install_path: if install { dest } else { None },
    };

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    let mut build_manager = BuildManager::new();
    let build_id = rt
        .block_on(build_manager.start_build(request))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    let result = rt
        .block_on(build_manager.wait_for_build(&build_id))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    if result.success {
        Ok(format!("Build succeeded: {}", build_id))
    } else {
        Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
            "Build failed: {}",
            result.errors
        )))
    }
}

/// Get the build system type for a given source directory.
/// Equivalent to `rez.build_.get_build_system(working_dir)`
#[pyfunction]
#[pyo3(signature = (source_dir=None))]
pub fn get_build_system(source_dir: Option<&str>) -> PyResult<String> {
    use std::path::PathBuf;

    let cwd = std::env::current_dir()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    let source = PathBuf::from(source_dir.unwrap_or("."));
    let source = if source.is_relative() {
        cwd.join(&source)
    } else {
        source
    };

    if source.join("rezbuild.py").exists() {
        return Ok("python_rezbuild".to_string());
    }
    if source.join("CMakeLists.txt").exists() {
        return Ok("cmake".to_string());
    }
    if source.join("Makefile").exists() || source.join("makefile").exists() {
        return Ok("make".to_string());
    }
    if source.join("setup.py").exists() || source.join("pyproject.toml").exists() {
        return Ok("python".to_string());
    }
    if source.join("package.json").exists() {
        return Ok("nodejs".to_string());
    }
    if source.join("Cargo.toml").exists() {
        return Ok("cargo".to_string());
    }
    if source.join("build.sh").exists() || source.join("build.bat").exists() {
        return Ok("custom_script".to_string());
    }
    Ok("unknown".to_string())
}
