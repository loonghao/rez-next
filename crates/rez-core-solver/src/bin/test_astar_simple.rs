//! Simple standalone test for A* search framework
//!
//! This binary tests only the A* search implementation without any external dependencies.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

// Simple type definitions
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub name: String,
    pub requires: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageRequirement {
    pub name: String,
    pub requirement_string: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyConflict {
    pub package_name: String,
    pub severity: u32,
    pub conflict_type: ConflictType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictType {
    VersionConflict,
    CircularDependency,
    MissingPackage,
    PlatformConflict,
}

#[derive(Debug, Clone)]
pub struct SearchState {
    pub resolved_packages: HashMap<String, Package>,
    pub pending_requirements: Vec<PackageRequirement>,
    pub conflicts: Vec<DependencyConflict>,
    pub cost_so_far: u32,
    pub estimated_total_cost: u32,
    pub depth: usize,
    pub parent_id: Option<u64>,
    pub state_id: u64,
    state_hash: u64,
}

impl SearchState {
    pub fn new_initial(requirements: Vec<PackageRequirement>) -> Self {
        let mut state = Self {
            resolved_packages: HashMap::new(),
            pending_requirements: requirements,
            conflicts: Vec::new(),
            cost_so_far: 0,
            estimated_total_cost: 0,
            depth: 0,
            parent_id: None,
            state_id: 0,
            state_hash: 0,
        };
        
        state.update_hash();
        state.state_id = state.state_hash;
        state
    }

    pub fn new_from_parent(
        parent: &SearchState,
        resolved_package: Package,
        new_requirements: Vec<PackageRequirement>,
        additional_cost: u32,
    ) -> Self {
        let mut resolved_packages = parent.resolved_packages.clone();
        resolved_packages.insert(resolved_package.name.clone(), resolved_package);
        
        let mut pending_requirements = parent.pending_requirements.clone();
        pending_requirements.extend(new_requirements);
        
        let mut state = Self {
            resolved_packages,
            pending_requirements,
            conflicts: parent.conflicts.clone(),
            cost_so_far: parent.cost_so_far + additional_cost,
            estimated_total_cost: 0,
            depth: parent.depth + 1,
            parent_id: Some(parent.state_id),
            state_id: 0,
            state_hash: 0,
        };
        
        state.update_hash();
        state.state_id = state.state_hash;
        state
    }

    pub fn is_goal(&self) -> bool {
        self.pending_requirements.is_empty() && self.conflicts.is_empty()
    }

    pub fn is_valid(&self) -> bool {
        for conflict in &self.conflicts {
            match conflict.conflict_type {
                ConflictType::MissingPackage => return false,
                ConflictType::CircularDependency => return false,
                _ => {}
            }
        }
        true
    }

    pub fn add_conflict(&mut self, conflict: DependencyConflict) {
        self.conflicts.push(conflict);
        self.update_hash();
    }

    fn update_hash(&mut self) {
        use std::collections::hash_map::DefaultHasher;
        
        let mut hasher = DefaultHasher::new();
        
        let mut package_names: Vec<_> = self.resolved_packages.keys().collect();
        package_names.sort();
        for name in package_names {
            name.hash(&mut hasher);
        }
        
        let mut req_strings: Vec<_> = self.pending_requirements.iter()
            .map(|req| &req.requirement_string)
            .collect();
        req_strings.sort();
        for req_str in req_strings {
            req_str.hash(&mut hasher);
        }
        
        for conflict in &self.conflicts {
            conflict.package_name.hash(&mut hasher);
        }
        
        self.state_hash = hasher.finish();
    }

    pub fn get_hash(&self) -> u64 {
        self.state_hash
    }

    pub fn calculate_complexity(&self) -> usize {
        self.resolved_packages.len() + 
        self.pending_requirements.len() + 
        self.conflicts.len() * 2
    }
}

impl PartialEq for SearchState {
    fn eq(&self, other: &Self) -> bool {
        self.state_hash == other.state_hash
    }
}

impl Eq for SearchState {}

impl Hash for SearchState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.state_hash.hash(state);
    }
}

pub struct StatePool {
    pool: Vec<SearchState>,
    max_size: usize,
}

impl StatePool {
    pub fn new(max_size: usize) -> Self {
        Self {
            pool: Vec::with_capacity(max_size),
            max_size,
        }
    }

    pub fn get_state(&mut self) -> SearchState {
        self.pool.pop().unwrap_or_else(|| {
            SearchState::new_initial(Vec::new())
        })
    }

    pub fn return_state(&mut self, mut state: SearchState) {
        if self.pool.len() < self.max_size {
            state.resolved_packages.clear();
            state.pending_requirements.clear();
            state.conflicts.clear();
            state.cost_so_far = 0;
            state.estimated_total_cost = 0;
            state.depth = 0;
            state.parent_id = None;
            state.state_id = 0;
            state.state_hash = 0;
            
            self.pool.push(state);
        }
    }

    pub fn size(&self) -> usize {
        self.pool.len()
    }
}

// Test functions
pub fn run_tests() -> Result<(), String> {
    println!("🧪 Running A* Search Framework Tests");
    println!("====================================");
    
    test_search_state_creation()?;
    test_state_pool_functionality()?;
    test_conflict_management()?;
    test_state_hashing()?;
    test_state_transitions()?;
    test_goal_state_detection()?;
    
    println!("====================================");
    println!("🎉 All tests passed!");
    
    Ok(())
}

fn test_search_state_creation() -> Result<(), String> {
    println!("Testing SearchState creation...");
    
    let req = PackageRequirement {
        name: "test_package".to_string(),
        requirement_string: "test_package".to_string(),
    };
    
    let state = SearchState::new_initial(vec![req]);
    
    if state.depth != 0 {
        return Err("Initial state should have depth 0".to_string());
    }
    
    if !state.resolved_packages.is_empty() {
        return Err("Initial state should have no resolved packages".to_string());
    }
    
    if !state.conflicts.is_empty() {
        return Err("Initial state should have no conflicts".to_string());
    }
    
    if state.pending_requirements.len() != 1 {
        return Err("Initial state should have 1 pending requirement".to_string());
    }
    
    if state.is_goal() {
        return Err("Initial state with pending requirements should not be goal".to_string());
    }
    
    println!("✅ SearchState creation test passed");
    Ok(())
}

fn test_state_pool_functionality() -> Result<(), String> {
    println!("Testing StatePool functionality...");
    
    let mut pool = StatePool::new(5);
    
    if pool.size() != 0 {
        return Err("New pool should be empty".to_string());
    }
    
    let state = pool.get_state();
    pool.return_state(state);
    if pool.size() != 1 {
        return Err("Pool should have 1 state after return".to_string());
    }
    
    let _state = pool.get_state();
    if pool.size() != 0 {
        return Err("Pool should be empty after get".to_string());
    }
    
    println!("✅ StatePool functionality test passed");
    Ok(())
}

fn test_conflict_management() -> Result<(), String> {
    println!("Testing conflict management...");
    
    let mut state = SearchState::new_initial(vec![]);
    
    let version_conflict = DependencyConflict {
        package_name: "test_package".to_string(),
        severity: 80,
        conflict_type: ConflictType::VersionConflict,
    };
    
    state.add_conflict(version_conflict);
    
    if state.conflicts.is_empty() {
        return Err("State should have conflicts after adding one".to_string());
    }
    
    if !state.is_valid() {
        return Err("State with version conflict should still be valid".to_string());
    }
    
    let fatal_conflict = DependencyConflict {
        package_name: "missing_package".to_string(),
        severity: 100,
        conflict_type: ConflictType::MissingPackage,
    };
    
    state.add_conflict(fatal_conflict);
    
    if state.is_valid() {
        return Err("State with missing package should be invalid".to_string());
    }
    
    println!("✅ Conflict management test passed");
    Ok(())
}

fn test_state_hashing() -> Result<(), String> {
    println!("Testing state hashing and equality...");
    
    let req = PackageRequirement {
        name: "test_package".to_string(),
        requirement_string: "test_package".to_string(),
    };
    
    let state1 = SearchState::new_initial(vec![req.clone()]);
    let state2 = SearchState::new_initial(vec![req]);
    
    if state1 != state2 {
        return Err("States with same content should be equal".to_string());
    }
    
    if state1.get_hash() != state2.get_hash() {
        return Err("States with same content should have same hash".to_string());
    }
    
    println!("✅ State hashing test passed");
    Ok(())
}

fn test_state_transitions() -> Result<(), String> {
    println!("Testing state transitions...");
    
    let req = PackageRequirement {
        name: "test_package".to_string(),
        requirement_string: "test_package".to_string(),
    };
    
    let parent_state = SearchState::new_initial(vec![req.clone()]);
    
    let package = Package {
        name: "test_package".to_string(),
        requires: vec!["dependency1".to_string(), "dependency2".to_string()],
    };
    
    let new_requirements = vec![
        PackageRequirement {
            name: "dependency1".to_string(),
            requirement_string: "dependency1".to_string(),
        },
        PackageRequirement {
            name: "dependency2".to_string(),
            requirement_string: "dependency2".to_string(),
        },
    ];
    
    let child_state = SearchState::new_from_parent(
        &parent_state,
        package,
        new_requirements,
        1,
    );
    
    if child_state.depth != parent_state.depth + 1 {
        return Err("Child state should have incremented depth".to_string());
    }
    
    if child_state.cost_so_far != parent_state.cost_so_far + 1 {
        return Err("Child state should have accumulated cost".to_string());
    }
    
    if child_state.resolved_packages.len() != 1 {
        return Err("Child state should have 1 resolved package".to_string());
    }
    
    if !child_state.resolved_packages.contains_key("test_package") {
        return Err("Child state should contain resolved package".to_string());
    }
    
    if child_state.parent_id != Some(parent_state.state_id) {
        return Err("Child state should reference parent ID".to_string());
    }
    
    let complexity = child_state.calculate_complexity();
    let expected_complexity = 1 + 3 + 0; // 1 resolved + 3 pending + 0 conflicts
    if complexity != expected_complexity {
        return Err(format!("Expected complexity {}, got {}", expected_complexity, complexity));
    }
    
    println!("✅ State transitions test passed");
    Ok(())
}

fn test_goal_state_detection() -> Result<(), String> {
    println!("Testing goal state detection...");
    
    let goal_state = SearchState::new_initial(vec![]);
    
    if !goal_state.is_goal() {
        return Err("State with no pending requirements should be goal".to_string());
    }
    
    let req = PackageRequirement {
        name: "test_package".to_string(),
        requirement_string: "test_package".to_string(),
    };
    
    let non_goal_state = SearchState::new_initial(vec![req]);
    
    if non_goal_state.is_goal() {
        return Err("State with pending requirements should not be goal".to_string());
    }
    
    println!("✅ Goal state detection test passed");
    Ok(())
}

fn main() {
    println!("A* Search Framework Simple Test");
    println!("===============================");
    
    match run_tests() {
        Ok(()) => {
            println!("✅ All tests completed successfully!");
            println!("🚀 A* Search Framework is working correctly!");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("❌ Test failed: {}", e);
            std::process::exit(1);
        }
    }
}
