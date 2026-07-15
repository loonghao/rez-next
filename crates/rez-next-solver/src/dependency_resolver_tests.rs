//! Tests for DependencyResolver — basic resolution, version ranges, cycle detection, and DAG.
//! Backtracking / advanced version strategy tests → resolver_version_strategy_tests.rs (Cycle 145)

#[cfg(test)]
mod tests {
    use crate::SolverConfig;
    use crate::dependency_resolver::{
        DependencyResolver, DetailedResolutionResult, ResolutionStats,
    };
    use crate::resolution_state::ResolutionState;
    use crate::solver::ConflictStrategy;
    use rez_next_package::Requirement;
    use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
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
        assert!(
            names.contains(&"b_pkg"),
            "b_pkg dependency should be resolved"
        );
        assert!(
            names.contains(&"c_pkg"),
            "c_pkg dependency should be resolved"
        );
        assert!(
            names.contains(&"d_pkg"),
            "d_pkg should be resolved as transitive dep"
        );
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

    #[test]
    fn test_variant_prefers_fewer_additional_packages_and_honors_conflict() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "platform-windows", "1.0.0", &[]);
        write_package(tmp.path(), "python_embedded", "1.0.0", &[]);
        let package_dir = tmp.path().join("python").join("3.14.5");
        std::fs::create_dir_all(&package_dir).unwrap();
        std::fs::write(
            package_dir.join("package.py"),
            "name = 'python'\nversion = '3.14.5'\nvariants = [\n    ['platform-windows', 'python_embedded'],\n    ['platform-windows', '!python_embedded'],\n]\n",
        )
        .unwrap();

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let mut resolver = DependencyResolver::new(Arc::new(manager), SolverConfig::default());
        let result = rt
            .block_on(resolver.resolve(vec![Requirement::new("python".to_string())]))
            .unwrap();

        let python = result
            .resolved_packages
            .iter()
            .find(|package| package.package.name == "python")
            .unwrap();
        assert_eq!(python.variant_index, Some(1));
        let materialized_root = python.materialized_package().root().unwrap();
        assert!(
            std::path::Path::new(&materialized_root)
                .ends_with(std::path::Path::new("platform-windows").join("!python_embedded"))
        );
        assert!(
            result
                .resolved_packages
                .iter()
                .all(|package| package.package.name != "python_embedded")
        );
    }

    #[test]
    fn test_package_is_rejected_when_no_variant_is_compatible() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "marker", "1.0.0", &[]);
        let package_dir = tmp.path().join("tool").join("1.0.0");
        std::fs::create_dir_all(&package_dir).unwrap();
        std::fs::write(
            package_dir.join("package.py"),
            "name = 'tool'\nversion = '1.0.0'\nvariants = [['!marker']]\n",
        )
        .unwrap();

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let mut resolver = DependencyResolver::new(
            Arc::new(manager),
            SolverConfig {
                strict_mode: true,
                ..SolverConfig::default()
            },
        );

        let error = rt
            .block_on(resolver.resolve(vec![
                Requirement::new("marker".to_string()),
                Requirement::new("tool".to_string()),
            ]))
            .expect_err("a package with no compatible variant cannot resolve");

        assert!(
            error.to_string().contains("tool"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn test_later_constraint_replaces_package_with_compatible_version() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "python", "3.9.0", &[]);
        write_package(tmp.path(), "python", "3.14.0", &[]);
        write_package(tmp.path(), "app", "1.0.0", &["tool"]);
        write_package(tmp.path(), "tool", "1.0.0", &["python-3.7..3.10"]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let mut resolver = DependencyResolver::new(Arc::new(manager), SolverConfig::default());
        let result = rt
            .block_on(resolver.resolve(vec![
                Requirement::new("app".to_string()),
                "python-3..4".parse().unwrap(),
            ]))
            .unwrap();

        let python = result
            .resolved_packages
            .iter()
            .find(|package| package.package.name == "python")
            .unwrap();
        assert_eq!(
            python.package.version.as_ref().unwrap().to_string(),
            "3.9.0"
        );
    }

    #[test]
    fn test_variants_follow_resolved_version_family_and_narrow_it() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "python", "2.7.18", &[]);
        write_package(tmp.path(), "python", "3.9.13", &[]);
        write_package(tmp.path(), "python", "3.11.9", &[]);

        for package in ["future", "tox"] {
            let package_dir = tmp.path().join(package).join("1.0.0");
            std::fs::create_dir_all(&package_dir).unwrap();
            let python_3_range = if package == "future" {
                "python-3.7..3.13"
            } else {
                "python-3.7..3.10"
            };
            std::fs::write(
                package_dir.join("package.py"),
                format!(
                    "name = '{package}'\nversion = '1.0.0'\nvariants = [\n    ['python-2.7'],\n    ['{python_3_range}'],\n]\n"
                ),
            )
            .unwrap();
        }

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let mut resolver = DependencyResolver::new(Arc::new(manager), SolverConfig::default());
        let result = rt
            .block_on(resolver.resolve(vec![
                "python-3..4".parse().unwrap(),
                Requirement::new("future".to_string()),
                Requirement::new("tox".to_string()),
            ]))
            .unwrap();

        for package in ["future", "tox"] {
            assert_eq!(
                result
                    .resolved_packages
                    .iter()
                    .find(|resolved| resolved.package.name == package)
                    .unwrap()
                    .variant_index,
                Some(1)
            );
        }
        let python = result
            .resolved_packages
            .iter()
            .find(|resolved| resolved.package.name == "python")
            .unwrap();
        assert_eq!(
            python.package.version.as_ref().unwrap().to_string(),
            "3.9.13"
        );
    }

    #[test]
    fn test_missing_weak_requirement_does_not_fail_strict_resolve() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut resolver = DependencyResolver::new(
            Arc::new(RepositoryManager::new()),
            SolverConfig {
                strict_mode: true,
                ..SolverConfig::default()
            },
        );

        let result = rt
            .block_on(resolver.resolve(vec!["~optional-1".parse().unwrap()]))
            .unwrap();

        assert!(result.resolved_packages.is_empty());
        assert!(result.failed_requirements.is_empty());
    }

    #[test]
    fn test_replacing_package_removes_dependencies_from_old_version() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "metadata", "2.0.0", &[]);
        write_package(
            tmp.path(),
            "metadata",
            "4.0.0",
            &["typing_extensions-3..4.5"],
        );
        write_package(tmp.path(), "limiter", "1.0.0", &["~metadata-2"]);
        write_package(tmp.path(), "client", "1.0.0", &["typing_extensions-4.5..5"]);
        write_package(tmp.path(), "typing_extensions", "4.5.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let mut resolver = DependencyResolver::new(
            Arc::new(manager),
            SolverConfig {
                strict_mode: true,
                ..SolverConfig::default()
            },
        );

        let result = rt
            .block_on(resolver.resolve(vec![
                Requirement::new("metadata".to_string()),
                Requirement::new("limiter".to_string()),
                Requirement::new("client".to_string()),
            ]))
            .unwrap();

        let version = |name: &str| {
            result
                .resolved_packages
                .iter()
                .find(|resolved| resolved.package.name == name)
                .and_then(|resolved| resolved.package.version.as_ref())
                .unwrap()
                .to_string()
        };
        assert_eq!(version("metadata"), "2.0.0");
        assert_eq!(version("typing_extensions"), "4.5.0");
        assert!(result.failed_requirements.is_empty());
    }

    #[test]
    fn test_strict_resolve_backtracks_package_that_introduced_conflict() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "typing", "3.0.0", &[]);
        write_package(tmp.path(), "typing", "4.0.0", &[]);
        write_package(tmp.path(), "plugin", "1.0.0", &["typing-3"]);
        write_package(tmp.path(), "plugin", "1.1.0", &["typing-4"]);
        write_package(
            tmp.path(),
            "application",
            "1.0.0",
            &["plugin-1", "typing-3"],
        );

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let mut resolver = DependencyResolver::new(
            Arc::new(manager),
            SolverConfig {
                strict_mode: true,
                ..SolverConfig::default()
            },
        );

        let result = rt
            .block_on(resolver.resolve(vec![Requirement::new("application".to_string())]))
            .unwrap();

        let version = |name: &str| {
            result
                .resolved_packages
                .iter()
                .find(|resolved| resolved.package.name == name)
                .and_then(|resolved| resolved.package.version.as_ref())
                .unwrap()
                .to_string()
        };
        assert_eq!(version("plugin"), "1.0.0");
        assert_eq!(version("typing"), "3.0.0");
        assert!(result.stats.backtrack_steps > 0);
    }

    #[test]
    fn test_root_conflict_requirement_excludes_matching_version() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        write_package(tmp.path(), "tool", "1.0.0", &[]);
        write_package(tmp.path(), "tool", "2.0.0", &[]);

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            tmp.path(),
            "test".to_string(),
        )));
        let mut resolver = DependencyResolver::new(
            Arc::new(manager),
            SolverConfig {
                strict_mode: true,
                ..SolverConfig::default()
            },
        );

        let result = rt
            .block_on(resolver.resolve(vec![
                Requirement::new("tool".to_string()),
                "!tool-2".parse().unwrap(),
            ]))
            .unwrap();

        assert_eq!(result.resolved_packages.len(), 1);
        assert_eq!(
            result.resolved_packages[0]
                .package
                .version
                .as_ref()
                .unwrap()
                .to_string(),
            "1.0.0"
        );
    }
}
