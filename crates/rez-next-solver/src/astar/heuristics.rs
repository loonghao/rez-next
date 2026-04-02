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

use super::search_state::{ConflictType, DependencyConflict, SearchState};
use rez_next_package::{Package, PackageRequirement};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

        base_penalty * conflict.severity() * self.config.conflict_penalty_multiplier
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
        let estimated_depth = if requirement.name.contains("core")
            || requirement.name.contains("base")
        {
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

    fn calculate_version_preference_cost(&self, package: &Package) -> f64 {
        let Some(ref version) = package.version else {
            // Unknown version — assign moderate cost
            return 1.0;
        };
        let ver_str = version.as_str();

        // Pre-release indicator: versions containing alpha/beta/rc/dev suffixes
        let is_prerelease = ver_str.contains("alpha")
            || ver_str.contains("beta")
            || ver_str.contains("rc")
            || ver_str.contains("dev")
            || ver_str.contains("pre");

        if is_prerelease {
            // High cost: discourage pre-release packages when prefer_latest is set
            return if self.config.prefer_latest_versions { 5.0 } else { 2.0 };
        }

        // Parse numeric components to determine recency preference.
        // Higher major version → lower cost (prefer newer).
        let major = ver_str
            .split('.')
            .next()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        if self.config.prefer_latest_versions {
            // Diminishing cost as major version grows: cost = 1 / (major + 1)
            1.0 / (major as f64 + 1.0)
        } else {
            // Prefer older/stable: higher major = slightly higher cost
            (major as f64 * 0.05).min(1.0)
        }
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
    #[allow(dead_code)]
    config: HeuristicConfig,
}

impl CompositeHeuristic {
    pub fn new(config: HeuristicConfig) -> Self {
        // Add all heuristics using vec! macro instead of push chain
        let heuristics: Vec<Box<dyn DependencyHeuristic + Send + Sync>> = vec![
            Box::new(RemainingRequirementsHeuristic::new(config.clone())),
            Box::new(ConflictPenaltyHeuristic::new(config.clone())),
            Box::new(DependencyDepthHeuristic::new(config.clone())),
            Box::new(VersionPreferenceHeuristic::new(config.clone())),
        ];

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
            }
            _ => Box::new(CompositeHeuristic::new(HeuristicConfig::default())),
        }
    }
}

/// Adaptive heuristic that adjusts based on search progress
pub struct AdaptiveHeuristic {
    #[allow(dead_code)]
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
    pub fn update_stats(
        &mut self,
        states_evaluated: usize,
        conflicts: usize,
        branching_factor: f64,
        depth: usize,
    ) {
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
            PackageRequirement::new("test_package".to_string()),
            PackageRequirement::new("another_package".to_string()),
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

        state.add_conflict(DependencyConflict::new(
            "test_package".to_string(),
            vec![],
            1.0,
            ConflictType::VersionConflict,
        ));

        let cost = heuristic.calculate(&state);
        assert!(cost > 0.0);
        assert!(!heuristic.is_admissible());
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

    #[test]
    fn test_remaining_requirements_heuristic_name_and_admissibility() {
        let h = RemainingRequirementsHeuristic::new(HeuristicConfig::default());
        assert_eq!(h.name(), "RemainingRequirements");
        assert!(h.is_admissible());
    }

    #[test]
    fn test_conflict_penalty_heuristic_name_not_admissible() {
        let h = ConflictPenaltyHeuristic::new(HeuristicConfig::default());
        assert_eq!(h.name(), "ConflictPenalty");
        assert!(!h.is_admissible(), "ConflictPenalty should NOT be admissible");
    }

    #[test]
    fn test_composite_heuristic_not_admissible_with_conflict_penalty() {
        // CompositeHeuristic includes ConflictPenaltyHeuristic (not admissible)
        let h = CompositeHeuristic::new(HeuristicConfig::default());
        assert!(!h.is_admissible(),
            "CompositeHeuristic with ConflictPenalty should not be admissible");
    }

    #[test]
    fn test_heuristic_factory_scenario_fast_and_thorough() {
        let fast = HeuristicFactory::create_for_scenario("fast");
        let thorough = HeuristicFactory::create_for_scenario("thorough");
        let state = create_test_state();
        assert!(fast.calculate(&state) >= 0.0, "fast heuristic should return >= 0");
        assert!(thorough.calculate(&state) >= 0.0, "thorough heuristic should return >= 0");
    }

    #[test]
    fn test_heuristic_factory_conflict_heavy_scenario() {
        let h = HeuristicFactory::create_for_scenario("conflict_heavy");
        let mut state = create_test_state();
        state.add_conflict(DependencyConflict::new(
            "pkgA".to_string(),
            vec![">=1.0".to_string(), "<2.0".to_string()],
            1.0,
            ConflictType::VersionConflict,
        ));
        let cost = h.calculate(&state);
        // conflict_heavy has high penalty multiplier — cost should be substantial
        assert!(cost > 1000.0,
            "conflict_heavy scenario cost with version conflict should be > 1000, got {}", cost);
    }

    #[test]
    fn test_dependency_depth_heuristic_core_package_lower_depth() {
        let config = HeuristicConfig::default();
        let h = DependencyDepthHeuristic::new(config);

        // State with a "core" package requirement — estimated depth = 1
        let reqs = vec![PackageRequirement::new("core_utils".to_string())];
        let state_core = SearchState::new_initial(reqs);

        // State with an "app" package requirement — estimated depth = 5
        let reqs_app = vec![PackageRequirement::new("my_app".to_string())];
        let state_app = SearchState::new_initial(reqs_app);

        let cost_core = h.calculate(&state_core);
        let cost_app = h.calculate(&state_app);
        assert!(cost_core < cost_app,
            "core package depth cost ({}) should be < app package depth cost ({})",
            cost_core, cost_app);
    }

    #[test]
    fn test_version_preference_heuristic_prerelease_higher_cost() {
        let config_prefer_latest = HeuristicConfig {
            prefer_latest_versions: true,
            version_preference_weight: 1.0, // weight=1 for easy math
            ..Default::default()
        };
        let h = VersionPreferenceHeuristic::new(config_prefer_latest);

        // Stable package: major=2, prefer_latest=true → cost = 1/(2+1) = 0.333
        let mut state_stable = SearchState::new_initial(vec![]);
        let mut pkg_stable = Package::new("mypkg_stable".to_string());
        pkg_stable.version = Some(rez_next_version::Version::parse("2.0.0").unwrap());
        state_stable.resolved_packages.insert("mypkg_stable".to_string(), pkg_stable);
        let cost_stable = h.calculate(&state_stable);
        assert!(cost_stable > 0.0 && cost_stable < 1.0,
            "Stable v2.0.0 cost should be 0 < cost < 1, got {}", cost_stable);

        // package with no version → cost = 1.0 * weight
        let mut state_unknown = SearchState::new_initial(vec![]);
        let pkg_unknown = Package::new("mypkg_unknown".to_string());
        state_unknown.resolved_packages.insert("mypkg_unknown".to_string(), pkg_unknown);
        let cost_unknown = h.calculate(&state_unknown);
        assert!((cost_unknown - 1.0).abs() < 1e-9,
            "Unknown-version cost should be 1.0, got {}", cost_unknown);

        // stable v1 vs stable v10: v10 should have lower cost (prefer latest)
        let mut state_v1 = SearchState::new_initial(vec![]);
        let mut pkg_v1 = Package::new("mypkg_v1".to_string());
        pkg_v1.version = Some(rez_next_version::Version::parse("1.0.0").unwrap());
        state_v1.resolved_packages.insert("mypkg_v1".to_string(), pkg_v1);

        let mut state_v10 = SearchState::new_initial(vec![]);
        let mut pkg_v10 = Package::new("mypkg_v10".to_string());
        pkg_v10.version = Some(rez_next_version::Version::parse("10.0.0").unwrap());
        state_v10.resolved_packages.insert("mypkg_v10".to_string(), pkg_v10);

        let cost_v1 = h.calculate(&state_v1);
        let cost_v10 = h.calculate(&state_v10);
        assert!(cost_v10 < cost_v1,
            "v10 ({}) should have lower preference cost than v1 ({}) when prefer_latest=true",
            cost_v10, cost_v1);
    }

    #[test]
    fn test_version_preference_heuristic_no_version_moderate_cost() {
        let config = HeuristicConfig::default(); // weight=0.1
        let h = VersionPreferenceHeuristic::new(config);
        let mut state = SearchState::new_initial(vec![]);
        let pkg = Package::new("unknown_ver_pkg".to_string()); // no version set
        state.resolved_packages.insert("unknown_ver_pkg".to_string(), pkg);
        let cost = h.calculate(&state);
        // Unknown version: base_cost=1.0 * weight=0.1 → 0.1
        assert!((cost - 0.1).abs() < 1e-9, "Expected 0.1, got {}", cost);
    }
}

