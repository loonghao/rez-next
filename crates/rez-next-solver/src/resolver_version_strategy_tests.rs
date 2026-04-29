//! Tests for DependencyResolver — backtracking, version strategy, and advanced constraints.
//! Split from dependency_resolver_tests.rs (Cycle 145) to keep file size ≤500 lines.

#[cfg(test)]
mod tests {
    use crate::dependency_resolver::DependencyResolver;
    use crate::SolverConfig;
    use rez_next_package::Requirement;
    use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
    use std::sync::Arc;

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
}
