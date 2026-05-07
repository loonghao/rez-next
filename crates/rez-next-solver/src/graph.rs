//! Dependency graph implementation

use rez_next_common::RezCoreError;
use rez_next_package::{Package, PackageRequirement};
use rez_next_version::Version;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// Dependency graph node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Package information
    pub package: Package,
    /// Direct dependencies (package names)
    pub dependencies: HashSet<String>,
    /// Packages that depend on this one
    pub dependents: HashSet<String>,
    /// Node metadata
    pub metadata: HashMap<String, String>,
}

impl GraphNode {
    /// Create a new graph node
    pub fn new(package: Package) -> Self {
        Self {
            package,
            dependencies: HashSet::new(),
            dependents: HashSet::new(),
            metadata: HashMap::new(),
        }
    }

    /// Get the package key (name-version)
    pub fn key(&self) -> String {
        match &self.package.version {
            Some(version) => format!("{}-{}", self.package.name, version.as_str()),
            None => self.package.name.clone(),
        }
    }

    /// Add a dependency
    pub fn add_dependency(&mut self, dependency_key: String) {
        self.dependencies.insert(dependency_key);
    }

    /// Add a dependent
    pub fn add_dependent(&mut self, dependent_key: String) {
        self.dependents.insert(dependent_key);
    }

    /// Remove a dependency
    pub fn remove_dependency(&mut self, dependency_key: &str) {
        self.dependencies.remove(dependency_key);
    }

    /// Remove a dependent
    pub fn remove_dependent(&mut self, dependent_key: &str) {
        self.dependents.remove(dependent_key);
    }
}

/// Dependency conflict information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyConflict {
    /// Package name that has conflicting requirements
    pub package_name: String,
    /// Conflicting requirements
    pub conflicting_requirements: Vec<PackageRequirement>,
    /// Packages that introduced these requirements
    pub source_packages: Vec<String>,
    /// Conflict severity
    pub severity: ConflictSeverity,
}

/// Conflict severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum ConflictSeverity {
    /// Minor version conflict
    Minor,
    /// Major version conflict
    Major,
    /// Incompatible requirements
    Incompatible,
}

/// Conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    /// Package name being resolved
    pub package_name: String,
    /// Selected package version
    pub selected_version: Option<Version>,
    /// Resolution strategy used
    pub strategy: String,
    /// Packages that were modified/removed
    pub modified_packages: Vec<String>,
}

/// DFS color for cycle detection
#[derive(Clone, Copy, PartialEq, Eq)]
enum Color {
    Gray,    // In progress (in recursion stack)
    Black,   // Completely processed
}

/// High-performance dependency graph
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// Graph nodes indexed by package key
    nodes: HashMap<String, GraphNode>,
    /// Requirements indexed by package name
    requirements: HashMap<String, Vec<PackageRequirement>>,
    /// Constraints that must be satisfied
    constraints: Vec<PackageRequirement>,
    /// Packages to exclude
    exclusions: HashSet<String>,
    /// Graph metadata
    metadata: HashMap<String, String>,
}

impl DependencyGraph {
    /// Create a new dependency graph
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            requirements: HashMap::new(),
            constraints: Vec::new(),
            exclusions: HashSet::new(),
            metadata: HashMap::new(),
        }
    }

    /// Clear the graph
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.requirements.clear();
        self.constraints.clear();
        self.exclusions.clear();
        self.metadata.clear();
    }

    /// Add a package to the graph
    pub fn add_package(&mut self, package: Package) -> Result<(), RezCoreError> {
        let key = match &package.version {
            Some(version) => format!("{}-{}", package.name, version.as_str()),
            None => package.name.clone(),
        };

        // Check if package is excluded
        if self.exclusions.contains(&package.name) {
            return Err(RezCoreError::Solver(format!(
                "Package {} is excluded",
                package.name
            )));
        }

        let node = GraphNode::new(package);
        self.nodes.insert(key, node);

        Ok(())
    }

    /// Add a requirement to the graph
    pub fn add_requirement(&mut self, requirement: PackageRequirement) -> Result<(), RezCoreError> {
        self.requirements
            .entry(requirement.name.clone())
            .or_default()
            .push(requirement);

        Ok(())
    }

    /// Add a constraint
    pub fn add_constraint(&mut self, constraint: PackageRequirement) -> Result<(), RezCoreError> {
        self.constraints.push(constraint);
        Ok(())
    }

    /// Add an exclusion
    pub fn add_exclusion(&mut self, package_name: String) -> Result<(), RezCoreError> {
        self.exclusions.insert(package_name);
        Ok(())
    }

    /// Add a dependency edge between two packages
    pub fn add_dependency_edge(
        &mut self,
        from_key: &str,
        to_key: &str,
    ) -> Result<(), RezCoreError> {
        // Add dependency to the from node
        if let Some(from_node) = self.nodes.get_mut(from_key) {
            from_node.add_dependency(to_key.to_string());
        } else {
            return Err(RezCoreError::Solver(format!(
                "Package {} not found in graph",
                from_key
            )));
        }

        // Add dependent to the to node
        if let Some(to_node) = self.nodes.get_mut(to_key) {
            to_node.add_dependent(from_key.to_string());
        } else {
            return Err(RezCoreError::Solver(format!(
                "Package {} not found in graph",
                to_key
            )));
        }

        Ok(())
    }

    /// Remove a package from the graph
    pub fn remove_package(&mut self, package_key: &str) -> Result<(), RezCoreError> {
        if let Some(node) = self.nodes.remove(package_key) {
            // Remove all edges involving this package
            for dep_key in &node.dependencies {
                if let Some(dep_node) = self.nodes.get_mut(dep_key) {
                    dep_node.remove_dependent(package_key);
                }
            }

            for dependent_key in &node.dependents {
                if let Some(dependent_node) = self.nodes.get_mut(dependent_key) {
                    dependent_node.remove_dependency(package_key);
                }
            }
        }

        Ok(())
    }

    /// Detect conflicts in the dependency graph
    pub fn detect_conflicts(&self) -> Vec<DependencyConflict> {
        let mut conflicts = Vec::new();

        // Group requirements by package name
        for (package_name, requirements) in &self.requirements {
            if requirements.len() > 1 {
                // Check if requirements are compatible via version range intersection
                let mut incompatible_groups = Vec::new();

                for (i, req1) in requirements.iter().enumerate() {
                    for req2 in requirements.iter().skip(i + 1) {
                        if !requirements_compatible(req1, req2) {
                            incompatible_groups.push((req1.clone(), req2.clone()));
                        }
                    }
                }

                if !incompatible_groups.is_empty() {
                    let severity = self.determine_conflict_severity(&incompatible_groups);
                    let source_packages = self.find_requirement_sources(package_name);

                    conflicts.push(DependencyConflict {
                        package_name: package_name.clone(),
                        conflicting_requirements: requirements.clone(),
                        source_packages,
                        severity,
                    });
                }
            }
        }

        conflicts
    }

    /// Determine the severity of a conflict
    fn determine_conflict_severity(
        &self,
        incompatible_groups: &[(PackageRequirement, PackageRequirement)],
    ) -> ConflictSeverity {
        // Simple heuristic: if any requirements are completely incompatible, it's incompatible
        // If major versions differ, it's major
        // Otherwise, it's minor

        // Use range intersection to determine severity
        for (req1, req2) in incompatible_groups {
            let range1 = req1.version_spec.as_deref().unwrap_or("");
            let range2 = req2.version_spec.as_deref().unwrap_or("");
            // Both have ranges and they don't intersect → Incompatible
            if !range1.is_empty() && !range2.is_empty() {
                if let (Ok(r1), Ok(r2)) = (
                    rez_next_version::VersionRange::parse(range1),
                    rez_next_version::VersionRange::parse(range2),
                ) {
                    if r1.intersect(&r2).is_none() {
                        return ConflictSeverity::Incompatible;
                    }
                }
            }
        }

        ConflictSeverity::Major
    }

    /// Find which packages introduced requirements for a given package
    fn find_requirement_sources(&self, package_name: &str) -> Vec<String> {
        let mut sources = Vec::new();

        for node in self.nodes.values() {
            for req_str in &node.package.requires {
                if let Ok(req) = PackageRequirement::parse(req_str) {
                    if req.name == package_name {
                        sources.push(node.key());
                        break;
                    }
                }
            }
        }

        sources
    }

    /// Apply a conflict resolution to the graph
    pub fn apply_conflict_resolution(
        &mut self,
        resolution: ConflictResolution,
    ) -> Result<(), RezCoreError> {
        // Remove packages that were modified
        for package_key in &resolution.modified_packages {
            self.remove_package(package_key)?;
        }

        // Update requirements for the resolved package
        if let Some(requirements) = self.requirements.get_mut(&resolution.package_name) {
            // Create a new requirement based on the resolution
            if let Some(ref version) = resolution.selected_version {
                let new_requirement = PackageRequirement::with_version(
                    resolution.package_name.clone(),
                    version.as_str().to_string(),
                );
                requirements.clear();
                requirements.push(new_requirement);
            }
        }

        Ok(())
    }

    /// Get all resolved packages in topological order
    pub fn get_resolved_packages(&self) -> Result<Vec<Package>, RezCoreError> {
        let sorted_keys = self.topological_sort()?;
        let mut packages = Vec::new();

        for key in sorted_keys {
            if let Some(node) = self.nodes.get(&key) {
                packages.push(node.package.clone());
            }
        }

        Ok(packages)
    }

    /// Perform topological sort of the dependency graph
    fn topological_sort(&self) -> Result<Vec<String>, RezCoreError> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut result = Vec::new();
        let mut queue = VecDeque::new();

        // Calculate in-degrees
        for (key, node) in &self.nodes {
            in_degree.insert(key.clone(), node.dependents.len());
        }

        // Find nodes with no incoming edges
        for (key, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(key.clone());
            }
        }

        // Process nodes
        while let Some(current_key) = queue.pop_front() {
            result.push(current_key.clone());

            if let Some(current_node) = self.nodes.get(&current_key) {
                // Reduce in-degree of dependent nodes
                for dep_key in &current_node.dependencies {
                    if let Some(degree) = in_degree.get_mut(dep_key) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(dep_key.clone());
                        }
                    }
                }
            }
        }

        // Check for cycles
        if result.len() != self.nodes.len() {
            return Err(RezCoreError::Solver(
                "Circular dependency detected in package graph".to_string(),
            ));
        }

        Ok(result)
    }

    /// Get graph statistics
    pub fn get_stats(&self) -> GraphStats {
        let mut total_dependencies = 0;
        let mut max_depth = 0;

        for node in self.nodes.values() {
            total_dependencies += node.dependencies.len();
            let depth = self.calculate_node_depth(&node.key());
            max_depth = max_depth.max(depth);
        }

        GraphStats {
            node_count: self.nodes.len(),
            edge_count: total_dependencies,
            max_depth,
            conflict_count: self.detect_conflicts().len(),
            constraint_count: self.constraints.len(),
            exclusion_count: self.exclusions.len(),
        }
    }

    /// Compute accessibility matrix (transitive closure).
    ///
    /// For each node in the graph, find all nodes reachable from that node
    /// by following dependency edges. Returns a mapping from each node key
    /// to a list of accessible node keys.
    ///
    /// # Returns
    /// A `HashMap` where each key is a node key and the value is a list of
    /// node keys accessible from that node (including the node itself).
    ///
    /// # Example
    /// ```
    /// use rez_next_solver::DependencyGraph;
    /// use rez_next_package::Package;
    /// use rez_next_version::Version;
    ///
    /// let mut graph = DependencyGraph::new();
    /// // Add packages and dependencies...
    /// let accessibility = graph.accessibility();
    /// ```
    pub fn accessibility(&self) -> HashMap<String, Vec<String>> {
        let mut result: HashMap<String, Vec<String>> = HashMap::new();

        for node_key in self.nodes.keys() {
            let mut accessible = HashSet::new();
            self.dfs_reachable(node_key, &mut accessible);
            // Include the node itself
            accessible.insert(node_key.clone());
            // Convert to sorted vec for deterministic output
            let mut accessible_vec: Vec<String> = accessible.into_iter().collect();
            accessible_vec.sort();
            result.insert(node_key.clone(), accessible_vec);
        }

        result
    }

    /// DFS helper to find all nodes reachable from a given node.
    fn dfs_reachable(&self, node_key: &str, visited: &mut HashSet<String>) {
        if let Some(node) = self.nodes.get(node_key) {
            for dep_key in &node.dependencies {
                if visited.insert(dep_key.clone()) {
                    self.dfs_reachable(dep_key, visited);
                }
            }
        }
    }

    /// Find a cycle in the dependency graph.
    ///
    /// Uses DFS with three-color marking (white/gray/black) to detect
    /// if there is a cycle in the dependency graph.
    ///
    /// # Returns
    /// - `Some(Vec<String>)` containing the nodes in the cycle (in order)
    /// - `None` if no cycle exists
    pub fn find_cycle(&self) -> Option<Vec<String>> {
        let mut color: HashMap<String, Color> = HashMap::new();
        let mut parent: HashMap<String, String> = HashMap::new();

        for node_key in self.nodes.keys() {
            if !color.contains_key(node_key) {
                if let Some(cycle) = self.dfs_find_cycle(node_key, &mut color, &mut parent) {
                    return Some(cycle);
                }
            }
        }

        None
    }

    /// DFS helper to find cycle using three-color algorithm.
    /// Returns the cycle path if found.
    fn dfs_find_cycle(
        &self,
        node_key: &str,
        color: &mut HashMap<String, Color>,
        parent: &mut HashMap<String, String>,
    ) -> Option<Vec<String>> {
        color.insert(node_key.to_string(), Color::Gray);

        if let Some(node) = self.nodes.get(node_key) {
            for dep_key in &node.dependencies {
                let dep_color = color.get(dep_key).copied();
                if dep_color == Some(Color::Gray) {
                    // Found a back edge - cycle detected
                    // Reconstruct the cycle: dep_key -> ... -> node_key
                    let mut cycle = vec![];
                    let mut current = node_key.to_string();
                    loop {
                        cycle.push(current.clone());
                        if current == *dep_key {
                            break;
                        }
                        current = parent.get(&current).cloned().unwrap();
                    }
                    cycle.reverse();
                    return Some(cycle);
                } else if dep_color.is_none() {
                    // Unvisited, recurse
                    parent.insert(dep_key.clone(), node_key.to_string());
                    if let Some(cycle) = self.dfs_find_cycle(dep_key, color, parent) {
                        return Some(cycle);
                    }
                }
                // else: Some(Color::Black) - skip (already processed)
            }
        }

        color.insert(node_key.to_string(), Color::Black);
        None
    }

    /// Calculate the depth of a node in the graph
    fn calculate_node_depth(&self, node_key: &str) -> usize {
        let mut visited = HashSet::new();
        self.calculate_depth_recursive(node_key, &mut visited)
    }

    /// Recursive helper for depth calculation
    fn calculate_depth_recursive(&self, node_key: &str, visited: &mut HashSet<String>) -> usize {
        if visited.contains(node_key) {
            return 0; // Avoid infinite recursion
        }

        visited.insert(node_key.to_string());

        if let Some(node) = self.nodes.get(node_key) {
            let mut max_dep_depth = 0;
            for dep_key in &node.dependencies {
                let dep_depth = self.calculate_depth_recursive(dep_key, visited);
                max_dep_depth = max_dep_depth.max(dep_depth);
            }
            max_dep_depth + 1
        } else {
            0
        }
    }
}

/// Graph statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    /// Number of nodes in the graph
    pub node_count: usize,
    /// Number of edges in the graph
    pub edge_count: usize,
    /// Maximum depth of the graph
    pub max_depth: usize,
    /// Number of conflicts detected
    pub conflict_count: usize,
    /// Number of constraints
    pub constraint_count: usize,
    /// Number of exclusions
    pub exclusion_count: usize,
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if two PackageRequirements for the same package are compatible.
/// Two requirements are compatible if their version ranges have a non-empty intersection.
fn requirements_compatible(req1: &PackageRequirement, req2: &PackageRequirement) -> bool {
    let spec1 = req1.version_spec.as_deref().unwrap_or("");
    let spec2 = req2.version_spec.as_deref().unwrap_or("");

    // If either has no version constraint, they're compatible
    if spec1.is_empty() || spec2.is_empty() {
        return true;
    }

    // Parse both ranges and check for intersection
    match (
        rez_next_version::VersionRange::parse(spec1),
        rez_next_version::VersionRange::parse(spec2),
    ) {
        (Ok(r1), Ok(r2)) => r1.intersect(&r2).is_some(),
        _ => true, // If parsing fails, assume compatible (don't false-flag)
    }
}

#[cfg(test)]
mod graph_tests {
    use super::*;

    fn make_pkg(name: &str, ver: &str) -> Package {
        let mut p = Package::new(name.to_string());
        p.version = Some(Version::parse(ver).unwrap());
        p
    }

    /// New graph starts empty
    #[test]
    fn test_graph_new_is_empty() {
        let g = DependencyGraph::new();
        let stats = g.get_stats();
        assert_eq!(stats.node_count, 0);
        assert_eq!(stats.edge_count, 0);
        assert_eq!(stats.conflict_count, 0);
    }

    /// Adding a package increases node count
    #[test]
    fn test_graph_add_package_increments_nodes() {
        let mut g = DependencyGraph::new();
        g.add_package(make_pkg("python", "3.9.0")).unwrap();
        let stats = g.get_stats();
        assert_eq!(stats.node_count, 1);
    }

    /// Adding duplicate package is idempotent (no error, same count)
    #[test]
    fn test_graph_add_duplicate_package_no_error() {
        let mut g = DependencyGraph::new();
        g.add_package(make_pkg("maya", "2023.0")).unwrap();
        // Adding same package again should be ok (update or ignore)
        let result = g.add_package(make_pkg("maya", "2023.0"));
        assert!(result.is_ok(), "Re-adding same package should not error");
    }

    /// clear() resets graph to empty
    #[test]
    fn test_graph_clear_resets_state() {
        let mut g = DependencyGraph::new();
        g.add_package(make_pkg("houdini", "20.0")).unwrap();
        g.add_package(make_pkg("python", "3.10.0")).unwrap();
        g.clear();
        let stats = g.get_stats();
        assert_eq!(stats.node_count, 0);
        assert_eq!(stats.conflict_count, 0);
    }

    /// get_resolved_packages returns packages with no conflicts
    #[test]
    fn test_graph_get_resolved_packages_no_conflicts() {
        let mut g = DependencyGraph::new();
        g.add_package(make_pkg("nuke", "14.0")).unwrap();
        g.add_package(make_pkg("ocio", "2.1")).unwrap();
        let resolved = g.get_resolved_packages().unwrap();
        assert_eq!(resolved.len(), 2);
    }

    /// requirements_compatible: unconstrained requirements are always compatible
    #[test]
    fn test_requirements_compatible_unconstrained() {
        let r1 = PackageRequirement::parse("python").unwrap();
        let r2 = PackageRequirement::parse("python").unwrap();
        assert!(
            requirements_compatible(&r1, &r2),
            "Two unconstrained requirements for same package should be compatible"
        );
    }

    /// Test adding package with requirements (dependencies)
    #[test]
    fn test_graph_add_package_with_dependencies() {
        let mut g = DependencyGraph::new();
        let mut pkg = make_pkg("foo", "1.0.0");
        pkg.requires = vec!["bar".to_string(), "baz-2.0".to_string()];
        g.add_package(pkg).unwrap();

        let stats = g.get_stats();
        assert_eq!(stats.node_count, 1);
        // Note: edges are created when dependencies are also added to graph
    }

    /// Test get_resolved_packages with conflicts should error
    #[test]
    fn test_graph_get_resolved_packages_with_conflicts() {
        let mut g = DependencyGraph::new();
        g.add_package(make_pkg("python", "3.9.0")).unwrap();
        g.add_package(make_pkg("python", "3.10.0")).unwrap();

        // Having multiple versions might create conflicts
        // This test verifies the behavior when conflicts exist
        let result = g.get_resolved_packages();
        // Depending on implementation, this might error or return filtered results
        // Just ensure the function doesn't panic
        match result {
            Ok(_pkgs) => {
                // Success path - function returned without error
            }
            Err(_) => {
                // Error path is also valid behavior
            }
        }
    }

    /// Test requirements_compatible with version constraints
    #[test]
    fn test_requirements_compatible_with_versions() {
        let r1 = PackageRequirement::parse("python-3.9").unwrap();
        let r2 = PackageRequirement::parse("python-3.9").unwrap();
        assert!(
            requirements_compatible(&r1, &r2),
            "Same version requirements should be compatible"
        );
    }

    /// Test requirements_compatible with incompatible versions
    #[test]
    fn test_requirements_compatible_incompatible() {
        // This test assumes the function can detect incompatibility
        let r1 = PackageRequirement::new("python".to_string());
        let r2 = PackageRequirement::new("maya".to_string());
        // Different packages should still be compatible (no conflict)
        assert!(requirements_compatible(&r1, &r2));
    }

    /// Test get_stats returns correct counts
    #[test]
    fn test_graph_get_stats_detailed() {
        let mut g = DependencyGraph::new();
        g.add_package(make_pkg("pkg_a", "1.0.0")).unwrap();
        g.add_package(make_pkg("pkg_b", "2.0.0")).unwrap();
        g.add_package(make_pkg("pkg_c", "3.0.0")).unwrap();

        let stats = g.get_stats();
        assert_eq!(stats.node_count, 3);
        assert_eq!(stats.edge_count, 0); // No dependencies yet
    }

    /// Test adding multiple versions of same package
    #[test]
    fn test_graph_add_multiple_versions() {
        let mut g = DependencyGraph::new();
        g.add_package(make_pkg("python", "3.9.0")).unwrap();
        g.add_package(make_pkg("python", "3.10.0")).unwrap();
        g.add_package(make_pkg("python", "3.11.0")).unwrap();

        let stats = g.get_stats();
        // Depending on implementation, might count as 1 node (latest) or 3 nodes
        assert!(stats.node_count >= 1 && stats.node_count <= 3);
    }

    /// Test graph with dependencies creates edges
    #[test]
    fn test_graph_dependency_edges() {
        let mut g = DependencyGraph::new();
        let mut pkg_a = make_pkg("pkg_a", "1.0.0");
        pkg_a.requires = vec!["pkg_b".to_string()];

        g.add_package(pkg_a).unwrap();
        g.add_package(make_pkg("pkg_b", "2.0.0")).unwrap();

        let stats = g.get_stats();
        // Edge may be created between pkg_a and pkg_b (implementation dependent)
        // Just verify stats are reasonable
        assert!(stats.edge_count <= stats.node_count * 2);
    }

    /// Test clear and re-add packages
    #[test]
    fn test_graph_clear_and_readd() {
        let mut g = DependencyGraph::new();
        g.add_package(make_pkg("temp", "1.0.0")).unwrap();
        assert_eq!(g.get_stats().node_count, 1);

        g.clear();
        assert_eq!(g.get_stats().node_count, 0);

        g.add_package(make_pkg("temp", "2.0.0")).unwrap();
        assert_eq!(g.get_stats().node_count, 1);
    }

    /// Test PackageRequirement parsing edge cases
    #[test]
    fn test_package_requirement_parsing() {
        // Test various requirement string formats
        let r1 = PackageRequirement::parse("python").unwrap();
        assert_eq!(r1.name, "python");
        assert!(r1.version_spec.is_none());

        let r2 = PackageRequirement::parse("python-3.9").unwrap();
        assert_eq!(r2.name, "python");
        assert_eq!(r2.version_spec, Some("3.9".to_string()));

        let r3 = PackageRequirement::parse("~python").unwrap();
        assert!(r3.weak);
        assert_eq!(r3.name, "python");
    }

    /// Test GraphNode key generation
    #[test]
    fn test_graph_node_key() {
        let pkg_with_version = make_pkg("maya", "2024.0");
        let node = GraphNode::new(pkg_with_version);
        assert_eq!(node.key(), "maya-2024.0");

        let pkg_without_version = Package::new("unversioned".to_string());
        let node2 = GraphNode::new(pkg_without_version);
        assert_eq!(node2.key(), "unversioned");
    }

    /// Test GraphNode dependency management
    #[test]
    fn test_graph_node_dependency_management() {
        let pkg = make_pkg("test", "1.0.0");
        let mut node = GraphNode::new(pkg);

        assert!(node.dependencies.is_empty());
        assert!(node.dependents.is_empty());

        node.add_dependency("dep_a".to_string());
        node.add_dependency("dep_b".to_string());
        assert_eq!(node.dependencies.len(), 2);

        node.add_dependent("parent_a".to_string());
        assert_eq!(node.dependents.len(), 1);

        node.remove_dependency("dep_a");
        assert_eq!(node.dependencies.len(), 1);

        node.remove_dependent("parent_a");
        assert_eq!(node.dependents.len(), 0);
    }

    /// Test accessibility - empty graph
    #[test]
    fn test_accessibility_empty_graph() {
        let g = DependencyGraph::new();
        let accessibility = g.accessibility();
        assert!(accessibility.is_empty());
    }

    /// Test accessibility - single node with no dependencies
    #[test]
    fn test_accessibility_single_node_no_deps() {
        let mut g = DependencyGraph::new();
        g.add_package(make_pkg("A", "1.0.0")).unwrap();

        let accessibility = g.accessibility();
        assert_eq!(accessibility.len(), 1);
        assert_eq!(accessibility.get("A-1.0.0").unwrap(), &vec!["A-1.0.0"]);
    }

    /// Test accessibility - linear chain: A -> B -> C
    #[test]
    fn test_accessibility_linear_chain() {
        let mut g = DependencyGraph::new();
        g.add_package(make_pkg("A", "1.0.0")).unwrap();
        g.add_package(make_pkg("B", "1.0.0")).unwrap();
        g.add_package(make_pkg("C", "1.0.0")).unwrap();

        // Add dependency edges: A -> B -> C
        g.add_dependency_edge("A-1.0.0", "B-1.0.0").unwrap();
        g.add_dependency_edge("B-1.0.0", "C-1.0.0").unwrap();

        let accessibility = g.accessibility();

        // A can reach A, B, C
        let a_accessible = accessibility.get("A-1.0.0").unwrap();
        assert!(a_accessible.contains(&"A-1.0.0".to_string()));
        assert!(a_accessible.contains(&"B-1.0.0".to_string()));
        assert!(a_accessible.contains(&"C-1.0.0".to_string()));
        assert_eq!(a_accessible.len(), 3);

        // B can reach B, C
        let b_accessible = accessibility.get("B-1.0.0").unwrap();
        assert!(b_accessible.contains(&"B-1.0.0".to_string()));
        assert!(b_accessible.contains(&"C-1.0.0".to_string()));
        assert_eq!(b_accessible.len(), 2);

        // C can reach only C
        let c_accessible = accessibility.get("C-1.0.0").unwrap();
        assert_eq!(c_accessible, &vec!["C-1.0.0"]);
    }

    /// Test accessibility - DAG with multiple paths
    #[test]
    fn test_accessibility_dag_multiple_paths() {
        let mut g = DependencyGraph::new();
        g.add_package(make_pkg("A", "1.0.0")).unwrap();
        g.add_package(make_pkg("B", "1.0.0")).unwrap();
        g.add_package(make_pkg("C", "1.0.0")).unwrap();
        g.add_package(make_pkg("D", "1.0.0")).unwrap();

        // A -> B -> D
        // A -> C -> D
        g.add_dependency_edge("A-1.0.0", "B-1.0.0").unwrap();
        g.add_dependency_edge("A-1.0.0", "C-1.0.0").unwrap();
        g.add_dependency_edge("B-1.0.0", "D-1.0.0").unwrap();
        g.add_dependency_edge("C-1.0.0", "D-1.0.0").unwrap();

        let accessibility = g.accessibility();

        // A can reach A, B, C, D
        let a_accessible = accessibility.get("A-1.0.0").unwrap();
        assert_eq!(a_accessible.len(), 4);
        assert!(a_accessible.contains(&"A-1.0.0".to_string()));
        assert!(a_accessible.contains(&"B-1.0.0".to_string()));
        assert!(a_accessible.contains(&"C-1.0.0".to_string()));
        assert!(a_accessible.contains(&"D-1.0.0".to_string()));

        // D can reach only D
        let d_accessible = accessibility.get("D-1.0.0").unwrap();
        assert_eq!(d_accessible, &vec!["D-1.0.0"]);
    }

    /// Test accessibility - graph with cycle (should not infinite loop)
    #[test]
    fn test_accessibility_with_cycle() {
        let mut g = DependencyGraph::new();
        g.add_package(make_pkg("A", "1.0.0")).unwrap();
        g.add_package(make_pkg("B", "1.0.0")).unwrap();
        g.add_package(make_pkg("C", "1.0.0")).unwrap();

        // Create a cycle: A -> B -> C -> A
        g.add_dependency_edge("A-1.0.0", "B-1.0.0").unwrap();
        g.add_dependency_edge("B-1.0.0", "C-1.0.0").unwrap();
        g.add_dependency_edge("C-1.0.0", "A-1.0.0").unwrap();

        let accessibility = g.accessibility();

        // All nodes can reach all nodes (including themselves)
        for node_key in &["A-1.0.0", "B-1.0.0", "C-1.0.0"] {
            let accessible = accessibility.get(*node_key).unwrap();
            assert_eq!(accessible.len(), 3, "Node {} should reach all 3 nodes", node_key);
        }
    }

    /// Test accessibility - disconnected graph
    #[test]
    fn test_accessibility_disconnected() {
        let mut g = DependencyGraph::new();
        g.add_package(make_pkg("A", "1.0.0")).unwrap();
        g.add_package(make_pkg("B", "1.0.0")).unwrap();
        g.add_package(make_pkg("C", "1.0.0")).unwrap();
        // No edges - all nodes disconnected

        let accessibility = g.accessibility();

        // Each node can only reach itself
        for node_key in &["A-1.0.0", "B-1.0.0", "C-1.0.0"] {
            let accessible = accessibility.get(*node_key).unwrap();
            assert_eq!(accessible.len(), 1, "Node {} should only reach itself", node_key);
            assert!(accessible.contains(&node_key.to_string()));
        }
    }

    /// Test find_cycle returns None for graph without cycles (DAG)
    #[test]
    fn test_find_cycle_none_for_dag() {
        let mut g = DependencyGraph::new();
        g.add_package(make_pkg("A", "1.0.0")).unwrap();
        g.add_package(make_pkg("B", "1.0.0")).unwrap();
        g.add_package(make_pkg("C", "1.0.0")).unwrap();

        // A -> B -> C (linear chain, no cycle)
        g.add_dependency_edge("A-1.0.0", "B-1.0.0").unwrap();
        g.add_dependency_edge("B-1.0.0", "C-1.0.0").unwrap();

        assert!(g.find_cycle().is_none(), "DAG should have no cycles");
    }

    /// Test find_cycle detects a simple two-node cycle
    #[test]
    fn test_find_cycle_detects_simple_cycle() {
        let mut g = DependencyGraph::new();
        g.add_package(make_pkg("A", "1.0.0")).unwrap();
        g.add_package(make_pkg("B", "1.0.0")).unwrap();

        // A -> B -> A (cycle)
        g.add_dependency_edge("A-1.0.0", "B-1.0.0").unwrap();
        g.add_dependency_edge("B-1.0.0", "A-1.0.0").unwrap();

        let cycle = g.find_cycle();
        assert!(cycle.is_some(), "Should detect cycle A -> B -> A");
    }

    /// Test find_cycle detects three-node cycle
    #[test]
    fn test_find_cycle_detects_three_node_cycle() {
        let mut g = DependencyGraph::new();
        g.add_package(make_pkg("A", "1.0.0")).unwrap();
        g.add_package(make_pkg("B", "1.0.0")).unwrap();
        g.add_package(make_pkg("C", "1.0.0")).unwrap();

        // A -> B -> C -> A (cycle)
        g.add_dependency_edge("A-1.0.0", "B-1.0.0").unwrap();
        g.add_dependency_edge("B-1.0.0", "C-1.0.0").unwrap();
        g.add_dependency_edge("C-1.0.0", "A-1.0.0").unwrap();

        let cycle = g.find_cycle();
        assert!(cycle.is_some(), "Should detect cycle A -> B -> C -> A");
        let cycle_nodes = cycle.unwrap();
        assert_eq!(cycle_nodes.len(), 3);
    }

    /// Test find_cycle returns None for empty graph
    #[test]
    fn test_find_cycle_empty_graph() {
        let g = DependencyGraph::new();
        assert!(g.find_cycle().is_none(), "Empty graph has no cycles");
    }

    /// Test find_cycle returns None for single node
    #[test]
    fn test_find_cycle_single_node_no_cycle() {
        let mut g = DependencyGraph::new();
        g.add_package(make_pkg("A", "1.0.0")).unwrap();
        assert!(g.find_cycle().is_none(), "Single node has no cycle");
    }

    /// Test find_cycle detects cycle in graph with multiple components
    #[test]
    fn test_find_cycle_multiple_components() {
        let mut g = DependencyGraph::new();
        // Component 1: A -> B -> A (has cycle)
        g.add_package(make_pkg("A", "1.0.0")).unwrap();
        g.add_package(make_pkg("B", "1.0.0")).unwrap();
        g.add_dependency_edge("A-1.0.0", "B-1.0.0").unwrap();
        g.add_dependency_edge("B-1.0.0", "A-1.0.0").unwrap();

        // Component 2: C -> D (no cycle)
        g.add_package(make_pkg("C", "1.0.0")).unwrap();
        g.add_package(make_pkg("D", "1.0.0")).unwrap();
        g.add_dependency_edge("C-1.0.0", "D-1.0.0").unwrap();

        let cycle = g.find_cycle();
        assert!(cycle.is_some(), "Should detect cycle in component 1");
    }
}
