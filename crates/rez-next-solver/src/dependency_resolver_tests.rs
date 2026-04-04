//! Tests for DependencyResolver — extracted from dependency_resolver.rs to keep file size ≤1000L.

#[cfg(test)]
mod tests {
    use crate::dependency_resolver::{
        DetailedResolutionResult, DependencyResolver, ResolutionConflict, ResolutionStats,
        ResolvedPackageInfo,
    };
    use crate::resolution_state::ResolutionState;
    use crate::solver::ConflictStrategy;
    use crate::SolverConfig;
    use rez_next_package::Requirement;
    use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
    use serde_json;
    use std::sync::Arc;

    /// Write a minimal package.py to a temp directory and return path
    fn write_package(base: &std::path::Path, name: &str, version: &str, requires: &[&str]) {
        let pkg_dir = base.join(name).join(version);
        std::fs::create_dir_all(&pkg_dir).unwrap();
        let mut content = format!("name = '{}'\nversion = '{}'\n", name, version);
        if !requires.is_empty() {
            content.push_str("requires = [\n");
            for req in requires {
                content.push_str(&format!("    '{}',\n", req));
            }
            content.push_str("]\n");
        }
        std::fs::write(pkg_dir.join("package.py"), content).unwrap();
    }

    #[test]
    fn test_empty_requirements_resolves_empty() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let repo = RepositoryManager::new();
        let repo_arc = Arc::new(repo);
        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());

        let result = rt.block_on(resolver.resolve(vec![])).unwrap();
        assert!(result.resolved_packages.is_empty());
        assert!(result.failed_requirements.is_empty());
    }

    #[test]
    fn test_single_package_file_repo() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "foo", "1.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let req = Requirement::new("foo".to_string());
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();

        assert_eq!(result.resolved_packages.len(), 1);
        assert_eq!(result.resolved_packages[0].package.name, "foo");
    }

    #[test]
    fn test_package_with_dependency_resolved() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "bar", "2.0.0", &[]);
        write_package(tmp.path(), "foo", "1.0.0", &["bar"]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let req = Requirement::new("foo".to_string());
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();

        let names: Vec<&str> = result
            .resolved_packages
            .iter()
            .map(|r| r.package.name.as_str())
            .collect();
        assert!(
            names.contains(&"foo"),
            "foo should be resolved, got: {:?}",
            names
        );
        assert!(
            names.contains(&"bar"),
            "bar dependency should be resolved, got: {:?}",
            names
        );
    }

    #[test]
    fn test_prefer_latest_version_selection() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "foo", "1.0.0", &[]);
        write_package(tmp.path(), "foo", "2.0.0", &[]);
        write_package(tmp.path(), "foo", "1.5.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let config = SolverConfig {
            prefer_latest: true,
            ..Default::default()
        };
        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), config);

        let req = Requirement::new("foo".to_string());
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();

        assert_eq!(result.resolved_packages.len(), 1);
        let ver = result.resolved_packages[0]
            .package
            .version
            .as_ref()
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        assert_eq!(ver, "2.0.0", "Should select latest version (2.0.0)");
    }

    #[test]
    fn test_missing_package_returns_empty_or_error() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let repo = RepositoryManager::new();
        let repo_arc = Arc::new(repo);
        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());

        let req = Requirement::new("nonexistent".to_string());
        let result = rt.block_on(resolver.resolve(vec![req]));

        match result {
            Err(_) => {}
            Ok(r) => {
                assert!(
                    !r.failed_requirements.is_empty() || r.resolved_packages.is_empty(),
                    "Non-existent package should fail or leave failed requirements"
                );
            }
        }
    }

    #[test]
    fn test_solver_config_defaults() {
        let config = SolverConfig::default();
        assert!(config.prefer_latest);
        assert!(config.max_attempts > 0);
        assert!(!config.allow_prerelease);
    }

    #[test]
    fn test_resolution_stats_default() {
        let stats = ResolutionStats::default();
        assert_eq!(stats.packages_considered, 0);
        assert_eq!(stats.variants_evaluated, 0);
        assert_eq!(stats.conflicts_encountered, 0);
    }

    #[test]
    fn test_resolution_result_structure() {
        let result = DetailedResolutionResult {
            resolved_packages: vec![],
            failed_requirements: vec![],
            conflicts: vec![],
            stats: ResolutionStats::default(),
        };
        assert!(result.resolved_packages.is_empty());
        assert!(result.failed_requirements.is_empty());
        assert!(result.conflicts.is_empty());
    }

    /// Phase 61: Verify real VersionRange filtering works in solver
    #[test]
    fn test_version_range_constraint_filters_packages() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "foo", "1.0.0", &[]);
        write_package(tmp.path(), "foo", "1.5.0", &[]);
        write_package(tmp.path(), "foo", "2.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let config = SolverConfig {
            prefer_latest: true,
            ..Default::default()
        };
        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), config);

        let req: Requirement = "foo>=1.0.0,<1.5.0".parse().unwrap();
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();

        assert_eq!(
            result.resolved_packages.len(),
            1,
            "Should resolve exactly one foo"
        );
        let resolved_ver = result.resolved_packages[0]
            .package
            .version
            .as_ref()
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        assert_eq!(
            resolved_ver, "1.0.0",
            "Should select 1.0.0 (only version satisfying >=1.0.0,<1.5.0), got: {}",
            resolved_ver
        );
    }

    /// Verify that a too-strict constraint returns no results
    #[test]
    fn test_version_range_excludes_all() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "bar", "1.0.0", &[]);
        write_package(tmp.path(), "bar", "2.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let req: Requirement = "bar>=3.0".parse().unwrap();
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();

        assert!(
            result.resolved_packages.is_empty() || !result.failed_requirements.is_empty(),
            "bar>=3.0 should not resolve when only 1.0 and 2.0 exist"
        );
    }

    /// Verify exact version constraint
    #[test]
    fn test_exact_version_constraint() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "baz", "1.0.0", &[]);
        write_package(tmp.path(), "baz", "1.5.0", &[]);
        write_package(tmp.path(), "baz", "2.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let req: Requirement = "baz==1.5.0".parse().unwrap();
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();

        assert_eq!(result.resolved_packages.len(), 1);
        let resolved_ver = result.resolved_packages[0]
            .package
            .version
            .as_ref()
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        assert_eq!(resolved_ver, "1.5.0", "Should resolve exact version 1.5.0");
    }

    /// Phase 70: Test diamond dependency pattern
    #[test]
    fn test_diamond_dependency() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "d_pkg", "1.0.0", &[]);
        write_package(tmp.path(), "d_pkg", "1.5.0", &[]);
        write_package(tmp.path(), "d_pkg", "2.0.0", &[]);
        write_package(tmp.path(), "b_pkg", "1.0.0", &["d_pkg>=1.0.0"]);
        write_package(tmp.path(), "c_pkg", "1.0.0", &["d_pkg>=1.5.0"]);
        write_package(tmp.path(), "a_pkg", "1.0.0", &["b_pkg", "c_pkg"]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let req = Requirement::new("a_pkg".to_string());
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();

        let names: Vec<&str> = result
            .resolved_packages
            .iter()
            .map(|r| r.package.name.as_str())
            .collect();
        assert!(names.contains(&"a_pkg"), "a_pkg should be resolved");
        assert!(names.contains(&"b_pkg"), "b_pkg dependency should be resolved");
        assert!(names.contains(&"c_pkg"), "c_pkg dependency should be resolved");
        assert!(names.contains(&"d_pkg"), "d_pkg should be resolved as transitive dep");
    }

    /// Phase 70: Test multi-package resolution in one request
    #[test]
    fn test_multiple_packages_request() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "pkg_x", "1.0.0", &[]);
        write_package(tmp.path(), "pkg_y", "2.0.0", &[]);
        write_package(tmp.path(), "pkg_z", "3.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let reqs = vec![
            Requirement::new("pkg_x".to_string()),
            Requirement::new("pkg_y".to_string()),
            Requirement::new("pkg_z".to_string()),
        ];
        let result = rt.block_on(resolver.resolve(reqs)).unwrap();

        let names: Vec<&str> = result
            .resolved_packages
            .iter()
            .map(|r| r.package.name.as_str())
            .collect();
        assert!(names.contains(&"pkg_x"), "pkg_x should resolve");
        assert!(names.contains(&"pkg_y"), "pkg_y should resolve");
        assert!(names.contains(&"pkg_z"), "pkg_z should resolve");
    }

    /// Phase 70: Test prefer_earliest_version config
    #[test]
    fn test_prefer_earliest_version_selection() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "lib", "1.0.0", &[]);
        write_package(tmp.path(), "lib", "2.0.0", &[]);
        write_package(tmp.path(), "lib", "3.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let config = SolverConfig {
            prefer_latest: false,
            ..Default::default()
        };
        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), config);

        let req = Requirement::new("lib".to_string());
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();

        assert_eq!(result.resolved_packages.len(), 1);
        let ver = result.resolved_packages[0]
            .package
            .version
            .as_ref()
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        assert_eq!(ver, "1.0.0", "Should select earliest version (1.0.0)");
    }

    /// Phase 81: Detect direct cycle A -> B -> A
    #[test]
    fn test_cycle_detection_ab() {
        let mut state = ResolutionState::new(vec![]);
        state.record_dependency("A", "B");
        state.record_dependency("B", "A");
        let cycle = state.detect_cycle();
        assert!(cycle.is_some(), "Should detect A->B->A cycle");
        let c = cycle.unwrap();
        assert!(c.len() >= 2, "Cycle path should have >= 2 nodes: {:?}", c);
    }

    /// Phase 81: No cycle in a linear chain A -> B -> C
    #[test]
    fn test_no_cycle_linear() {
        let mut state = ResolutionState::new(vec![]);
        state.record_dependency("A", "B");
        state.record_dependency("B", "C");
        assert!(state.detect_cycle().is_none(), "Linear chain has no cycle");
    }

    /// Phase 81: Detect 3-node cycle A -> B -> C -> A
    #[test]
    fn test_cycle_detection_three_nodes() {
        let mut state = ResolutionState::new(vec![]);
        state.record_dependency("A", "B");
        state.record_dependency("B", "C");
        state.record_dependency("C", "A");
        let cycle = state.detect_cycle();
        assert!(cycle.is_some(), "Should detect A->B->C->A cycle");
    }

    /// Phase 81: resolver returns Err on cyclic packages
    #[test]
    fn test_resolver_errors_on_cyclic_packages() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "cyclic_a", "1.0.0", &["cyclic_b"]);
        write_package(tmp.path(), "cyclic_b", "1.0.0", &["cyclic_a"]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let req = Requirement::new("cyclic_a".to_string());
        let result = rt.block_on(resolver.resolve(vec![req]));

        assert!(
            result.is_err(),
            "Cyclic dependencies should return an error"
        );
        if let Err(e) = result {
            let msg = format!("{}", e);
            assert!(
                msg.contains("Cyclic") || msg.contains("cycle"),
                "Error message should mention cycle: {}",
                msg
            );
        }
    }

    /// Phase 81: resolver succeeds on a DAG (no cycle), diamond is fine
    #[test]
    fn test_resolver_succeeds_on_dag() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "dag_d", "1.0.0", &[]);
        write_package(tmp.path(), "dag_b", "1.0.0", &["dag_d"]);
        write_package(tmp.path(), "dag_c", "1.0.0", &["dag_d"]);
        write_package(tmp.path(), "dag_a", "1.0.0", &["dag_b", "dag_c"]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let req = Requirement::new("dag_a".to_string());
        let result = rt.block_on(resolver.resolve(vec![req]));
        assert!(
            result.is_ok(),
            "DAG (diamond) should resolve without error: {:?}",
            result
        );
        let r = result.unwrap();
        let names: Vec<&str> = r
            .resolved_packages
            .iter()
            .map(|p| p.package.name.as_str())
            .collect();
        assert!(names.contains(&"dag_a"));
        assert!(names.contains(&"dag_b"));
        assert!(names.contains(&"dag_c"));
        assert!(names.contains(&"dag_d"));
    }

    // ── Phase 89: Backtracking / version-downgrade scenarios ─────────────────

    /// Two packages require different version ranges of a shared dep.
    #[test]
    fn test_shared_dep_intersection() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "shared", "1.0.0", &[]);
        write_package(tmp.path(), "shared", "1.5.0", &[]);
        write_package(tmp.path(), "shared", "2.0.0", &[]);
        write_package(tmp.path(), "pkgA", "1.0.0", &["shared>=1.0.0,<2.0.0"]);
        write_package(tmp.path(), "pkgB", "1.0.0", &["shared>=1.5.0,<2.0.0"]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let reqs = vec![
            Requirement::new("pkgA".to_string()),
            Requirement::new("pkgB".to_string()),
        ];
        let result = rt.block_on(resolver.resolve(reqs)).unwrap();
        let names: Vec<&str> = result
            .resolved_packages
            .iter()
            .map(|r| r.package.name.as_str())
            .collect();
        assert!(names.contains(&"pkgA"), "pkgA should resolve");
        assert!(names.contains(&"pkgB"), "pkgB should resolve");
        assert!(names.contains(&"shared"), "shared should resolve");
    }

    /// Resolver picks latest by default — confirm newest 3.x is selected when available
    #[test]
    fn test_latest_across_major_versions() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "engine", "1.0.0", &[]);
        write_package(tmp.path(), "engine", "2.0.0", &[]);
        write_package(tmp.path(), "engine", "3.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let req = Requirement::new("engine".to_string());
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();
        assert_eq!(result.resolved_packages.len(), 1);
        let ver = result.resolved_packages[0]
            .package
            .version
            .as_ref()
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        assert_eq!(ver, "3.0.0", "Should pick latest 3.0.0");
    }

    /// Backtrack: first candidate 2.0 conflicts via constraint, resolver must use 1.9
    #[test]
    fn test_version_downgrade_on_upper_bound() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "util", "1.8.0", &[]);
        write_package(tmp.path(), "util", "1.9.0", &[]);
        write_package(tmp.path(), "util", "2.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let config = SolverConfig {
            prefer_latest: true,
            ..Default::default()
        };
        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), config);
        let req: Requirement = "util>=1.8.0,<2.0.0".parse().unwrap();
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();
        assert_eq!(result.resolved_packages.len(), 1);
        let ver = result.resolved_packages[0]
            .package
            .version
            .as_ref()
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        assert_eq!(
            ver, "1.9.0",
            "Should downgrade to 1.9.0 given <2.0.0 constraint, got: {}",
            ver
        );
    }

    /// Resolution stats: backtrack_steps and packages_considered are tracked
    #[test]
    fn test_resolution_stats_are_populated() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "alpha", "1.0.0", &[]);
        write_package(tmp.path(), "beta", "2.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let reqs = vec![
            Requirement::new("alpha".to_string()),
            Requirement::new("beta".to_string()),
        ];
        let result = rt.block_on(resolver.resolve(reqs)).unwrap();
        assert!(
            result.stats.packages_considered >= 2,
            "Should have considered at least 2 packages"
        );
    }

    /// conflict_strategy=FailOnConflict config exists and is serializable
    #[test]
    fn test_conflict_strategy_fail_on_conflict_config() {
        let config = SolverConfig {
            conflict_strategy: ConflictStrategy::FailOnConflict,
            ..Default::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(
            json.contains("FailOnConflict"),
            "Serialized config should contain strategy name"
        );
        let back: SolverConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.conflict_strategy, ConflictStrategy::FailOnConflict);
    }

    /// Dedup: requesting same package twice should not resolve it twice
    #[test]
    fn test_duplicate_requirements_dedup() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "mylib", "1.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let reqs = vec![
            Requirement::new("mylib".to_string()),
            Requirement::new("mylib".to_string()),
        ];
        let result = rt.block_on(resolver.resolve(reqs)).unwrap();
        let mylib_count = result
            .resolved_packages
            .iter()
            .filter(|r| r.package.name == "mylib")
            .count();
        assert_eq!(
            mylib_count, 1,
            "mylib should only appear once, got: {}",
            mylib_count
        );
    }

    // ── Phase 113: Solver + VersionRange end-to-end tests ───────────────────

    /// !=2.0 constraint excludes the bad version, selects latest non-excluded
    #[test]
    fn test_ne_constraint_excludes_version() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "lib", "1.0.0", &[]);
        write_package(tmp.path(), "lib", "2.0.0", &[]);
        write_package(tmp.path(), "lib", "3.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let req: Requirement = "lib!=2.0.0".parse().unwrap();
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();
        for r in &result.resolved_packages {
            if r.package.name == "lib" {
                let ver = r
                    .package
                    .version
                    .as_ref()
                    .map(|v| v.as_str().to_string())
                    .unwrap_or_default();
                assert_ne!(ver, "2.0.0", "Should not resolve to excluded version 2.0.0");
            }
        }
    }

    /// compatible release ~=1.2 means >=1.2,<2: selects within 1.x
    #[test]
    fn test_compatible_release_constraint() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "compat_lib", "1.0.0", &[]);
        write_package(tmp.path(), "compat_lib", "1.3.0", &[]);
        write_package(tmp.path(), "compat_lib", "2.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let req: Requirement = "compat_lib~=1.2".parse().unwrap();
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();
        if !result.resolved_packages.is_empty() {
            let ver = result.resolved_packages[0]
                .package
                .version
                .as_ref()
                .map(|v| v.as_str().to_string())
                .unwrap_or_default();
            assert_ne!(
                ver, "2.0.0",
                "~=1.2 should not select 2.0.0 (outside compatible range)"
            );
        }
    }

    /// prefer_latest=false with range: selects lowest in range
    #[test]
    fn test_prefer_earliest_with_range() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "rangelib", "1.0.0", &[]);
        write_package(tmp.path(), "rangelib", "1.5.0", &[]);
        write_package(tmp.path(), "rangelib", "1.9.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let config = SolverConfig {
            prefer_latest: false,
            ..Default::default()
        };
        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), config);
        let req: Requirement = "rangelib>=1.0.0,<2.0.0".parse().unwrap();
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();
        assert!(
            !result.resolved_packages.is_empty(),
            "Should resolve rangelib"
        );
        let ver = result.resolved_packages[0]
            .package
            .version
            .as_ref()
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        assert_eq!(
            ver, "1.0.0",
            "prefer_latest=false should pick 1.0.0 in range, got: {}",
            ver
        );
    }

    /// Empty repository: resolving any package returns empty or failed
    #[test]
    fn test_empty_repository_resolves_empty() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap(); // no packages written

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "empty".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());
        let req = Requirement::new("nonexistent".to_string());
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();
        assert!(
            result.resolved_packages.is_empty() || !result.failed_requirements.is_empty(),
            "Empty repo should yield no resolved packages or failed requirements"
        );
    }

    /// Multi-repo priority: first repo's package is preferred
    #[test]
    fn test_multi_repo_priority() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp1 = tempfile::TempDir::new().unwrap();
        let tmp2 = tempfile::TempDir::new().unwrap();
        write_package(tmp1.path(), "shared_pkg", "2.0.0", &[]);
        write_package(tmp2.path(), "shared_pkg", "1.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp1.path(),
            "repo1".to_string(),
        )));
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp2.path(),
            "repo2".to_string(),
        )));
        let repo_arc = Arc::new(manager);

        let config = SolverConfig {
            prefer_latest: true,
            ..Default::default()
        };
        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), config);
        let req = Requirement::new("shared_pkg".to_string());
        let result = rt.block_on(resolver.resolve(vec![req])).unwrap();
        assert!(
            !result.resolved_packages.is_empty(),
            "Should resolve shared_pkg from multi-repo"
        );
        let ver = result.resolved_packages[0]
            .package
            .version
            .as_ref()
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        assert_eq!(ver, "2.0.0", "Should prefer 2.0.0 (latest) from repo1");
    }

    /// All ConflictStrategy variants are serializable
    #[test]
    fn test_all_conflict_strategies_serializable() {
        let strategies = [
            ConflictStrategy::LatestWins,
            ConflictStrategy::EarliestWins,
            ConflictStrategy::FailOnConflict,
            ConflictStrategy::FindCompatible,
        ];
        for strategy in &strategies {
            let json = serde_json::to_string(strategy).unwrap();
            assert!(
                !json.is_empty(),
                "Strategy should serialize: {:?}",
                strategy
            );
            let back: ConflictStrategy = serde_json::from_str(&json).unwrap();
            assert_eq!(
                back, *strategy,
                "Strategy roundtrip should match: {:?}",
                strategy
            );
        }
    }
}
