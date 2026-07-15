//! Internal resolution state used by DependencyResolver during package resolution.

use rez_next_package::Requirement;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::dependency_resolver::{ResolutionConflict, ResolvedPackageInfo};
use rez_next_package::Package;
use rez_next_version::Version;
use std::sync::Arc;

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

    /// Package requirements that must not be present.
    pub(crate) conflict_requirements: Vec<Requirement>,

    /// Every positive constraint discovered for each package name.
    pub(crate) active_requirements: HashMap<String, Vec<Requirement>>,

    /// Requirements contributed by each currently selected package.
    pub(crate) package_requirements: HashMap<String, Vec<Requirement>>,

    /// Conflict requirements contributed by each currently selected package.
    pub(crate) package_conflict_requirements: HashMap<String, Vec<Requirement>>,

    /// Satisfied requirements (to avoid duplicates)
    pub(crate) satisfied_requirements: HashSet<String>,

    /// Package versions rejected while backtracking this solve.
    pub(crate) rejected_versions: HashMap<String, HashSet<Version>>,

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
            if !req.conflict {
                queue.push_back(req.clone());
            }
        }

        let mut state = Self {
            original_requirements: requirements,
            requirement_queue: queue,
            resolved_packages: Vec::new(),
            failed_requirements: Vec::new(),
            conflicts: Vec::new(),
            conflict_requirements: Vec::new(),
            active_requirements: HashMap::new(),
            package_requirements: HashMap::new(),
            package_conflict_requirements: HashMap::new(),
            satisfied_requirements: HashSet::new(),
            rejected_versions: HashMap::new(),
            dep_graph: HashMap::new(),
            packages_considered: 0,
            variants_evaluated: 0,
            backtrack_steps: 0,
        };
        state.rebuild_constraints();
        state
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
            if !visited.contains(node)
                && let Some(cycle) = self.dfs_cycle(node, &mut visited, &mut on_stack, &mut path)
            {
                return Some(cycle);
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
        while let Some(requirement) = self.requirement_queue.pop_front() {
            if self.is_requirement_active(&requirement) {
                return Some(requirement);
            }
        }
        None
    }

    /// Replace the dependency contribution of a selected package.
    ///
    /// A package version replacement must also replace its dependency constraints;
    /// otherwise requirements from the old version remain active and create false
    /// conflicts later in the solve.
    pub(crate) fn set_package_requirements(
        &mut self,
        package_name: String,
        requirements: Vec<Requirement>,
        conflicts: Vec<Requirement>,
    ) {
        self.package_requirements
            .insert(package_name.clone(), requirements);
        self.package_conflict_requirements
            .insert(package_name, conflicts);
        self.prune_orphaned_packages();
        self.rebuild_constraints();

        self.requeue_active_requirements();
        self.satisfied_requirements.clear();
        let failed_requirements = std::mem::take(&mut self.failed_requirements);
        self.failed_requirements = failed_requirements
            .into_iter()
            .filter(|requirement| self.is_requirement_active(requirement))
            .collect();
    }

    pub(crate) fn is_requirement_active(&self, requirement: &Requirement) -> bool {
        self.active_requirements
            .get(&requirement.name)
            .is_some_and(|requirements| requirements.contains(requirement))
    }

    pub(crate) fn reject_package(&mut self, package: &Package) {
        if let Some(version) = &package.version {
            self.rejected_versions
                .entry(package.name.clone())
                .or_default()
                .insert(version.clone());
        }
    }

    pub(crate) fn is_package_rejected(&self, package: &Package) -> bool {
        package.version.as_ref().is_some_and(|version| {
            self.rejected_versions
                .get(&package.name)
                .is_some_and(|versions| versions.contains(version))
        })
    }

    fn rebuild_constraints(&mut self) {
        self.active_requirements.clear();
        self.conflict_requirements.clear();
        self.dep_graph.clear();

        let original_requirements = self.original_requirements.clone();
        for requirement in original_requirements {
            if requirement.conflict {
                self.conflict_requirements.push(requirement);
            } else {
                self.insert_active_requirement(requirement);
            }
        }

        let package_requirements = self.package_requirements.clone();
        for (source, requirements) in package_requirements {
            for requirement in requirements {
                self.record_dependency(&source, &requirement.name);
                self.insert_active_requirement(requirement);
            }
        }

        for conflict in self.package_conflict_requirements.values().flatten() {
            if !self.conflict_requirements.contains(conflict) {
                self.conflict_requirements.push(conflict.clone());
            }
        }
    }

    fn insert_active_requirement(&mut self, requirement: Requirement) {
        let requirements = self
            .active_requirements
            .entry(requirement.name.clone())
            .or_default();
        if !requirements.contains(&requirement) {
            requirements.push(requirement);
        }
    }

    fn requeue_active_requirements(&mut self) {
        let mut queue = VecDeque::new();
        for requirement in &self.original_requirements {
            if !requirement.conflict {
                queue.push_back(requirement.clone());
            }
        }
        for resolved in &self.resolved_packages {
            if let Some(requirements) = self.package_requirements.get(&resolved.package.name) {
                for requirement in requirements {
                    queue.push_back(requirement.clone());
                }
            }
        }
        self.requirement_queue = queue;
    }

    fn prune_orphaned_packages(&mut self) {
        let mut reachable: HashSet<String> = self
            .original_requirements
            .iter()
            .filter(|requirement| !requirement.weak && !requirement.conflict)
            .map(|requirement| requirement.name.clone())
            .collect();
        let mut pending: Vec<String> = reachable.iter().cloned().collect();

        while let Some(package_name) = pending.pop() {
            if let Some(requirements) = self.package_requirements.get(&package_name) {
                for requirement in requirements.iter().filter(|requirement| !requirement.weak) {
                    if reachable.insert(requirement.name.clone()) {
                        pending.push(requirement.name.clone());
                    }
                }
            }
        }

        self.resolved_packages
            .retain(|resolved| reachable.contains(&resolved.package.name));
        self.package_requirements
            .retain(|package_name, _| reachable.contains(package_name));
        self.package_conflict_requirements
            .retain(|package_name, _| reachable.contains(package_name));
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
                    .is_none_or(|v| requirement.is_satisfied_by(v))
        })
    }

    pub(crate) fn check_conflicts(
        &self,
        candidate: &Arc<Package>,
        _requirement: &Requirement,
    ) -> Option<ResolutionConflict> {
        if let Some(conflict) = self.check_explicit_conflicts(candidate) {
            return Some(conflict);
        }

        if let Some(version) = &candidate.version
            && let Some(requirements) = self.active_requirements.get(&candidate.name)
            && let Some(unsatisfied) = requirements
                .iter()
                .find(|requirement| !requirement.is_satisfied_by(version))
        {
            return Some(ResolutionConflict {
                package_name: candidate.name.clone(),
                conflicting_requirements: vec![unsatisfied.clone()],
                source_packages: Vec::new(),
            });
        }

        None
    }

    pub(crate) fn check_explicit_conflicts(
        &self,
        candidate: &Arc<Package>,
    ) -> Option<ResolutionConflict> {
        self.conflict_requirements
            .iter()
            .find(|conflict| {
                conflict.name == candidate.name
                    && candidate
                        .version
                        .as_ref()
                        .is_none_or(|version| conflict.is_satisfied_by(version))
            })
            .map(|conflict| ResolutionConflict {
                package_name: candidate.name.clone(),
                conflicting_requirements: vec![conflict.clone()],
                source_packages: Vec::new(),
            })
    }

    pub(crate) fn add_resolved_package(&mut self, package: ResolvedPackageInfo) {
        if let Some(existing) = self
            .resolved_packages
            .iter_mut()
            .find(|existing| existing.package.name == package.package.name)
        {
            *existing = package;
        } else {
            self.resolved_packages.push(package);
        }
    }

    pub(crate) fn is_original_requirement(&self, requirement: &Requirement) -> bool {
        self.original_requirements
            .iter()
            .any(|orig| orig.name == requirement.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rez_next_package::Requirement;

    #[test]
    fn test_resolution_state_new() {
        let reqs = vec![
            Requirement::new("python".to_string()),
            Requirement::new("maya".to_string()),
        ];
        let state = ResolutionState::new(reqs.clone());
        assert_eq!(state.original_requirements.len(), 2);
        assert_eq!(state.requirement_queue.len(), 2);
    }

    #[test]
    fn test_resolution_state_detect_cycle_none() {
        let reqs = vec![];
        let state = ResolutionState::new(reqs);
        // No dependencies, no cycle
        assert!(state.detect_cycle().is_none());
    }

    #[test]
    fn test_resolution_state_detect_cycle_simple() {
        let reqs = vec![];
        let mut state = ResolutionState::new(reqs);
        // Create a cycle: A -> B -> C -> A
        state.record_dependency("A", "B");
        state.record_dependency("B", "C");
        state.record_dependency("C", "A");
        let cycle = state.detect_cycle();
        assert!(cycle.is_some());
        let cycle_path = cycle.unwrap();
        assert!(cycle_path.contains(&"A".to_string()));
        assert!(cycle_path.contains(&"B".to_string()));
        assert!(cycle_path.contains(&"C".to_string()));
    }

    #[test]
    fn test_resolution_state_detect_cycle_no_cycle() {
        let reqs = vec![];
        let mut state = ResolutionState::new(reqs);
        // No cycle: A -> B -> C
        state.record_dependency("A", "B");
        state.record_dependency("B", "C");
        assert!(state.detect_cycle().is_none());
    }

    #[test]
    fn test_resolution_state_get_next_requirement() {
        let reqs = vec![
            Requirement::new("python".to_string()),
            Requirement::new("maya".to_string()),
        ];
        let mut state = ResolutionState::new(reqs);
        let next = state.get_next_requirement();
        assert!(next.is_some());
        assert_eq!(next.unwrap().name, "python");
        let next2 = state.get_next_requirement();
        assert!(next2.is_some());
        assert_eq!(next2.unwrap().name, "maya");
        let next3 = state.get_next_requirement();
        assert!(next3.is_none());
    }

    #[test]
    fn test_resolution_state_is_original_requirement() {
        let reqs = vec![Requirement::new("python".to_string())];
        let state = ResolutionState::new(reqs);
        let req = Requirement::new("python".to_string());
        assert!(state.is_original_requirement(&req));
        let req2 = Requirement::new("maya".to_string());
        assert!(!state.is_original_requirement(&req2));
    }
}
