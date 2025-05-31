//! Heuristic Functions for A* Dependency Resolution
//!
//! This module implements various heuristic functions to guide the A* search
//! algorithm towards optimal dependency resolution solutions efficiently.
//!
//! ## Heuristic Functions
//!
//! - **Remaining Requirements Heuristic**: Estimates cost based on unresolved requirements
//! - **Conflict Penalty Heuristic**: Adds penalty for existing conflicts
//! - **Dependency Depth Heuristic**: Considers the depth of dependency chains
//! - **Version Preference Heuristic**: Prefers certain version patterns
//! - **Composite Heuristic**: Combines multiple heuristics with weights

use super::search_state::{SearchState, DependencyConflict, ConflictType, Package, PackageRequirement};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Configuration for heuristic functions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeuristicConfig {
    /// Weight for remaining requirements heuristic
    pub remaining_requirements_weight: f64,
    
    /// Weight for conflict penalty heuristic
    pub conflict_penalty_weight: f64,
    
    /// Weight for dependency depth heuristic
    pub dependency_depth_weight: f64,
    
    /// Weight for version preference heuristic
    pub version_preference_weight: f64,
    
    /// Prefer latest versions
    pub prefer_latest_versions: bool,
    
    /// Penalty multiplier for conflicts
    pub conflict_penalty_multiplier: f64,
    
    /// Maximum estimated dependency depth
    pub max_estimated_depth: usize,
}

impl Default for HeuristicConfig {
    fn default() -> Self {
        Self {
            remaining_requirements_weight: 1.0,
            conflict_penalty_weight: 10.0,
            dependency_depth_weight: 0.5,
            version_preference_weight: 0.1,
            prefer_latest_versions: true,
            conflict_penalty_multiplier: 100.0,
            max_estimated_depth: 10,
        }
    }
}

/// Heuristic function trait for dependency resolution
pub trait DependencyHeuristic {
    /// Calculate heuristic value for a search state
    fn calculate(&self, state: &SearchState) -> f64;
    
    /// Get heuristic name for debugging
    fn name(&self) -> &'static str;
    
    /// Check if heuristic is admissible (never overestimates)
    fn is_admissible(&self) -> bool;
}

/// Remaining requirements heuristic
/// Estimates cost based on the number of unresolved requirements
pub struct RemainingRequirementsHeuristic {
    config: HeuristicConfig,
}

impl RemainingRequirementsHeuristic {
    pub fn new(config: HeuristicConfig) -> Self {
        Self { config }
    }
}

impl DependencyHeuristic for RemainingRequirementsHeuristic {
    fn calculate(&self, state: &SearchState) -> f64 {
        // Simple estimate: each remaining requirement costs at least 1 unit
        state.pending_requirements.len() as f64 * self.config.remaining_requirements_weight
    }
    
    fn name(&self) -> &'static str {
        "RemainingRequirements"
    }
    
    fn is_admissible(&self) -> bool {
        // This is admissible if each requirement costs at least the weight
        true
    }
}

/// Conflict penalty heuristic
/// Adds significant penalty for states with conflicts
pub struct ConflictPenaltyHeuristic {
    config: HeuristicConfig,
}

impl ConflictPenaltyHeuristic {
    pub fn new(config: HeuristicConfig) -> Self {
        Self { config }
    }
    
    fn calculate_conflict_penalty(&self, conflict: &DependencyConflict) -> f64 {
        let base_penalty = match conflict.conflict_type {
            ConflictType::VersionConflict => 50.0,
            ConflictType::CircularDependency => 1000.0, // Very high penalty
            ConflictType::MissingPackage => 500.0,
            ConflictType::PlatformConflict => 100.0,
        };
        
        base_penalty * conflict.severity * self.config.conflict_penalty_multiplier
    }
}

impl DependencyHeuristic for ConflictPenaltyHeuristic {
    fn calculate(&self, state: &SearchState) -> f64 {
        let mut penalty = 0.0;
        
        for conflict in &state.conflicts {
            penalty += self.calculate_conflict_penalty(conflict);
        }
        
        penalty * self.config.conflict_penalty_weight
    }
    
    fn name(&self) -> &'static str {
        "ConflictPenalty"
    }
    
    fn is_admissible(&self) -> bool {
        // This is not strictly admissible as it may overestimate
        // But it's useful for guiding search away from problematic states
        false
    }
}

/// Dependency depth heuristic
/// Estimates cost based on the expected depth of dependency chains
pub struct DependencyDepthHeuristic {
    config: HeuristicConfig,
    /// Cache of estimated dependency depths for packages
    depth_cache: HashMap<String, usize>,
}

impl DependencyDepthHeuristic {
    pub fn new(config: HeuristicConfig) -> Self {
        Self { 
            config,
            depth_cache: HashMap::new(),
        }
    }
    
    fn estimate_dependency_depth(&self, requirement: &PackageRequirement) -> usize {
        // Use cached value if available
        if let Some(&depth) = self.depth_cache.get(&requirement.name) {
            return depth;
        }
        
        // Estimate based on package name patterns
        let estimated_depth = if requirement.name.contains("core") || requirement.name.contains("base") {
            1 // Core packages typically have few dependencies
        } else if requirement.name.contains("plugin") || requirement.name.contains("extension") {
            3 // Plugins typically have moderate dependencies
        } else if requirement.name.contains("app") || requirement.name.contains("tool") {
            5 // Applications typically have many dependencies
        } else {
            2 // Default estimate
        };
        
        std::cmp::min(estimated_depth, self.config.max_estimated_depth)
    }
}

impl DependencyHeuristic for DependencyDepthHeuristic {
    fn calculate(&self, state: &SearchState) -> f64 {
        let mut total_depth_cost = 0.0;
        
        for requirement in &state.pending_requirements {
            let estimated_depth = self.estimate_dependency_depth(requirement);
            total_depth_cost += estimated_depth as f64;
        }
        
        total_depth_cost * self.config.dependency_depth_weight
    }
    
    fn name(&self) -> &'static str {
        "DependencyDepth"
    }
    
    fn is_admissible(&self) -> bool {
        // This is admissible if our depth estimates are conservative
        true
    }
}

/// Version preference heuristic
/// Guides search towards preferred version patterns
pub struct VersionPreferenceHeuristic {
    config: HeuristicConfig,
}

impl VersionPreferenceHeuristic {
    pub fn new(config: HeuristicConfig) -> Self {
        Self { config }
    }
    
    fn calculate_version_preference_cost(&self, _package: &Package) -> f64 {
        // TODO: Implement version preference logic when version system is available
        // For now, return minimal cost
        0.1
    }
}

impl DependencyHeuristic for VersionPreferenceHeuristic {
    fn calculate(&self, state: &SearchState) -> f64 {
        let mut preference_cost = 0.0;
        
        for package in state.resolved_packages.values() {
            preference_cost += self.calculate_version_preference_cost(package);
        }
        
        preference_cost * self.config.version_preference_weight
    }
    
    fn name(&self) -> &'static str {
        "VersionPreference"
    }
    
    fn is_admissible(&self) -> bool {
        true
    }
}

/// Composite heuristic that combines multiple heuristics
pub struct CompositeHeuristic {
    heuristics: Vec<Box<dyn DependencyHeuristic + Send + Sync>>,
    config: HeuristicConfig,
}

impl CompositeHeuristic {
    pub fn new(config: HeuristicConfig) -> Self {
        let mut heuristics: Vec<Box<dyn DependencyHeuristic + Send + Sync>> = Vec::new();
        
        // Add all heuristics
        heuristics.push(Box::new(RemainingRequirementsHeuristic::new(config.clone())));
        heuristics.push(Box::new(ConflictPenaltyHeuristic::new(config.clone())));
        heuristics.push(Box::new(DependencyDepthHeuristic::new(config.clone())));
        heuristics.push(Box::new(VersionPreferenceHeuristic::new(config.clone())));
        
        Self { heuristics, config }
    }
    
    /// Create a fast heuristic optimized for performance
    pub fn new_fast() -> Self {
        let config = HeuristicConfig {
            remaining_requirements_weight: 1.0,
            conflict_penalty_weight: 20.0,
            dependency_depth_weight: 0.2,
            version_preference_weight: 0.05,
            prefer_latest_versions: true,
            conflict_penalty_multiplier: 50.0,
            max_estimated_depth: 5,
        };
        
        Self::new(config)
    }
    
    /// Create a thorough heuristic optimized for solution quality
    pub fn new_thorough() -> Self {
        let config = HeuristicConfig {
            remaining_requirements_weight: 1.0,
            conflict_penalty_weight: 100.0,
            dependency_depth_weight: 1.0,
            version_preference_weight: 0.5,
            prefer_latest_versions: true,
            conflict_penalty_multiplier: 200.0,
            max_estimated_depth: 15,
        };
        
        Self::new(config)
    }
}

impl DependencyHeuristic for CompositeHeuristic {
    fn calculate(&self, state: &SearchState) -> f64 {
        let mut total_cost = 0.0;
        
        for heuristic in &self.heuristics {
            total_cost += heuristic.calculate(state);
        }
        
        total_cost
    }
    
    fn name(&self) -> &'static str {
        "Composite"
    }
    
    fn is_admissible(&self) -> bool {
        // Composite is admissible only if all component heuristics are admissible
        self.heuristics.iter().all(|h| h.is_admissible())
    }
}

/// Heuristic factory for creating appropriate heuristics based on problem characteristics
pub struct HeuristicFactory;

impl HeuristicFactory {
    /// Create heuristic based on problem complexity
    pub fn create_for_complexity(complexity: usize) -> Box<dyn DependencyHeuristic + Send + Sync> {
        if complexity < 10 {
            // Simple problems: use fast heuristic
            Box::new(CompositeHeuristic::new_fast())
        } else if complexity < 50 {
            // Medium problems: use balanced heuristic
            Box::new(CompositeHeuristic::new(HeuristicConfig::default()))
        } else {
            // Complex problems: use thorough heuristic
            Box::new(CompositeHeuristic::new_thorough())
        }
    }
    
    /// Create heuristic optimized for specific scenarios
    pub fn create_for_scenario(scenario: &str) -> Box<dyn DependencyHeuristic + Send + Sync> {
        match scenario {
            "fast" => Box::new(CompositeHeuristic::new_fast()),
            "thorough" => Box::new(CompositeHeuristic::new_thorough()),
            "conflict_heavy" => {
                let config = HeuristicConfig {
                    conflict_penalty_weight: 50.0,
                    conflict_penalty_multiplier: 500.0,
                    ..Default::default()
                };
                Box::new(CompositeHeuristic::new(config))
            },
            _ => Box::new(CompositeHeuristic::new(HeuristicConfig::default())),
        }
    }
}

/// Adaptive heuristic that adjusts based on search progress
pub struct AdaptiveHeuristic {
    base_heuristic: CompositeHeuristic,
    config: HeuristicConfig,
    /// Statistics for adaptation
    search_stats: AdaptiveStats,
}

#[derive(Debug, Clone, Default)]
struct AdaptiveStats {
    states_evaluated: usize,
    conflicts_encountered: usize,
    avg_branching_factor: f64,
    search_depth: usize,
}

impl AdaptiveHeuristic {
    pub fn new(config: HeuristicConfig) -> Self {
        Self {
            base_heuristic: CompositeHeuristic::new(config.clone()),
            config,
            search_stats: AdaptiveStats::default(),
        }
    }

    /// Update statistics based on search progress
    pub fn update_stats(&mut self, states_evaluated: usize, conflicts: usize, branching_factor: f64, depth: usize) {
        self.search_stats.states_evaluated = states_evaluated;
        self.search_stats.conflicts_encountered = conflicts;
        self.search_stats.avg_branching_factor = branching_factor;
        self.search_stats.search_depth = depth;
    }

    /// Adapt heuristic weights based on current search characteristics
    fn adapt_weights(&self) -> HeuristicConfig {
        let mut adapted_config = self.config.clone();

        // If we're encountering many conflicts, increase conflict penalty
        if self.search_stats.conflicts_encountered > 5 {
            adapted_config.conflict_penalty_weight *= 2.0;
        }

        // If branching factor is high, increase depth weight to prune more aggressively
        if self.search_stats.avg_branching_factor > 10.0 {
            adapted_config.dependency_depth_weight *= 1.5;
        }

        // If search is going deep, increase remaining requirements weight
        if self.search_stats.search_depth > 10 {
            adapted_config.remaining_requirements_weight *= 1.2;
        }

        adapted_config
    }
}

impl DependencyHeuristic for AdaptiveHeuristic {
    fn calculate(&self, state: &SearchState) -> f64 {
        // Use adapted weights for calculation
        let adapted_config = self.adapt_weights();
        let adapted_heuristic = CompositeHeuristic::new(adapted_config);
        adapted_heuristic.calculate(state)
    }

    fn name(&self) -> &'static str {
        "Adaptive"
    }

    fn is_admissible(&self) -> bool {
        // Adaptive heuristic may not be strictly admissible due to weight adjustments
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_state() -> SearchState {
        let requirements = vec![
            PackageRequirement {
                name: "test_package".to_string(),
                requirement_string: "test_package".to_string(),
            },
            PackageRequirement {
                name: "another_package".to_string(),
                requirement_string: "another_package".to_string(),
            },
        ];

        SearchState::new_initial(requirements)
    }

    #[test]
    fn test_remaining_requirements_heuristic() {
        let config = HeuristicConfig::default();
        let heuristic = RemainingRequirementsHeuristic::new(config);
        let state = create_test_state();

        let cost = heuristic.calculate(&state);
        assert_eq!(cost, 2.0); // 2 requirements * weight of 1.0
        assert!(heuristic.is_admissible());
    }

    #[test]
    fn test_conflict_penalty_heuristic() {
        let config = HeuristicConfig::default();
        let heuristic = ConflictPenaltyHeuristic::new(config);
        let mut state = create_test_state();

        // Add a conflict
        let conflict = DependencyConflict {
            package_name: "test_package".to_string(),
            conflicting_requirements: vec![],
            severity: 1.0,
            conflict_type: ConflictType::VersionConflict,
        };
        state.add_conflict(conflict);

        let cost = heuristic.calculate(&state);
        assert!(cost > 0.0);
        assert!(!heuristic.is_admissible()); // Conflict penalty is not admissible
    }

    #[test]
    fn test_dependency_depth_heuristic() {
        let config = HeuristicConfig::default();
        let heuristic = DependencyDepthHeuristic::new(config);
        let state = create_test_state();

        let cost = heuristic.calculate(&state);
        assert!(cost > 0.0);
        assert!(heuristic.is_admissible());
    }

    #[test]
    fn test_composite_heuristic() {
        let heuristic = CompositeHeuristic::new_fast();
        let state = create_test_state();

        let cost = heuristic.calculate(&state);
        assert!(cost > 0.0);
        assert_eq!(heuristic.name(), "Composite");
    }

    #[test]
    fn test_heuristic_factory() {
        let simple_heuristic = HeuristicFactory::create_for_complexity(5);
        let complex_heuristic = HeuristicFactory::create_for_complexity(100);

        let state = create_test_state();
        let simple_cost = simple_heuristic.calculate(&state);
        let complex_cost = complex_heuristic.calculate(&state);

        assert!(simple_cost > 0.0);
        assert!(complex_cost > 0.0);
    }

    #[test]
    fn test_adaptive_heuristic() {
        let config = HeuristicConfig::default();
        let mut heuristic = AdaptiveHeuristic::new(config);
        let state = create_test_state();

        // Test initial calculation
        let initial_cost = heuristic.calculate(&state);
        assert!(initial_cost > 0.0);

        // Update stats to trigger adaptation
        heuristic.update_stats(100, 10, 15.0, 15);

        // Calculate again with adapted weights
        let adapted_cost = heuristic.calculate(&state);
        assert!(adapted_cost > 0.0);
        // Adapted cost should be different due to weight adjustments
        assert_ne!(initial_cost, adapted_cost);
    }
}
