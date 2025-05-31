//! Build environment management

use rez_core_common::RezCoreError;
use rez_core_package::Package;
use rez_core_context::ResolvedContext;
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
        let package_build_dir = base_build_dir.join(&package.name);
        let install_dir = package_build_dir.join("install");
        let temp_dir = package_build_dir.join("tmp");

        // Set up environment variables
        let mut env_vars = HashMap::new();
        
        // Add package-specific variables
        env_vars.insert("REZ_BUILD_PACKAGE_NAME".to_string(), package.name.clone());
        
        if let Some(ref version) = package.version {
            env_vars.insert("REZ_BUILD_PACKAGE_VERSION".to_string(), version.as_str().to_string());
        }

        env_vars.insert("REZ_BUILD_INSTALL_PATH".to_string(), install_dir.to_string_lossy().to_string());
        env_vars.insert("REZ_BUILD_PATH".to_string(), package_build_dir.to_string_lossy().to_string());

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
        tokio::fs::create_dir_all(&self.build_dir).await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to create build dir: {}", e)))?;

        tokio::fs::create_dir_all(&self.install_dir).await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to create install dir: {}", e)))?;

        tokio::fs::create_dir_all(&self.temp_dir).await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to create temp dir: {}", e)))?;

        Ok(())
    }

    /// Clean build environment
    pub async fn clean(&self) -> Result<(), RezCoreError> {
        if self.build_dir.exists() {
            tokio::fs::remove_dir_all(&self.build_dir).await
                .map_err(|e| RezCoreError::BuildError(format!("Failed to clean build dir: {}", e)))?;
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
}
