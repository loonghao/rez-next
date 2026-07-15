//! Resolved context implementation - equivalent to Python's ResolvedContext

use rez_next_common::RezCoreError;
use rez_next_package::{Package, Requirement};
use rez_next_version::Version;
use std::collections::HashMap;
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
            user: whoami::username().unwrap_or_else(|_| String::from("unknown")),
            host: whoami::hostname().unwrap_or_else(|_| String::from("unknown")),
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
        if let Some(resolved_pkg) = self.get_package(package_name)
            && let Some(variant_index) = resolved_pkg.variant_index
        {
            return resolved_pkg.package.variants.get(variant_index);
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
        if let Some(assignment) = command.strip_prefix("export ")
            && let Some((var_name, value)) = assignment.split_once('=')
        {
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
    pub fn get_summary(&self) -> ResolvedContextSummary {
        ResolvedContextSummary {
            num_packages: self.resolved_packages.len(),
            package_names: self.get_package_names(),
            failed: self.failed,
            timestamp: self.timestamp,
            requirements: self.requirements.iter().map(|r| r.to_string()).collect(),
        }
    }
}

/// Summary information about a resolved context
#[derive(Debug, Clone)]
pub struct ResolvedContextSummary {
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

/// Custom serialization for `Arc<Package>`
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

#[cfg(test)]
mod tests {
    use super::*;
    use rez_next_package::Package;
    use std::path::PathBuf;
    use std::sync::Arc;

    // Helper to create a test package
    fn make_package(name: &str, version: &str) -> Arc<Package> {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(rez_next_version::Version::parse(version).unwrap());
        Arc::new(pkg)
    }

    // Helper to create a test resolved package
    fn make_resolved_package(name: &str, version: &str) -> ResolvedPackage {
        let pkg = make_package(name, version);
        ResolvedPackage::new(pkg, PathBuf::from(format!("/packages/{}", name)), true)
    }

    #[test]
    fn test_rez_resolved_context_new() {
        let reqs = vec![];
        let ctx = RezResolvedContext::new(reqs);
        assert!(!ctx.failed);
        assert_eq!(ctx.resolved_packages.len(), 0);
        assert_eq!(ctx.requirements.len(), 0);
        assert!(ctx.failure_description.is_none());
        assert!(!ctx.user.is_empty());
        assert!(!ctx.host.is_empty());
    }

    #[test]
    fn test_rez_resolved_context_with_requirements() {
        let reqs = vec![
            rez_next_package::Requirement::new("python".to_string()),
            rez_next_package::Requirement::new("maya".to_string()),
        ];
        let ctx = RezResolvedContext::new(reqs);
        assert_eq!(ctx.requirements.len(), 2);
    }

    #[test]
    fn test_get_package_names_empty() {
        let ctx = RezResolvedContext::new(vec![]);
        let names = ctx.get_package_names();
        assert!(names.is_empty());
    }

    #[test]
    fn test_get_package_names_with_packages() {
        let mut ctx = RezResolvedContext::new(vec![]);
        ctx.resolved_packages
            .push(make_resolved_package("python", "3.9.0"));
        ctx.resolved_packages
            .push(make_resolved_package("maya", "2024.0.0"));
        ctx.resolved_packages
            .push(make_resolved_package("numpy", "1.24.0"));

        let names = ctx.get_package_names();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"python".to_string()));
        assert!(names.contains(&"maya".to_string()));
        assert!(names.contains(&"numpy".to_string()));
    }

    #[test]
    fn test_resolved_package_new() {
        let pkg = make_package("test_pkg", "1.0.0");
        let root = PathBuf::from("/packages/test_pkg");
        let rp = ResolvedPackage::new(pkg.clone(), root.clone(), true);

        assert_eq!(rp.root, root);
        assert!(rp.requested);
        assert!(rp.variant_index.is_none());
        assert!(rp.conflict.is_none());
        assert_eq!(rp.parent_packages.len(), 0);
        assert_eq!(rp.package.name, "test_pkg");
    }

    #[test]
    fn test_resolved_package_with_variant() {
        let pkg = make_package("test_pkg", "1.0.0");
        let rp =
            ResolvedPackage::new(pkg, PathBuf::from("/packages/test_pkg"), true).with_variant(2);

        assert_eq!(rp.variant_index, Some(2));
    }

    #[test]
    fn test_resolved_package_add_parent() {
        let pkg = make_package("child", "1.0.0");
        let mut rp = ResolvedPackage::new(pkg, PathBuf::from("/packages/child"), false);

        rp.add_parent("parent1".to_string());
        assert_eq!(rp.parent_packages.len(), 1);
        assert_eq!(rp.parent_packages[0], "parent1");

        // Adding duplicate should not increase count
        rp.add_parent("parent1".to_string());
        assert_eq!(rp.parent_packages.len(), 1);

        rp.add_parent("parent2".to_string());
        assert_eq!(rp.parent_packages.len(), 2);
    }

    #[test]
    fn test_rez_resolved_context_failed() {
        let mut ctx = RezResolvedContext::new(vec![]);
        ctx.failed = true;
        ctx.failure_description = Some("Dependency conflict".to_string());

        assert!(ctx.failed);
        assert_eq!(
            ctx.failure_description,
            Some("Dependency conflict".to_string())
        );
    }

    #[test]
    fn test_rez_resolved_context_metadata() {
        let mut ctx = RezResolvedContext::new(vec![]);
        ctx.metadata
            .insert("build_type".to_string(), "release".to_string());
        ctx.metadata
            .insert("target".to_string(), "linux".to_string());

        assert_eq!(ctx.metadata.len(), 2);
        assert_eq!(ctx.metadata.get("build_type"), Some(&"release".to_string()));
    }

    #[test]
    fn test_get_summary_empty_context() {
        let ctx = RezResolvedContext::new(vec![]);
        let summary = ctx.get_summary();

        assert_eq!(summary.num_packages, 0);
        assert!(summary.package_names.is_empty());
        assert!(!summary.failed);
        assert_eq!(summary.requirements.len(), 0);
    }

    #[test]
    fn test_get_summary_with_packages() {
        let mut ctx = RezResolvedContext::new(vec![]);
        ctx.resolved_packages
            .push(make_resolved_package("python", "3.9.0"));
        ctx.resolved_packages
            .push(make_resolved_package("maya", "2024.0.0"));

        let summary = ctx.get_summary();
        assert_eq!(summary.num_packages, 2);
        assert_eq!(summary.package_names.len(), 2);
    }

    #[test]
    fn test_resolved_context_clone() {
        let ctx = RezResolvedContext::new(vec![]);
        let cloned = ctx.clone();

        assert_eq!(ctx.failed, cloned.failed);
        assert_eq!(ctx.user, cloned.user);
        assert_eq!(ctx.platform, cloned.platform);
    }
}
