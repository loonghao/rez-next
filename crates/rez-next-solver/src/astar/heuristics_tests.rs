//! Unit tests for heuristic functions.

use super::{
    AdaptiveHeuristic, CompositeHeuristic, ConflictPenaltyHeuristic, DependencyDepthHeuristic,
    DependencyHeuristic, HeuristicConfig, HeuristicFactory, RemainingRequirementsHeuristic,
    VersionPreferenceHeuristic,
};
use crate::astar::search_state::{ConflictType, DependencyConflict, SearchState};
use rez_next_package::{Package, PackageRequirement};

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
    assert!(
        !h.is_admissible(),
        "ConflictPenalty should NOT be admissible"
    );
}

#[test]
fn test_composite_heuristic_not_admissible_with_conflict_penalty() {
    // CompositeHeuristic includes ConflictPenaltyHeuristic (not admissible)
    let h = CompositeHeuristic::new(HeuristicConfig::default());
    assert!(
        !h.is_admissible(),
        "CompositeHeuristic with ConflictPenalty should not be admissible"
    );
}

#[test]
fn test_heuristic_factory_scenario_fast_and_thorough() {
    let fast = HeuristicFactory::create_for_scenario("fast");
    let thorough = HeuristicFactory::create_for_scenario("thorough");
    let state = create_test_state();
    assert!(
        fast.calculate(&state) >= 0.0,
        "fast heuristic should return >= 0"
    );
    assert!(
        thorough.calculate(&state) >= 0.0,
        "thorough heuristic should return >= 0"
    );
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
    assert!(
        cost > 1000.0,
        "conflict_heavy scenario cost with version conflict should be > 1000, got {}",
        cost
    );
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
    assert!(
        cost_core < cost_app,
        "core package depth cost ({}) should be < app package depth cost ({})",
        cost_core,
        cost_app
    );
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
    state_stable
        .resolved_packages
        .insert("mypkg_stable".to_string(), pkg_stable);
    let cost_stable = h.calculate(&state_stable);
    assert!(
        cost_stable > 0.0 && cost_stable < 1.0,
        "Stable v2.0.0 cost should be 0 < cost < 1, got {}",
        cost_stable
    );

    // package with no version → cost = 1.0 * weight
    let mut state_unknown = SearchState::new_initial(vec![]);
    let pkg_unknown = Package::new("mypkg_unknown".to_string());
    state_unknown
        .resolved_packages
        .insert("mypkg_unknown".to_string(), pkg_unknown);
    let cost_unknown = h.calculate(&state_unknown);
    assert!(
        (cost_unknown - 1.0).abs() < 1e-9,
        "Unknown-version cost should be 1.0, got {}",
        cost_unknown
    );

    // stable v1 vs stable v10: v10 should have lower cost (prefer latest)
    let mut state_v1 = SearchState::new_initial(vec![]);
    let mut pkg_v1 = Package::new("mypkg_v1".to_string());
    pkg_v1.version = Some(rez_next_version::Version::parse("1.0.0").unwrap());
    state_v1
        .resolved_packages
        .insert("mypkg_v1".to_string(), pkg_v1);

    let mut state_v10 = SearchState::new_initial(vec![]);
    let mut pkg_v10 = Package::new("mypkg_v10".to_string());
    pkg_v10.version = Some(rez_next_version::Version::parse("10.0.0").unwrap());
    state_v10
        .resolved_packages
        .insert("mypkg_v10".to_string(), pkg_v10);

    let cost_v1 = h.calculate(&state_v1);
    let cost_v10 = h.calculate(&state_v10);
    assert!(
        cost_v10 < cost_v1,
        "v10 ({}) should have lower preference cost than v1 ({}) when prefer_latest=true",
        cost_v10,
        cost_v1
    );
}

#[test]
fn test_version_preference_heuristic_no_version_moderate_cost() {
    let config = HeuristicConfig::default(); // weight=0.1
    let h = VersionPreferenceHeuristic::new(config);
    let mut state = SearchState::new_initial(vec![]);
    let pkg = Package::new("unknown_ver_pkg".to_string()); // no version set
    state
        .resolved_packages
        .insert("unknown_ver_pkg".to_string(), pkg);
    let cost = h.calculate(&state);
    // Unknown version: base_cost=1.0 * weight=0.1 → 0.1
    assert!((cost - 0.1).abs() < 1e-9, "Expected 0.1, got {}", cost);
}
