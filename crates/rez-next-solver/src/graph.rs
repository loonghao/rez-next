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
            .or_insert_with(Vec::new)
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
                // Check if requirements are compatible
                let mut incompatible_groups = Vec::new();

                for (i, req1) in requirements.iter().enumerate() {
                    for req2 in requirements.iter().skip(i + 1) {
                        // TODO: Implement is_compatible_with method for PackageRequirement
                        // if !req1.is_compatible_with(req2) {
                        //     incompatible_groups.push((req1.clone(), req2.clone()));
                        // }
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

        // TODO: Implement range checking when PackageRequirement has range field
        // for (req1, req2) in incompatible_groups {
        //     if let (Some(range1), Some(range2)) = (&req1.range, &req2.range) {
        //         if !range1.intersects(range2) {
        //             return ConflictSeverity::Incompatible;
        //         }
        //     }
        // }

        ConflictSeverity::Major // Default to major for now
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
                // TODO: Implement exact requirement creation when method is available
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
