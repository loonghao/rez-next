//! Built-in PyPI package build plugin.
//!
//! This covers the common rez-pip style workflow for a single package: install
//! a pip-compatible artifact or spec into the Rez package install root.

use crate::{BuildEnvironment, BuildRequest, BuildStep, BuildStepResult};
use rez_next_common::RezCoreError;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::process::Child;
use tokio::sync::Mutex;

const PACKAGE_ENV: &str = "REZ_PYPI_PACKAGE";
const REQUIREMENT_ENV: &str = "REZ_PYPI_REQUIREMENT";
const EXTRA_ARGS_ENV: &str = "REZ_PYPI_EXTRA_ARGS";
const WITH_DEPS_ENV: &str = "REZ_PYPI_WITH_DEPS";

/// Native build system for pip-compatible Python packages.
#[derive(Debug, Clone, Default)]
pub struct PypiBuildSystem;

impl PypiBuildSystem {
    pub fn new() -> Self {
        Self
    }

    pub async fn configure(
        &self,
        _request: &BuildRequest,
        environment: &BuildEnvironment,
    ) -> Result<BuildStepResult, RezCoreError> {
        run_python(
            environment,
            None,
            BuildStep::Configuring,
            &["-m".to_string(), "pip".to_string(), "--version".to_string()],
        )
        .await
    }

    pub async fn compile(
        &self,
        _request: &BuildRequest,
        _environment: &BuildEnvironment,
        _child_process: Arc<Mutex<Option<Child>>>,
    ) -> Result<BuildStepResult, RezCoreError> {
        Ok(step_ok(
            BuildStep::Compiling,
            "pypi uses pip install artifacts",
        ))
    }

    pub async fn test(
        &self,
        _request: &BuildRequest,
        _environment: &BuildEnvironment,
        _child_process: Arc<Mutex<Option<Child>>>,
    ) -> Result<BuildStepResult, RezCoreError> {
        Ok(step_ok(BuildStep::Testing, "pypi plugin tests skipped"))
    }

    pub async fn package(
        &self,
        _request: &BuildRequest,
        _environment: &BuildEnvironment,
    ) -> Result<BuildStepResult, RezCoreError> {
        Ok(step_ok(BuildStep::Packaging, "pypi package step completed"))
    }

    pub async fn install(
        &self,
        request: &BuildRequest,
        environment: &BuildEnvironment,
    ) -> Result<BuildStepResult, RezCoreError> {
        let install_dir = environment.get_install_dir();
        let python_dir = install_dir.join("python");
        tokio::fs::create_dir_all(&python_dir)
            .await
            .map_err(|err| {
                RezCoreError::BuildError(format!("Failed to create pypi install dir: {err}"))
            })?;

        let env = environment.get_env_vars();
        let package_spec = env
            .get(PACKAGE_ENV)
            .filter(|value| !value.is_empty())
            .cloned()
            .or_else(|| default_package_spec(request));
        let requirement = env.get(REQUIREMENT_ENV).filter(|value| !value.is_empty());

        if package_spec.is_none() && requirement.is_none() {
            return Ok(BuildStepResult {
                step: BuildStep::Installing,
                success: false,
                output: String::new(),
                errors: format!("pypi requires {PACKAGE_ENV} or {REQUIREMENT_ENV}"),
                duration_ms: 0,
            });
        }

        let mut args = vec![
            "-m".to_string(),
            "pip".to_string(),
            "install".to_string(),
            "--disable-pip-version-check".to_string(),
            "--target".to_string(),
            python_dir.to_string_lossy().to_string(),
        ];

        if !env_var_truthy(env.get(WITH_DEPS_ENV)) {
            args.push("--no-deps".to_string());
        }

        if let Some(extra_args) = env.get(EXTRA_ARGS_ENV).filter(|value| !value.is_empty()) {
            args.extend(extra_args.split_whitespace().map(ToString::to_string));
        }

        if let Some(requirement) = requirement {
            args.push("-r".to_string());
            args.push(requirement.clone());
        }

        if let Some(package_spec) = package_spec {
            args.push(package_spec);
        }

        run_python(
            environment,
            Some(&request.source_dir),
            BuildStep::Installing,
            &args,
        )
        .await
    }
}

fn default_package_spec(request: &BuildRequest) -> Option<String> {
    request
        .package
        .version
        .as_ref()
        .map(|version| format!("{}=={}", request.package.name, version.as_str()))
}

async fn run_python(
    environment: &BuildEnvironment,
    cwd: Option<&Path>,
    step: BuildStep,
    args: &[String],
) -> Result<BuildStepResult, RezCoreError> {
    let mut env = environment.get_env_vars().clone();
    add_windows_runtime_env(&mut env);

    let started_at = Instant::now();
    let mut command = tokio::process::Command::new("python");
    command.args(args).env_clear().envs(&env);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }

    let output = command
        .output()
        .await
        .map_err(|err| RezCoreError::ExecutionError(format!("Failed to run python: {err}")))?;

    Ok(BuildStepResult {
        step,
        success: output.status.success(),
        output: String::from_utf8_lossy(&output.stdout).to_string(),
        errors: String::from_utf8_lossy(&output.stderr).to_string(),
        duration_ms: started_at.elapsed().as_millis() as u64,
    })
}

fn add_windows_runtime_env(env: &mut std::collections::HashMap<String, String>) {
    if !cfg!(windows) {
        return;
    }

    for key in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
        if !env.contains_key(key)
            && let Ok(value) = std::env::var(key)
        {
            env.insert(key.to_string(), value);
        }
    }
}

fn env_var_truthy(value: Option<&String>) -> bool {
    matches!(
        value.map(|value| value.to_ascii_lowercase()),
        Some(value) if matches!(value.as_str(), "1" | "true" | "yes" | "on")
    )
}

fn step_ok(step: BuildStep, output: &str) -> BuildStepResult {
    BuildStepResult {
        step,
        success: true,
        output: output.to_string(),
        errors: String::new(),
        duration_ms: 0,
    }
}
