//! Internal resolution state used by DependencyResolver during package resolution.

use rez_next_package::Requirement;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::dependency_resolver::{ResolvedPackageInfo, ResolutionConflict};
use std::sync::Arc;
use rez_next_package::Package;

/// Internal state for the resolution process
#[derive(Debug)]
pub(crate) struct ResolutionState {
    /// Original requirements to resolve
    pub(crate) original_requirements: Vec<Requirement>,

    /// Queue of requirements to process
    pub(crate) requirement_queue: VecDeque<Requirement>,

    /// Successfully resolved packages
    pub(crate) resolved_packages: Vec<ResolvedPackageInfo>,

    /// Requirements that couldn't be satisfied
    pub(crate) failed_requirements: Vec<Requirement>,

    /// Conflicts encountered
    pub(crate) conflicts: Vec<ResolutionConflict>,

    /// Satisfied requirements (to avoid duplicates)
    pub(crate) satisfied_requirements: HashSet<String>,

    /// Dependency graph edges: package_name -> list of its direct requirements (package names)
    pub(crate) dep_graph: HashMap<String, Vec<String>>,

    /// Statistics
    pub(crate) packages_considered: usize,
    pub(crate) variants_evaluated: usize,
    pub(crate) backtrack_steps: usize,
}

impl ResolutionState {
    pub(crate) fn new(requirements: Vec<Requirement>) -> Self {
        let mut queue = VecDeque::new();
        for req in &requirements {
            queue.push_back(req.clone());
        }

        Self {
            original_requirements: requirements,
            requirement_queue: queue,
            resolved_packages: Vec::new(),
            failed_requirements: Vec::new(),
            conflicts: Vec::new(),
            satisfied_requirements: HashSet::new(),
            dep_graph: HashMap::new(),
            packages_considered: 0,
            variants_evaluated: 0,
            backtrack_steps: 0,
        }
    }

    /// Record a dependency edge: `from_pkg` requires `to_pkg`
    pub(crate) fn record_dependency(&mut self, from_pkg: &str, to_pkg: &str) {
        self.dep_graph
            .entry(from_pkg.to_string())
            .or_default()
            .push(to_pkg.to_string());
    }

    /// Detect cycles in the dependency graph using DFS.
    /// Returns Some(cycle_path) if a cycle is detected, None otherwise.
    pub fn detect_cycle(&self) -> Option<Vec<String>> {
        let mut visited: HashSet<String> = HashSet::new();
        let mut path: Vec<String> = Vec::new();
        let mut on_stack: HashSet<String> = HashSet::new();

        for node in self.dep_graph.keys() {
            if !visited.contains(node) {
                if let Some(cycle) = self.dfs_cycle(node, &mut visited, &mut on_stack, &mut path) {
                    return Some(cycle);
                }
            }
        }
        None
    }

    fn dfs_cycle(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        on_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(node.to_string());
        on_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(neighbors) = self.dep_graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if let Some(cycle) = self.dfs_cycle(neighbor, visited, on_stack, path) {
                        return Some(cycle);
                    }
                } else if on_stack.contains(neighbor) {
                    // Found a back-edge: extract cycle
                    let cycle_start = path.iter().position(|n| n == neighbor).unwrap_or(0);
                    let mut cycle = path[cycle_start..].to_vec();
                    cycle.push(neighbor.clone()); // close the cycle
                    return Some(cycle);
                }
            }
        }

        on_stack.remove(node);
        path.pop();
        None
    }

    pub(crate) fn get_next_requirement(&mut self) -> Option<Requirement> {
        self.requirement_queue.pop_front()
    }

    pub(crate) fn add_requirement(&mut self, requirement: Requirement) {
        let req_key = format!(
            "{}:{}",
            requirement.name,
            requirement
                .version_constraint
                .as_ref()
                .map(|v| format!("{:?}", v))
                .unwrap_or_default()
        );
        if !self.satisfied_requirements.contains(&req_key) {
            self.requirement_queue.push_back(requirement);
        }
    }

    pub(crate) fn mark_requirement_satisfied(
        &mut self,
        requirement: &Requirement,
        _package_name: String,
    ) {
        let req_key = format!(
            "{}:{}",
            requirement.name,
            requirement
                .version_constraint
                .as_ref()
                .map(|v| format!("{:?}", v))
                .unwrap_or_default()
        );
        self.satisfied_requirements.insert(req_key);
    }

    pub(crate) fn find_satisfying_package(
        &self,
        requirement: &Requirement,
    ) -> Option<&ResolvedPackageInfo> {
        self.resolved_packages.iter().find(|pkg| {
            pkg.package.name == requirement.name
                && pkg
                    .package
                    .version
                    .as_ref()
                    .map_or(true, |v| requirement.is_satisfied_by(v))
        })
    }

    pub(crate) fn check_conflicts(
        &self,
        candidate: &Arc<Package>,
        requirement: &Requirement,
    ) -> Option<ResolutionConflict> {
        // Check for version conflicts with existing packages
        for existing in &self.resolved_packages {
            if existing.package.name == candidate.name {
                if let (Some(existing_version), Some(candidate_version)) =
                    (&existing.package.version, &candidate.version)
                {
                    if existing_version != candidate_version {
                        return Some(ResolutionConflict {
                            package_name: candidate.name.clone(),
                            conflicting_requirements: vec![requirement.clone()],
                            source_packages: vec![existing.package.name.clone()],
                        });
                    }
                }
            }
        }

        None
    }

    pub(crate) fn add_resolved_package(&mut self, package: ResolvedPackageInfo) {
        self.resolved_packages.push(package);
    }

    pub(crate) fn is_original_requirement(&self, requirement: &Requirement) -> bool {
        self.original_requirements
            .iter()
            .any(|orig| orig.name == requirement.name)
    }
}
