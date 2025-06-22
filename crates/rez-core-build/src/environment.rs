//! Build environment management

use rez_core_common::RezCoreError;
use rez_core_context::ResolvedContext;
use rez_core_package::Package;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Build environment for package builds
#[derive(Debug, Clone)]
pub struct BuildEnvironment {
    /// Package being built
    package: Package,
    /// Build directory
    build_dir: PathBuf,
    /// Install directory
    install_dir: PathBuf,
    /// Temporary directory
    temp_dir: PathBuf,
    /// Environment variables
    env_vars: HashMap<String, String>,
    /// Build context (resolved dependencies)
    context: Option<ResolvedContext>,
}

impl BuildEnvironment {
    /// Create a new build environment
    pub fn new(
        package: &Package,
        base_build_dir: &PathBuf,
        context: Option<&ResolvedContext>,
    ) -> Result<Self, RezCoreError> {
        Self::with_install_path(package, base_build_dir, context, None)
    }

    /// Create a new build environment with custom install path
    pub fn with_install_path(
        package: &Package,
        base_build_dir: &PathBuf,
        context: Option<&ResolvedContext>,
        install_path: Option<&PathBuf>,
    ) -> Result<Self, RezCoreError> {
        // Normalize the base build directory to handle various path formats
        let normalized_base = Self::normalize_build_path(base_build_dir)?;
        let package_build_dir = normalized_base.join(&package.name);

        // Use custom install path or default to build directory
        let install_dir = if let Some(custom_path) = install_path {
            custom_path.join(&package.name).join(
                package
                    .version
                    .as_ref()
                    .map(|v| v.as_str())
                    .unwrap_or("unknown"),
            )
        } else {
            package_build_dir.join("install")
        };

        let temp_dir = package_build_dir.join("tmp");

        // Set up environment variables
        let mut env_vars = HashMap::new();

        // Add package-specific variables
        env_vars.insert("REZ_BUILD_PACKAGE_NAME".to_string(), package.name.clone());

        if let Some(ref version) = package.version {
            env_vars.insert(
                "REZ_BUILD_PACKAGE_VERSION".to_string(),
                version.as_str().to_string(),
            );
        }

        env_vars.insert(
            "REZ_BUILD_INSTALL_PATH".to_string(),
            install_dir.to_string_lossy().to_string(),
        );
        env_vars.insert(
            "REZ_BUILD_PATH".to_string(),
            package_build_dir.to_string_lossy().to_string(),
        );

        // Add context environment if available
        if let Some(context) = context {
            for (key, value) in &context.environment_vars {
                env_vars.insert(key.clone(), value.clone());
            }
        }

        Ok(Self {
            package: package.clone(),
            build_dir: package_build_dir,
            install_dir,
            temp_dir,
            env_vars,
            context: context.cloned(),
        })
    }

    /// Get build directory
    pub fn get_build_dir(&self) -> &PathBuf {
        &self.build_dir
    }

    /// Get install directory
    pub fn get_install_dir(&self) -> &PathBuf {
        &self.install_dir
    }

    /// Get temporary directory
    pub fn get_temp_dir(&self) -> &PathBuf {
        &self.temp_dir
    }

    /// Get environment variables
    pub fn get_env_vars(&self) -> &HashMap<String, String> {
        &self.env_vars
    }

    /// Get package
    pub fn get_package(&self) -> &Package {
        &self.package
    }

    /// Get context
    pub fn get_context(&self) -> Option<&ResolvedContext> {
        self.context.as_ref()
    }

    /// Add environment variable
    pub fn add_env_var(&mut self, name: String, value: String) {
        self.env_vars.insert(name, value);
    }

    /// Remove environment variable
    pub fn remove_env_var(&mut self, name: &str) {
        self.env_vars.remove(name);
    }

    /// Set up build environment directories
    pub async fn setup(&self) -> Result<(), RezCoreError> {
        // Create directories
        tokio::fs::create_dir_all(&self.build_dir)
            .await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to create build dir: {}", e)))?;

        tokio::fs::create_dir_all(&self.install_dir)
            .await
            .map_err(|e| {
                RezCoreError::BuildError(format!("Failed to create install dir: {}", e))
            })?;

        tokio::fs::create_dir_all(&self.temp_dir)
            .await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to create temp dir: {}", e)))?;

        Ok(())
    }

    /// Clean build environment
    pub async fn clean(&self) -> Result<(), RezCoreError> {
        if self.build_dir.exists() {
            tokio::fs::remove_dir_all(&self.build_dir)
                .await
                .map_err(|e| {
                    RezCoreError::BuildError(format!("Failed to clean build dir: {}", e))
                })?;
        }
        Ok(())
    }

    /// Get environment as shell script
    pub fn to_shell_script(&self, shell_type: &rez_core_context::ShellType) -> String {
        let mut script = String::new();

        match shell_type {
            rez_core_context::ShellType::Bash | rez_core_context::ShellType::Zsh => {
                script.push_str("#!/bin/bash\n");
                script.push_str("# Build environment setup\n\n");
                for (name, value) in &self.env_vars {
                    script.push_str(&format!("export {}=\"{}\"\n", name, value));
                }
            }
            rez_core_context::ShellType::Fish => {
                script.push_str("#!/usr/bin/env fish\n");
                script.push_str("# Build environment setup\n\n");
                for (name, value) in &self.env_vars {
                    script.push_str(&format!("set -x {} \"{}\"\n", name, value));
                }
            }
            rez_core_context::ShellType::Cmd => {
                script.push_str("@echo off\n");
                script.push_str("REM Build environment setup\n\n");
                for (name, value) in &self.env_vars {
                    script.push_str(&format!("set {}={}\n", name, value));
                }
            }
            rez_core_context::ShellType::PowerShell => {
                script.push_str("# Build environment setup\n\n");
                for (name, value) in &self.env_vars {
                    script.push_str(&format!("$env:{} = \"{}\"\n", name, value));
                }
            }
        }

        script
    }

    /// Normalize build path to handle various path formats
    fn normalize_build_path(path: &PathBuf) -> Result<PathBuf, RezCoreError> {
        let path_str = path.to_string_lossy();

        // Handle different path formats
        let normalized = if path_str.starts_with("~/") {
            // Handle home directory expansion
            Self::expand_home_path(&path_str)?
        } else if path_str.starts_with("\\\\") {
            // Handle UNC paths - validate but keep as-is
            Self::validate_unc_path(&path_str)?;
            path.clone()
        } else if path_str.len() >= 2 && path_str.chars().nth(1) == Some(':') {
            // Handle Windows drive paths - validate but keep as-is
            Self::validate_drive_path(&path_str)?;
            path.clone()
        } else if path.is_absolute() {
            // Already absolute path
            path.clone()
        } else {
            // Convert relative path to absolute
            std::env::current_dir()
                .map_err(|e| {
                    RezCoreError::BuildError(format!("Cannot get current directory: {}", e))
                })?
                .join(path)
        };

        Ok(normalized)
    }

    /// Expand home directory paths
    fn expand_home_path(path: &str) -> Result<PathBuf, RezCoreError> {
        if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
            let home_path = PathBuf::from(home);
            Ok(home_path.join(&path[2..]))
        } else {
            Err(RezCoreError::BuildError(
                "Cannot determine home directory".to_string(),
            ))
        }
    }

    /// Validate UNC paths
    fn validate_unc_path(path: &str) -> Result<(), RezCoreError> {
        if !path.starts_with("\\\\") {
            return Err(RezCoreError::BuildError(
                "Invalid UNC path format".to_string(),
            ));
        }

        // Basic UNC path validation: \\server\share\path
        let parts: Vec<&str> = path[2..].split('\\').collect();
        if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err(RezCoreError::BuildError(
                "UNC path must be in format \\\\server\\share\\path".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate Windows drive paths
    fn validate_drive_path(path: &str) -> Result<(), RezCoreError> {
        if path.len() < 2 {
            return Err(RezCoreError::BuildError(
                "Invalid drive path format".to_string(),
            ));
        }

        let drive_char = path.chars().nth(0).unwrap();
        if !drive_char.is_ascii_alphabetic() || path.chars().nth(1) != Some(':') {
            return Err(RezCoreError::BuildError(
                "Drive path must start with a letter followed by colon (e.g., C:)".to_string(),
            ));
        }

        Ok(())
    }
}
