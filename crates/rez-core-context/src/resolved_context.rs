//! Resolved context implementation - equivalent to Python's ResolvedContext

use rez_core_common::RezCoreError;
use rez_core_package::{Package, Requirement};
use rez_core_version::Version;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

/// A resolved context represents a specific combination of packages that satisfy requirements
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RezResolvedContext {
    /// The packages that make up this context
    pub resolved_packages: Vec<ResolvedPackage>,

    /// The original requirements that led to this resolution
    pub requirements: Vec<Requirement>,

    /// Environment variables set by this context
    pub environ: HashMap<String, String>,

    /// The suite path for this context (where context files are stored)
    pub suite_path: Option<PathBuf>,

    /// Whether this context failed to resolve
    pub failed: bool,

    /// Failure description if resolution failed
    pub failure_description: Option<String>,

    /// The timestamp when this context was created
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// The user who created this context
    pub user: String,

    /// The host where this context was created
    pub host: String,

    /// Platform information
    pub platform: String,

    /// Architecture information
    pub arch: String,

    /// The Rez version used to create this context
    pub rez_version: String,

    /// Context metadata
    pub metadata: HashMap<String, String>,
}

/// A resolved package within a context
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResolvedPackage {
    /// The package definition
    #[serde(with = "arc_package_serde")]
    pub package: Arc<Package>,

    /// The variant index if this package has variants
    pub variant_index: Option<usize>,

    /// The root path where this package is installed
    pub root: PathBuf,

    /// Whether this package was explicitly requested
    pub requested: bool,

    /// The conflict that caused this package to be selected (if any)
    pub conflict: Option<String>,

    /// The parent packages that caused this package to be included
    pub parent_packages: Vec<String>,
}

impl RezResolvedContext {
    /// Create a new resolved context
    pub fn new(requirements: Vec<Requirement>) -> Self {
        Self {
            resolved_packages: Vec::new(),
            requirements,
            environ: HashMap::new(),
            suite_path: None,
            failed: false,
            failure_description: None,
            timestamp: chrono::Utc::now(),
            user: whoami::username(),
            host: whoami::hostname(),
            platform: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            rez_version: env!("CARGO_PKG_VERSION").to_string(),
            metadata: HashMap::new(),
        }
    }

    /// Get all package names in this context
    pub fn get_package_names(&self) -> Vec<String> {
        self.resolved_packages
            .iter()
            .map(|rp| rp.package.name.clone())
            .collect()
    }

    /// Get a specific package by name
    pub fn get_package(&self, name: &str) -> Option<&ResolvedPackage> {
        self.resolved_packages
            .iter()
            .find(|rp| rp.package.name == name)
    }

    /// Get the variant of a package if it has variants
    pub fn get_variant(&self, package_name: &str) -> Option<&Vec<String>> {
        if let Some(resolved_pkg) = self.get_package(package_name) {
            if let Some(variant_index) = resolved_pkg.variant_index {
                return resolved_pkg.package.variants.get(variant_index);
            }
        }
        None
    }

    /// Check if this context contains a specific package
    pub fn has_package(&self, name: &str) -> bool {
        self.resolved_packages
            .iter()
            .any(|rp| rp.package.name == name)
    }

    /// Get the version of a specific package
    pub fn get_package_version(&self, name: &str) -> Option<&Version> {
        self.get_package(name)
            .and_then(|rp| rp.package.version.as_ref())
    }

    /// Get all tools provided by packages in this context
    pub fn get_tools(&self) -> HashMap<String, PathBuf> {
        let mut tools = HashMap::new();

        for resolved_pkg in &self.resolved_packages {
            for tool in &resolved_pkg.package.tools {
                // Tool path is typically {root}/bin/{tool}
                let tool_path = resolved_pkg.root.join("bin").join(tool);
                tools.insert(tool.clone(), tool_path);
            }
        }

        tools
    }

    /// Generate environment variables for this context
    pub fn get_environ(&self) -> Result<HashMap<String, String>, RezCoreError> {
        let mut environ = HashMap::new();

        // Start with system environment
        for (key, value) in std::env::vars() {
            environ.insert(key, value);
        }

        // Apply package environments in dependency order
        for resolved_pkg in &self.resolved_packages {
            if let Some(ref commands) = resolved_pkg.package.commands {
                self.apply_package_commands(commands, resolved_pkg, &mut environ)?;
            }
        }

        Ok(environ)
    }

    /// Apply commands from a package to the environment
    fn apply_package_commands(
        &self,
        commands: &str,
        resolved_pkg: &ResolvedPackage,
        environ: &mut HashMap<String, String>,
    ) -> Result<(), RezCoreError> {
        // Parse and apply shell commands
        for line in commands.lines() {
            let line = line.trim();
            if line.starts_with("export ") {
                self.apply_export_command(line, resolved_pkg, environ)?;
            }
        }
        Ok(())
    }

    /// Apply an export command to the environment
    fn apply_export_command(
        &self,
        command: &str,
        resolved_pkg: &ResolvedPackage,
        environ: &mut HashMap<String, String>,
    ) -> Result<(), RezCoreError> {
        // Parse export VAR="value" or export VAR="${VAR}:value"
        if let Some(assignment) = command.strip_prefix("export ") {
            if let Some((var_name, value)) = assignment.split_once('=') {
                let var_name = var_name.trim();
                let value = value.trim_matches('"');

                // Expand variables in the value
                let expanded_value = self.expand_variables(value, resolved_pkg, environ);

                // Handle path-like variables (containing ${VAR}:)
                if value.contains(&format!("${{{}}}:", var_name)) {
                    // Append to existing value
                    let existing = environ.get(var_name).cloned().unwrap_or_default();
                    let new_value = expanded_value
                        .replace(&format!("${{{}}}:", var_name), &format!("{}:", existing));
                    environ.insert(var_name.to_string(), new_value);
                } else if value.contains(&format!(":${{{}}}", var_name)) {
                    // Prepend to existing value
                    let existing = environ.get(var_name).cloned().unwrap_or_default();
                    let new_value = expanded_value
                        .replace(&format!(":${{{}}}", var_name), &format!(":{}", existing));
                    environ.insert(var_name.to_string(), new_value);
                } else {
                    // Simple assignment
                    environ.insert(var_name.to_string(), expanded_value);
                }
            }
        }
        Ok(())
    }

    /// Expand variables in a value string
    fn expand_variables(
        &self,
        value: &str,
        resolved_pkg: &ResolvedPackage,
        environ: &HashMap<String, String>,
    ) -> String {
        let mut result = value.to_string();

        // Expand {root}
        result = result.replace("{root}", &resolved_pkg.root.to_string_lossy());

        // Expand {version}
        if let Some(ref version) = resolved_pkg.package.version {
            result = result.replace("{version}", version.as_str());
        }

        // Expand {variant_index}
        if let Some(variant_index) = resolved_pkg.variant_index {
            result = result.replace("{variant_index}", &variant_index.to_string());
        }

        // Expand environment variables ${VAR}
        for (env_var, env_value) in environ {
            let pattern = format!("${{{}}}", env_var);
            result = result.replace(&pattern, env_value);
        }

        result
    }

    /// Save this context to a file
    pub fn save(&self, path: &PathBuf) -> Result<(), RezCoreError> {
        let serialized = serde_json::to_string_pretty(self)?;
        std::fs::write(path, serialized)?;

        Ok(())
    }

    /// Load a context from a file
    pub fn load(path: &PathBuf) -> Result<Self, RezCoreError> {
        let content = std::fs::read_to_string(path)?;
        let context: Self = serde_json::from_str(&content)?;

        Ok(context)
    }

    /// Get a summary of this context
    pub fn get_summary(&self) -> ContextSummary {
        ContextSummary {
            num_packages: self.resolved_packages.len(),
            package_names: self.get_package_names(),
            failed: self.failed,
            timestamp: self.timestamp,
            requirements: self.requirements.iter().map(|r| r.to_string()).collect(),
        }
    }
}

/// Summary information about a context
#[derive(Debug, Clone)]
pub struct ContextSummary {
    pub num_packages: usize,
    pub package_names: Vec<String>,
    pub failed: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub requirements: Vec<String>,
}

impl ResolvedPackage {
    /// Create a new resolved package
    pub fn new(package: Arc<Package>, root: PathBuf, requested: bool) -> Self {
        Self {
            package,
            variant_index: None,
            root,
            requested,
            conflict: None,
            parent_packages: Vec::new(),
        }
    }

    /// Set the variant index for this package
    pub fn with_variant(mut self, variant_index: usize) -> Self {
        self.variant_index = Some(variant_index);
        self
    }

    /// Add a parent package that caused this package to be included
    pub fn add_parent(&mut self, parent: String) {
        if !self.parent_packages.contains(&parent) {
            self.parent_packages.push(parent);
        }
    }
}

/// Custom serialization for Arc<Package>
mod arc_package_serde {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(package: &Arc<Package>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        package.as_ref().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Arc<Package>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let package = Package::deserialize(deserializer)?;
        Ok(Arc::new(package))
    }
}
