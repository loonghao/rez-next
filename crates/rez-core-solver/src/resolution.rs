//! Resolution result and related types

use rez_core_package::Package;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of a dependency resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionResult {
    /// Resolved packages in dependency order
    pub packages: Vec<Package>,
    /// Whether conflicts were resolved during resolution
    pub conflicts_resolved: bool,
    /// Resolution time in milliseconds
    pub resolution_time_ms: u64,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl ResolutionResult {
    /// Create a new resolution result
    pub fn new(packages: Vec<Package>) -> Self {
        Self {
            packages,
            conflicts_resolved: false,
            resolution_time_ms: 0,
            metadata: HashMap::new(),
        }
    }

    /// Create a successful resolution with conflicts resolved
    pub fn with_conflicts_resolved(packages: Vec<Package>, resolution_time_ms: u64) -> Self {
        Self {
            packages,
            conflicts_resolved: true,
            resolution_time_ms,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the resolution result
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Get the number of resolved packages
    pub fn package_count(&self) -> usize {
        self.packages.len()
    }

    /// Get package by name
    pub fn get_package(&self, name: &str) -> Option<&Package> {
        self.packages.iter().find(|p| p.name == name)
    }

    /// Get all package names
    pub fn get_package_names(&self) -> Vec<String> {
        self.packages.iter().map(|p| p.name.clone()).collect()
    }

    /// Check if a package is included in the resolution
    pub fn contains_package(&self, name: &str) -> bool {
        self.packages.iter().any(|p| p.name == name)
    }

    /// Get packages that match a pattern
    pub fn find_packages(&self, pattern: &str) -> Vec<&Package> {
        self.packages.iter()
            .filter(|p| self.matches_pattern(&p.name, pattern))
            .collect()
    }

    /// Simple pattern matching (supports * wildcard)
    fn matches_pattern(&self, text: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if pattern.contains('*') {
            // Convert to regex pattern
            let regex_pattern = pattern.replace("*", ".*");
            if let Ok(regex) = regex::Regex::new(&format!("^{}$", regex_pattern)) {
                return regex.is_match(text);
            }
        }

        text == pattern
    }

    /// Get resolution summary
    pub fn get_summary(&self) -> ResolutionSummary {
        let mut package_versions = HashMap::new();
        let mut total_size = 0u64;

        for package in &self.packages {
            if let Some(ref version) = package.version {
                package_versions.insert(package.name.clone(), version.as_str().to_string());
            } else {
                package_versions.insert(package.name.clone(), "latest".to_string());
            }

            // Estimate package size (this would be more accurate with actual file sizes)
            total_size += 1024 * 1024; // 1MB per package as estimate
        }

        ResolutionSummary {
            package_count: self.packages.len(),
            conflicts_resolved: self.conflicts_resolved,
            resolution_time_ms: self.resolution_time_ms,
            estimated_size_bytes: total_size,
            package_versions,
        }
    }

    /// Validate the resolution result
    pub fn validate(&self) -> Result<(), String> {
        // Check for duplicate packages
        let mut seen_packages = std::collections::HashSet::new();
        for package in &self.packages {
            let key = match &package.version {
                Some(version) => format!("{}-{}", package.name, version.as_str()),
                None => package.name.clone(),
            };

            if seen_packages.contains(&key) {
                return Err(format!("Duplicate package in resolution: {}", key));
            }
            seen_packages.insert(key);
        }

        // Validate package definitions
        for package in &self.packages {
            if let Err(e) = package.validate() {
                return Err(format!("Invalid package {}: {}", package.name, e));
            }
        }

        Ok(())
    }

    /// Convert to a format suitable for environment generation
    pub fn to_environment_spec(&self) -> EnvironmentSpec {
        let mut packages = Vec::new();
        let mut environment_vars = HashMap::new();

        for package in &self.packages {
            let package_spec = PackageSpec {
                name: package.name.clone(),
                version: package.version.as_ref().map(|v| v.as_str().to_string()),
                requirements: package.requires.clone(),
                tools: package.tools.clone(),
            };
            packages.push(package_spec);

            // Add package-specific environment variables
            if let Some(ref commands) = package.commands {
                environment_vars.insert(
                    format!("{}_COMMANDS", package.name.to_uppercase()),
                    commands.clone()
                );
            }
        }

        EnvironmentSpec {
            packages,
            environment_vars,
            metadata: self.metadata.clone(),
        }
    }
}

/// Summary of a resolution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionSummary {
    /// Number of packages in the resolution
    pub package_count: usize,
    /// Whether conflicts were resolved
    pub conflicts_resolved: bool,
    /// Resolution time in milliseconds
    pub resolution_time_ms: u64,
    /// Estimated total size in bytes
    pub estimated_size_bytes: u64,
    /// Package versions included
    pub package_versions: HashMap<String, String>,
}

/// Environment specification generated from resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentSpec {
    /// Packages in the environment
    pub packages: Vec<PackageSpec>,
    /// Environment variables to set
    pub environment_vars: HashMap<String, String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Package specification for environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSpec {
    /// Package name
    pub name: String,
    /// Package version
    pub version: Option<String>,
    /// Package requirements
    pub requirements: Vec<String>,
    /// Package tools
    pub tools: Vec<String>,
}

impl EnvironmentSpec {
    /// Get all package names
    pub fn get_package_names(&self) -> Vec<String> {
        self.packages.iter().map(|p| p.name.clone()).collect()
    }

    /// Get environment variable by name
    pub fn get_env_var(&self, name: &str) -> Option<&String> {
        self.environment_vars.get(name)
    }

    /// Add an environment variable
    pub fn add_env_var(&mut self, name: String, value: String) {
        self.environment_vars.insert(name, value);
    }

    /// Get all tools from all packages
    pub fn get_all_tools(&self) -> Vec<String> {
        let mut all_tools = Vec::new();
        for package in &self.packages {
            all_tools.extend(package.tools.iter().cloned());
        }
        all_tools.sort();
        all_tools.dedup();
        all_tools
    }

    /// Generate shell script for environment setup
    pub fn generate_shell_script(&self, shell: ShellType) -> String {
        let mut script = String::new();

        match shell {
            ShellType::Bash => {
                script.push_str("#!/bin/bash\n");
                script.push_str("# Generated by rez-core\n\n");

                for (name, value) in &self.environment_vars {
                    script.push_str(&format!("export {}=\"{}\"\n", name, value));
                }

                // Add tools to PATH
                let tools = self.get_all_tools();
                if !tools.is_empty() {
                    script.push_str("\n# Add tools to PATH\n");
                    for tool in tools {
                        script.push_str(&format!("export PATH=\"$PATH:/path/to/{}\"\n", tool));
                    }
                }
            }
            ShellType::Cmd => {
                script.push_str("@echo off\n");
                script.push_str("REM Generated by rez-core\n\n");

                for (name, value) in &self.environment_vars {
                    script.push_str(&format!("set {}={}\n", name, value));
                }

                // Add tools to PATH
                let tools = self.get_all_tools();
                if !tools.is_empty() {
                    script.push_str("\nREM Add tools to PATH\n");
                    for tool in tools {
                        script.push_str(&format!("set PATH=%PATH%;C:\\path\\to\\{}\n", tool));
                    }
                }
            }
            ShellType::PowerShell => {
                script.push_str("# Generated by rez-core\n\n");

                for (name, value) in &self.environment_vars {
                    script.push_str(&format!("$env:{} = \"{}\"\n", name, value));
                }

                // Add tools to PATH
                let tools = self.get_all_tools();
                if !tools.is_empty() {
                    script.push_str("\n# Add tools to PATH\n");
                    for tool in tools {
                        script.push_str(&format!("$env:PATH += \";C:\\path\\to\\{}\"\n", tool));
                    }
                }
            }
        }

        script
    }
}

/// Supported shell types
#[derive(Debug, Clone, PartialEq)]
pub enum ShellType {
    /// Bash shell
    Bash,
    /// Windows Command Prompt
    Cmd,
    /// PowerShell
    PowerShell,
}

impl Default for ResolutionResult {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
