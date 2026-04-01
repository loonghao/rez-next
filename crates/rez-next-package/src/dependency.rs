//! Package dependency resolution and analysis

use crate::{requirement::Requirement, Package};
use rez_next_common::RezCoreError;
use rez_next_version::Version;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

/// Dependency resolution strategy
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionStrategy {
    /// Use the latest compatible version
    Latest,
    /// Use the earliest compatible version
    Earliest,
    /// Use the most stable version (prefer releases over pre-releases)
    Stable,
    /// Use exact version matching only
    Exact,
    /// Use custom resolution logic
    Custom(String),
}

/// Dependency conflict resolution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Fail on any conflict
    Strict,
    /// Use the highest version that satisfies all constraints
    Highest,
    /// Use the lowest version that satisfies all constraints
    Lowest,
    /// Allow conflicts and use the first found version
    Permissive,
    /// Use custom conflict resolution logic
    Custom(String),
}

/// Dependency resolution options
#[derive(Debug, Clone)]
pub struct DependencyResolutionOptions {
    /// Resolution strategy
    pub strategy: ResolutionStrategy,
    /// Conflict resolution method
    pub conflict_resolution: ConflictResolution,
    /// Maximum dependency depth
    pub max_depth: usize,
    /// Include development dependencies
    pub include_dev_deps: bool,
    /// Include build dependencies
    pub include_build_deps: bool,
    /// Include private build dependencies
    pub include_private_build_deps: bool,
    /// Allow pre-release versions
    pub allow_prerelease: bool,
    /// Package repositories to search
    pub repositories: Vec<String>,
    /// Excluded packages
    pub excluded_packages: HashSet<String>,
    /// Version overrides
    pub version_overrides: HashMap<String, String>,
}

/// Dependency node in the resolution graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyNode {
    /// Package name
    pub package_name: String,
    /// Resolved version
    pub version: Option<Version>,
    /// Original requirement string
    pub requirement: String,
    /// Dependency type
    pub dependency_type: DependencyType,
    /// Depth in the dependency tree
    pub depth: usize,
    /// Parent package that required this dependency
    pub parent: Option<String>,
    /// Child dependencies
    pub children: Vec<String>,
    /// Whether this is a direct dependency
    pub is_direct: bool,
    /// Whether this dependency is optional
    pub is_optional: bool,
    /// Platform constraints
    pub platform_constraints: Vec<String>,
}

/// Type of dependency
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyType {
    /// Runtime dependency
    Runtime,
    /// Build dependency
    Build,
    /// Private build dependency
    PrivateBuild,
    /// Development dependency
    Development,
    /// Optional dependency
    Optional,
    /// Test dependency
    Test,
}

/// Dependency resolution result
#[derive(Debug, Clone)]
pub struct DependencyResolutionResult {
    /// Whether resolution was successful
    pub success: bool,
    /// Resolved dependency graph
    pub dependency_graph: HashMap<String, DependencyNode>,
    /// Resolution order (topologically sorted)
    pub resolution_order: Vec<String>,
    /// Conflicts found during resolution
    pub conflicts: Vec<DependencyConflict>,
    /// Warnings generated during resolution
    pub warnings: Vec<String>,
    /// Resolution statistics
    pub statistics: ResolutionStatistics,
}

/// Dependency conflict information
#[derive(Debug, Clone)]
pub struct DependencyConflict {
    /// Package name with conflict
    pub package_name: String,
    /// Conflicting requirements
    pub conflicting_requirements: Vec<String>,
    /// Packages that introduced the conflicting requirements
    pub sources: Vec<String>,
    /// Suggested resolution
    pub suggested_resolution: Option<String>,
}

/// Resolution statistics
#[derive(Debug, Clone)]
pub struct ResolutionStatistics {
    /// Total packages resolved
    pub total_packages: usize,
    /// Direct dependencies
    pub direct_dependencies: usize,
    /// Transitive dependencies
    pub transitive_dependencies: usize,
    /// Maximum depth reached
    pub max_depth_reached: usize,
    /// Resolution time in milliseconds
    pub resolution_time_ms: u64,
    /// Number of conflicts resolved
    pub conflicts_resolved: usize,
}

/// Dependency resolver
pub struct DependencyResolver {
    /// Resolution options
    options: DependencyResolutionOptions,
    /// Available packages cache
    available_packages: HashMap<String, Vec<Package>>,
    /// Resolution cache
    resolution_cache: HashMap<String, DependencyResolutionResult>,
}

impl Default for ResolutionStrategy {
    fn default() -> Self {
        Self::Latest
    }
}

impl Default for ConflictResolution {
    fn default() -> Self {
        Self::Highest
    }
}

impl Default for DependencyResolutionOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyResolutionOptions {
    /// Create new default options
    pub fn new() -> Self {
        Self {
            strategy: ResolutionStrategy::Latest,
            conflict_resolution: ConflictResolution::Highest,
            max_depth: 10,
            include_dev_deps: false,
            include_build_deps: true,
            include_private_build_deps: false,
            allow_prerelease: false,
            repositories: Vec::new(),
            excluded_packages: HashSet::new(),
            version_overrides: HashMap::new(),
        }
    }

    /// Create options for development builds
    pub fn development() -> Self {
        Self {
            strategy: ResolutionStrategy::Latest,
            conflict_resolution: ConflictResolution::Highest,
            max_depth: 15,
            include_dev_deps: true,
            include_build_deps: true,
            include_private_build_deps: true,
            allow_prerelease: true,
            repositories: Vec::new(),
            excluded_packages: HashSet::new(),
            version_overrides: HashMap::new(),
        }
    }

    /// Create options for production builds
    pub fn production() -> Self {
        Self {
            strategy: ResolutionStrategy::Stable,
            conflict_resolution: ConflictResolution::Strict,
            max_depth: 10,
            include_dev_deps: false,
            include_build_deps: true,
            include_private_build_deps: false,
            allow_prerelease: false,
            repositories: Vec::new(),
            excluded_packages: HashSet::new(),
            version_overrides: HashMap::new(),
        }
    }

    /// Add repository
    pub fn add_repository(&mut self, repo: String) {
        self.repositories.push(repo);
    }

    /// Exclude package
    pub fn exclude_package(&mut self, package: String) {
        self.excluded_packages.insert(package);
    }

    /// Add version override
    pub fn add_version_override(&mut self, package: String, version: String) {
        self.version_overrides.insert(package, version);
    }
}

impl DependencyNode {
    /// Create a new dependency node
    pub fn new(
        package_name: String,
        requirement: String,
        dependency_type: DependencyType,
        depth: usize,
    ) -> Self {
        Self {
            package_name,
            version: None,
            requirement,
            dependency_type,
            depth,
            parent: None,
            children: Vec::new(),
            is_direct: depth == 0,
            is_optional: false,
            platform_constraints: Vec::new(),
        }
    }

    /// Set resolved version
    pub fn set_version(&mut self, version: Version) {
        self.version = Some(version);
    }

    /// Set parent package
    pub fn set_parent(&mut self, parent: String) {
        self.parent = Some(parent);
    }

    /// Add child dependency
    pub fn add_child(&mut self, child: String) {
        self.children.push(child);
    }

    /// Mark as optional
    pub fn set_optional(&mut self, optional: bool) {
        self.is_optional = optional;
    }

    /// Add platform constraint
    pub fn add_platform_constraint(&mut self, constraint: String) {
        self.platform_constraints.push(constraint);
    }

    /// Get qualified name (package@version)
    pub fn qualified_name(&self) -> String {
        match &self.version {
            Some(version) => format!("{}@{}", self.package_name, version.as_str()),
            None => self.package_name.clone(),
        }
    }
}

impl fmt::Display for DependencyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Runtime => write!(f, "runtime"),
            Self::Build => write!(f, "build"),
            Self::PrivateBuild => write!(f, "private-build"),
            Self::Development => write!(f, "development"),
            Self::Optional => write!(f, "optional"),
            Self::Test => write!(f, "test"),
        }
    }
}

impl fmt::Display for ResolutionStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Latest => write!(f, "latest"),
            Self::Earliest => write!(f, "earliest"),
            Self::Stable => write!(f, "stable"),
            Self::Exact => write!(f, "exact"),
            Self::Custom(strategy) => write!(f, "custom({})", strategy),
        }
    }
}

impl fmt::Display for ConflictResolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Strict => write!(f, "strict"),
            Self::Highest => write!(f, "highest"),
            Self::Lowest => write!(f, "lowest"),
            Self::Permissive => write!(f, "permissive"),
            Self::Custom(resolution) => write!(f, "custom({})", resolution),
        }
    }
}

impl DependencyResolver {
    /// Create a new dependency resolver
    pub fn new(options: DependencyResolutionOptions) -> Self {
        Self {
            options,
            available_packages: HashMap::new(),
            resolution_cache: HashMap::new(),
        }
    }

    /// Create resolver with default options
    pub fn default() -> Self {
        Self::new(DependencyResolutionOptions::new())
    }

    /// Add available packages for a package name
    pub fn add_available_packages(&mut self, package_name: String, packages: Vec<Package>) {
        self.available_packages.insert(package_name, packages);
    }

    /// Resolve dependencies for a package
    pub fn resolve_dependencies(
        &mut self,
        package: &Package,
    ) -> Result<DependencyResolutionResult, RezCoreError> {
        let start_time = std::time::Instant::now();

        // Check cache first
        let cache_key = self.generate_cache_key(package);
        if let Some(cached_result) = self.resolution_cache.get(&cache_key) {
            return Ok(cached_result.clone());
        }

        let mut result = DependencyResolutionResult {
            success: true,
            dependency_graph: HashMap::new(),
            resolution_order: Vec::new(),
            conflicts: Vec::new(),
            warnings: Vec::new(),
            statistics: ResolutionStatistics {
                total_packages: 0,
                direct_dependencies: 0,
                transitive_dependencies: 0,
                max_depth_reached: 0,
                resolution_time_ms: 0,
                conflicts_resolved: 0,
            },
        };

        // Collect all requirements
        let mut requirements = Vec::new();

        // Add runtime dependencies
        for req_str in &package.requires {
            requirements.push((req_str.clone(), DependencyType::Runtime));
        }

        // Add build dependencies if requested
        if self.options.include_build_deps {
            for req_str in &package.build_requires {
                requirements.push((req_str.clone(), DependencyType::Build));
            }
        }

        // Add private build dependencies if requested
        if self.options.include_private_build_deps {
            for req_str in &package.private_build_requires {
                requirements.push((req_str.clone(), DependencyType::PrivateBuild));
            }
        }

        // Resolve each requirement
        let mut visited = HashSet::new();
        let mut resolution_queue = VecDeque::new();

        // Add direct dependencies to queue
        for (req_str, dep_type) in requirements {
            if let Ok(requirement) = req_str.parse::<Requirement>() {
                let node = DependencyNode::new(
                    requirement.package_name().to_string(),
                    req_str,
                    dep_type,
                    0,
                );
                resolution_queue.push_back(node);
                result.statistics.direct_dependencies += 1;
            } else {
                result
                    .warnings
                    .push(format!("Invalid requirement format: {}", req_str));
            }
        }

        // Process resolution queue
        while let Some(mut node) = resolution_queue.pop_front() {
            if visited.contains(&node.package_name) {
                continue;
            }

            if node.depth > self.options.max_depth {
                result.warnings.push(format!(
                    "Maximum dependency depth ({}) exceeded for package: {}",
                    self.options.max_depth, node.package_name
                ));
                continue;
            }

            // Check if package is excluded
            if self.options.excluded_packages.contains(&node.package_name) {
                result
                    .warnings
                    .push(format!("Package excluded: {}", node.package_name));
                continue;
            }

            // Resolve version for this node
            match self.resolve_version(&node) {
                Ok(version) => {
                    node.set_version(version);

                    // Add transitive dependencies if we have package info
                    if let Some(packages) = self.available_packages.get(&node.package_name) {
                        if let Some(resolved_package) = packages.iter().find(|p| {
                            p.version
                                .as_ref()
                                .map(|v| v == &node.version.as_ref().unwrap())
                                .unwrap_or(false)
                        }) {
                            // Add transitive dependencies
                            for req_str in &resolved_package.requires {
                                if let Ok(requirement) = req_str.parse::<Requirement>() {
                                    let mut child_node = DependencyNode::new(
                                        requirement.package_name().to_string(),
                                        req_str.clone(),
                                        DependencyType::Runtime,
                                        node.depth + 1,
                                    );
                                    child_node.set_parent(node.package_name.clone());
                                    node.add_child(child_node.package_name.clone());
                                    resolution_queue.push_back(child_node);
                                    result.statistics.transitive_dependencies += 1;
                                }
                            }
                        }
                    }

                    visited.insert(node.package_name.clone());
                    result.statistics.max_depth_reached =
                        result.statistics.max_depth_reached.max(node.depth);
                    result
                        .dependency_graph
                        .insert(node.package_name.clone(), node);
                }
                Err(e) => {
                    result.success = false;
                    result
                        .warnings
                        .push(format!("Failed to resolve {}: {}", node.package_name, e));
                }
            }
        }

        // Generate resolution order (topological sort)
        result.resolution_order = self.topological_sort(&result.dependency_graph)?;

        // Update statistics
        result.statistics.total_packages = result.dependency_graph.len();
        result.statistics.resolution_time_ms = start_time.elapsed().as_millis() as u64;

        // Cache result
        self.resolution_cache.insert(cache_key, result.clone());

        Ok(result)
    }

    /// Resolve version for a dependency node
    fn resolve_version(&self, node: &DependencyNode) -> Result<Version, RezCoreError> {
        // Check for version override first
        if let Some(override_version) = self.options.version_overrides.get(&node.package_name) {
            return Version::parse(override_version).map_err(|e| {
                RezCoreError::DependencyResolution(format!(
                    "Invalid override version '{}' for package '{}': {}",
                    override_version, node.package_name, e
                ))
            });
        }

        // Parse the requirement
        let requirement = node.requirement.parse::<Requirement>().map_err(|e| {
            RezCoreError::DependencyResolution(format!(
                "Invalid requirement '{}': {}",
                node.requirement, e
            ))
        })?;

        // Get available packages
        let available_packages =
            self.available_packages
                .get(&node.package_name)
                .ok_or_else(|| {
                    RezCoreError::DependencyResolution(format!(
                        "No packages available for: {}",
                        node.package_name
                    ))
                })?;

        // Filter compatible versions
        let mut compatible_versions: Vec<&Version> = available_packages
            .iter()
            .filter_map(|pkg| pkg.version.as_ref())
            .filter(|version| requirement.is_satisfied_by(version))
            .filter(|version| self.options.allow_prerelease || !version.is_prerelease())
            .collect();

        if compatible_versions.is_empty() {
            return Err(RezCoreError::DependencyResolution(format!(
                "No compatible versions found for requirement: {}",
                node.requirement
            )));
        }

        // Apply resolution strategy
        let selected_version = match self.options.strategy {
            ResolutionStrategy::Latest => {
                compatible_versions.sort();
                compatible_versions.last().unwrap()
            }
            ResolutionStrategy::Earliest => {
                compatible_versions.sort();
                compatible_versions.first().unwrap()
            }
            ResolutionStrategy::Stable => {
                // Prefer stable versions over pre-releases
                let stable_versions: Vec<&Version> = compatible_versions
                    .iter()
                    .filter(|v| !v.is_prerelease())
                    .copied()
                    .collect();

                if !stable_versions.is_empty() {
                    let mut stable = stable_versions;
                    stable.sort();
                    stable.last().unwrap()
                } else {
                    compatible_versions.sort();
                    compatible_versions.last().unwrap()
                }
            }
            ResolutionStrategy::Exact => {
                // For exact matching, we need to find the exact version specified
                // This is a simplified implementation
                compatible_versions.sort();
                compatible_versions.last().unwrap()
            }
            ResolutionStrategy::Custom(_) => {
                // Custom resolution logic would be implemented here
                compatible_versions.sort();
                compatible_versions.last().unwrap()
            }
        };

        Ok(selected_version.clone())
    }

    /// Perform topological sort on dependency graph
    fn topological_sort(
        &self,
        graph: &HashMap<String, DependencyNode>,
    ) -> Result<Vec<String>, RezCoreError> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_visited = HashSet::new();

        for node_name in graph.keys() {
            if !visited.contains(node_name) {
                self.topological_sort_visit(
                    node_name,
                    graph,
                    &mut visited,
                    &mut temp_visited,
                    &mut result,
                )?;
            }
        }

        result.reverse();
        Ok(result)
    }

    /// Recursive helper for topological sort
    fn topological_sort_visit(
        &self,
        node_name: &str,
        graph: &HashMap<String, DependencyNode>,
        visited: &mut HashSet<String>,
        temp_visited: &mut HashSet<String>,
        result: &mut Vec<String>,
    ) -> Result<(), RezCoreError> {
        if temp_visited.contains(node_name) {
            return Err(RezCoreError::DependencyResolution(format!(
                "Circular dependency detected involving: {}",
                node_name
            )));
        }

        if visited.contains(node_name) {
            return Ok(());
        }

        temp_visited.insert(node_name.to_string());

        if let Some(node) = graph.get(node_name) {
            for child_name in &node.children {
                self.topological_sort_visit(child_name, graph, visited, temp_visited, result)?;
            }
        }

        temp_visited.remove(node_name);
        visited.insert(node_name.to_string());
        result.push(node_name.to_string());

        Ok(())
    }

    /// Generate cache key for resolution result
    fn generate_cache_key(&self, package: &Package) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        package.name.hash(&mut hasher);
        package.version.hash(&mut hasher);
        package.requires.hash(&mut hasher);
        package.build_requires.hash(&mut hasher);
        package.private_build_requires.hash(&mut hasher);

        // Include options in hash
        format!("{:?}", self.options.strategy).hash(&mut hasher);
        format!("{:?}", self.options.conflict_resolution).hash(&mut hasher);
        self.options.max_depth.hash(&mut hasher);
        self.options.include_dev_deps.hash(&mut hasher);
        self.options.include_build_deps.hash(&mut hasher);
        self.options.include_private_build_deps.hash(&mut hasher);
        self.options.allow_prerelease.hash(&mut hasher);

        format!("{:x}", hasher.finish())
    }

    /// Clear resolution cache
    pub fn clear_cache(&mut self) {
        self.resolution_cache.clear();
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        stats.insert(
            "cached_resolutions".to_string(),
            self.resolution_cache.len(),
        );
        stats.insert(
            "available_packages".to_string(),
            self.available_packages.len(),
        );
        stats
    }
}

impl DependencyResolutionResult {
    /// Create a new successful result
    pub fn success() -> Self {
        Self {
            success: true,
            dependency_graph: HashMap::new(),
            resolution_order: Vec::new(),
            conflicts: Vec::new(),
            warnings: Vec::new(),
            statistics: ResolutionStatistics::default(),
        }
    }

    /// Create a new failed result
    pub fn failure(error: String) -> Self {
        let mut result = Self::success();
        result.success = false;
        result.warnings.push(error);
        result
    }

    /// Check if resolution has conflicts
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }

    /// Check if resolution has warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Get direct dependencies
    pub fn get_direct_dependencies(&self) -> Vec<&DependencyNode> {
        self.dependency_graph
            .values()
            .filter(|node| node.is_direct)
            .collect()
    }

    /// Get transitive dependencies
    pub fn get_transitive_dependencies(&self) -> Vec<&DependencyNode> {
        self.dependency_graph
            .values()
            .filter(|node| !node.is_direct)
            .collect()
    }

    /// Get dependencies by type
    pub fn get_dependencies_by_type(&self, dep_type: &DependencyType) -> Vec<&DependencyNode> {
        self.dependency_graph
            .values()
            .filter(|node| &node.dependency_type == dep_type)
            .collect()
    }

    /// Get dependency tree as string representation
    pub fn dependency_tree_string(&self) -> String {
        let mut result = String::new();
        let direct_deps = self.get_direct_dependencies();

        for dep in direct_deps {
            self.append_dependency_tree(&mut result, dep, 0);
        }

        result
    }

    /// Recursive helper for dependency tree string
    fn append_dependency_tree(&self, result: &mut String, node: &DependencyNode, indent: usize) {
        let indent_str = "  ".repeat(indent);
        result.push_str(&format!(
            "{}{} ({})\n",
            indent_str,
            node.qualified_name(),
            node.dependency_type
        ));

        for child_name in &node.children {
            if let Some(child_node) = self.dependency_graph.get(child_name) {
                self.append_dependency_tree(result, child_node, indent + 1);
            }
        }
    }
}

impl Default for ResolutionStatistics {
    fn default() -> Self {
        Self {
            total_packages: 0,
            direct_dependencies: 0,
            transitive_dependencies: 0,
            max_depth_reached: 0,
            resolution_time_ms: 0,
            conflicts_resolved: 0,
        }
    }
}

impl DependencyConflict {
    /// Create a new dependency conflict
    pub fn new(package_name: String) -> Self {
        Self {
            package_name,
            conflicting_requirements: Vec::new(),
            sources: Vec::new(),
            suggested_resolution: None,
        }
    }

    /// Add conflicting requirement
    pub fn add_conflicting_requirement(&mut self, requirement: String, source: String) {
        self.conflicting_requirements.push(requirement);
        self.sources.push(source);
    }

    /// Set suggested resolution
    pub fn set_suggested_resolution(&mut self, resolution: String) {
        self.suggested_resolution = Some(resolution);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_package(name: &str, version: &str, requires: Vec<&str>) -> Package {
        let mut package = Package::new(name.to_string());
        package.version = Some(Version::parse(version).unwrap());
        package.requires = requires.iter().map(|s| s.to_string()).collect();
        package
    }

    #[test]
    fn test_dependency_resolution_options() {
        let options = DependencyResolutionOptions::new();
        assert_eq!(options.strategy, ResolutionStrategy::Latest);
        assert_eq!(options.conflict_resolution, ConflictResolution::Highest);
        assert_eq!(options.max_depth, 10);
        assert!(!options.include_dev_deps);
        assert!(options.include_build_deps);

        let dev_options = DependencyResolutionOptions::development();
        assert!(dev_options.include_dev_deps);
        assert!(dev_options.allow_prerelease);

        let prod_options = DependencyResolutionOptions::production();
        assert_eq!(prod_options.strategy, ResolutionStrategy::Stable);
        assert_eq!(prod_options.conflict_resolution, ConflictResolution::Strict);
        assert!(!prod_options.allow_prerelease);
    }

    #[test]
    fn test_dependency_node_creation() {
        let mut node = DependencyNode::new(
            "test_package".to_string(),
            "test_package>=1.0.0".to_string(),
            DependencyType::Runtime,
            0,
        );

        assert_eq!(node.package_name, "test_package");
        assert_eq!(node.requirement, "test_package>=1.0.0");
        assert_eq!(node.dependency_type, DependencyType::Runtime);
        assert_eq!(node.depth, 0);
        assert!(node.is_direct);
        assert!(!node.is_optional);

        node.set_version(Version::parse("1.2.3").unwrap());
        assert_eq!(node.qualified_name(), "test_package@1.2.3");

        node.set_parent("parent_package".to_string());
        assert_eq!(node.parent, Some("parent_package".to_string()));

        node.add_child("child_package".to_string());
        assert!(node.children.contains(&"child_package".to_string()));
    }

    #[test]
    fn test_dependency_resolver_creation() {
        let options = DependencyResolutionOptions::new();
        let resolver = DependencyResolver::new(options);

        assert_eq!(resolver.available_packages.len(), 0);
        assert_eq!(resolver.resolution_cache.len(), 0);
    }

    #[test]
    fn test_resolution_result() {
        let mut result = DependencyResolutionResult::success();
        assert!(result.success);
        assert!(!result.has_conflicts());
        assert!(!result.has_warnings());

        result.warnings.push("Test warning".to_string());
        assert!(result.has_warnings());

        let conflict = DependencyConflict::new("test_package".to_string());
        result.conflicts.push(conflict);
        assert!(result.has_conflicts());

        let failure_result = DependencyResolutionResult::failure("Test error".to_string());
        assert!(!failure_result.success);
        assert!(failure_result.has_warnings());
    }

    #[test]
    fn test_dependency_types_display() {
        assert_eq!(DependencyType::Runtime.to_string(), "runtime");
        assert_eq!(DependencyType::Build.to_string(), "build");
        assert_eq!(DependencyType::PrivateBuild.to_string(), "private-build");
        assert_eq!(DependencyType::Development.to_string(), "development");
        assert_eq!(DependencyType::Optional.to_string(), "optional");
        assert_eq!(DependencyType::Test.to_string(), "test");
    }

    #[test]
    fn test_resolution_strategy_display() {
        assert_eq!(ResolutionStrategy::Latest.to_string(), "latest");
        assert_eq!(ResolutionStrategy::Earliest.to_string(), "earliest");
        assert_eq!(ResolutionStrategy::Stable.to_string(), "stable");
        assert_eq!(ResolutionStrategy::Exact.to_string(), "exact");
        assert_eq!(
            ResolutionStrategy::Custom("custom_logic".to_string()).to_string(),
            "custom(custom_logic)"
        );
    }

    #[test]
    fn test_conflict_resolution_display() {
        assert_eq!(ConflictResolution::Strict.to_string(), "strict");
        assert_eq!(ConflictResolution::Highest.to_string(), "highest");
        assert_eq!(ConflictResolution::Lowest.to_string(), "lowest");
        assert_eq!(ConflictResolution::Permissive.to_string(), "permissive");
        assert_eq!(
            ConflictResolution::Custom("custom_resolution".to_string()).to_string(),
            "custom(custom_resolution)"
        );
    }

    #[test]
    fn test_dependency_conflict() {
        let mut conflict = DependencyConflict::new("test_package".to_string());
        assert_eq!(conflict.package_name, "test_package");
        assert!(conflict.conflicting_requirements.is_empty());
        assert!(conflict.sources.is_empty());
        assert!(conflict.suggested_resolution.is_none());

        conflict.add_conflicting_requirement(">=1.0.0".to_string(), "package_a".to_string());
        conflict.add_conflicting_requirement("<1.0.0".to_string(), "package_b".to_string());

        assert_eq!(conflict.conflicting_requirements.len(), 2);
        assert_eq!(conflict.sources.len(), 2);

        conflict.set_suggested_resolution("Use version 1.0.0".to_string());
        assert!(conflict.suggested_resolution.is_some());
    }
}
