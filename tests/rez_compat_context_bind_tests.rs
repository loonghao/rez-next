//! Rez Compat — bind, package build_requires, and DependencyGraph conflict tests
//!
//! Extracted from rez_compat_context_tests.rs (Cycle 72) to keep it under 1000 lines.
//!
//! See also: rez_compat_context_tests.rs (context JSON/Rex/circular-dep)

// ─── rez.bind compatibility tests ───────────────────────────────────────────

/// rez bind: bind_tool with explicit version writes valid package.py
#[test]
fn test_bind_explicit_version_package_py() {
    use rez_next_bind::{BindOptions, PackageBinder};
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let binder = PackageBinder::new();

    let opts = BindOptions {
        version_override: Some("3.11.4".to_string()),
        install_path: Some(tmp.path().to_path_buf()),
        force: false,
        search_path: false,
        extra_metadata: vec![("description".to_string(), "CPython 3.11.4".to_string())],
    };

    let result = binder.bind("python", &opts).unwrap();

    assert_eq!(result.name, "python");
    assert_eq!(result.version, "3.11.4");

    let content = std::fs::read_to_string(result.install_path.join("package.py")).unwrap();
    assert!(content.contains("name = 'python'"));
    assert!(content.contains("version = '3.11.4'"));
    assert!(content.contains("tools = ['python']"));
}

/// rez bind: duplicate bind without force must fail
#[test]
fn test_bind_no_force_duplicate_fails() {
    use rez_next_bind::{BindError, BindOptions, PackageBinder};
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let binder = PackageBinder::new();

    let opts = BindOptions {
        version_override: Some("1.0.0".to_string()),
        install_path: Some(tmp.path().to_path_buf()),
        force: false,
        search_path: false,
        extra_metadata: Vec::new(),
    };

    // First bind succeeds
    binder.bind("python", &opts).unwrap();

    // Second bind without force must fail with AlreadyExists
    let result = binder.bind("python", &opts);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, BindError::AlreadyExists(_)),
        "Expected AlreadyExists, got {:?}",
        err
    );
}

/// rez bind: force flag replaces existing package
#[test]
fn test_bind_force_replaces_existing() {
    use rez_next_bind::{BindOptions, PackageBinder};
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let binder = PackageBinder::new();

    let opts_first = BindOptions {
        version_override: Some("1.0.0".to_string()),
        install_path: Some(tmp.path().to_path_buf()),
        force: false,
        search_path: false,
        extra_metadata: Vec::new(),
    };
    binder.bind("python", &opts_first).unwrap();

    let opts_force = BindOptions {
        version_override: Some("1.1.0".to_string()),
        install_path: Some(tmp.path().to_path_buf()),
        force: true,
        search_path: false,
        extra_metadata: Vec::new(),
    };
    let result = binder.bind("python", &opts_force).unwrap();
    assert_eq!(
        result.version, "1.1.0",
        "Force bind should update to new version"
    );
}

/// rez bind: no version + no executable in PATH must fail
#[test]
fn test_bind_no_version_no_executable_fails() {
    use rez_next_bind::{BindError, BindOptions, PackageBinder};
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let binder = PackageBinder::new();

    // Use a guaranteed-nonexistent tool name
    let opts = BindOptions {
        version_override: None,
        install_path: Some(tmp.path().to_path_buf()),
        force: false,
        search_path: true,
        extra_metadata: Vec::new(),
    };

    let result = binder.bind("__nonexistent_tool_xyz__", &opts);
    assert!(
        result.is_err(),
        "Binding unknown tool without version override must fail"
    );
    assert!(
        matches!(
            result.unwrap_err(),
            BindError::ToolNotFound(_) | BindError::VersionNotFound(_) | BindError::Other(_)
        ),
        "Should return ToolNotFound, VersionNotFound, or Other"
    );
}

/// rez bind: list available built-in binders (module-level API)
#[test]
fn test_bind_builtin_list() {
    use rez_next_bind::list_builtin_binders;

    let builtins = list_builtin_binders();

    // At minimum python should be bindable
    assert!(
        builtins.contains(&"python"),
        "Built-in binders must include 'python', got: {:?}",
        builtins
    );
}

/// rez bind: built-in binder metadata is well-formed
#[test]
fn test_bind_builtin_binder_metadata() {
    use rez_next_bind::get_builtin_binder;

    let binder = get_builtin_binder("python").unwrap();

    assert!(!binder.name.is_empty(), "binder name must not be empty");
    assert!(
        !binder.description.is_empty(),
        "binder description must not be empty"
    );
}

// ─── requires_private_build_only tests ──────────────────────────────────────

/// rez: package with build-only requirements (private_build_requires)
#[test]
fn test_package_private_build_requires_field() {
    use rez_next_package::Package;

    let mut pkg = Package::new("mypkg".to_string());
    // private_build_requires are stored in build_requires in rez-next
    pkg.build_requires = vec!["cmake-3+".to_string(), "ninja".to_string()];

    assert_eq!(pkg.build_requires.len(), 2);
    assert!(pkg.build_requires.contains(&"cmake-3+".to_string()));
    assert!(pkg.build_requires.contains(&"ninja".to_string()));
}

/// rez: private build requires are parseable as requirements
#[test]
fn test_package_private_build_requires_parseable() {
    use rez_next_package::PackageRequirement;

    let build_reqs = ["cmake-3+", "ninja", "gcc-9+<13", "python-3.9"];
    for req_str in &build_reqs {
        let r = PackageRequirement::parse(req_str);
        assert!(
            r.is_ok(),
            "Private build requirement '{}' should be parseable",
            req_str
        );
    }
}

/// rez: package.py private_build_requires are parsed separately from requires
#[test]
fn test_package_py_private_build_requires_parsed_separately() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'mylib'
version = '1.0.0'

requires = [
    'python-3.9',
]

private_build_requires = [
    'cmake-3+',
    'ninja',
]
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "mylib");
    assert_eq!(pkg.requires, vec!["python-3.9".to_string()]);
    assert!(
        pkg.build_requires.is_empty(),
        "build_requires should stay empty when only private_build_requires is declared"
    );
    assert_eq!(
        pkg.private_build_requires,
        vec!["cmake-3+".to_string(), "ninja".to_string()],
        "private_build_requires should preserve parsed package.py entries"
    );
}

/// rez: package with variants and build requirements
#[test]
fn test_package_variants_and_build_reqs() {
    use rez_next_package::Package;
    use rez_next_version::Version;

    let mut pkg = Package::new("maya_plugin".to_string());
    pkg.version = Some(Version::parse("1.2.0").unwrap());
    pkg.requires = vec!["maya-2024".to_string()];
    pkg.build_requires = vec!["cmake-3".to_string()];
    pkg.variants = vec![
        vec!["python-3.9".to_string()],
        vec!["python-3.10".to_string()],
    ];

    assert_eq!(pkg.variants.len(), 2);
    assert_eq!(pkg.build_requires.len(), 1);
    assert_eq!(pkg.requires.len(), 1);
}

// ─── DependencyGraph conflict detection extended tests ──────────────────────

/// rez: conflict detection reports incompatible python version ranges
#[test]
fn test_dependency_graph_conflict_python_versions() {
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_solver::DependencyGraph;
    use rez_next_version::Version;

    let mut graph = DependencyGraph::new();

    // pkgA requires python-3.9, pkgB requires python-3.11 — incompatible exact specs
    let mut pkg_a = Package::new("pkgA".to_string());
    pkg_a.version = Some(Version::parse("1.0").unwrap());
    pkg_a.requires = vec!["python-3.9".to_string()];

    let mut pkg_b = Package::new("pkgB".to_string());
    pkg_b.version = Some(Version::parse("1.0").unwrap());
    pkg_b.requires = vec!["python-3.11".to_string()];

    graph.add_package(pkg_a).unwrap();
    graph.add_package(pkg_b).unwrap();

    // Add conflicting requirements
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            "3.9".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            "3.11".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    // There should be at least one conflict for python
    assert!(
        !conflicts.is_empty(),
        "Incompatible python version requirements should produce at least one conflict"
    );
    assert_eq!(conflicts[0].package_name, "python");
}

/// rez: no conflict when single requirement for each package
#[test]
fn test_dependency_graph_no_conflict_single_requirements() {
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_solver::DependencyGraph;
    use rez_next_version::Version;

    let mut graph = DependencyGraph::new();

    let mut pkg = Package::new("myapp".to_string());
    pkg.version = Some(Version::parse("1.0").unwrap());
    pkg.requires = vec!["python-3.9".to_string()];
    graph.add_package(pkg).unwrap();

    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            "3.9".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(
        conflicts.is_empty(),
        "Single requirement per package should produce no conflicts"
    );
}

/// rez: graph stats reflects correct node/edge counts
#[test]
fn test_dependency_graph_stats_counts() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;
    use rez_next_version::Version;

    let mut graph = DependencyGraph::new();

    for name in &["a", "b", "c"] {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse("1.0").unwrap());
        graph.add_package(pkg).unwrap();
    }
    graph.add_dependency_edge("a-1.0", "b-1.0").unwrap();
    graph.add_dependency_edge("b-1.0", "c-1.0").unwrap();

    let stats = graph.get_stats();
    assert_eq!(stats.node_count, 3, "Graph should have 3 nodes");
    assert_eq!(stats.edge_count, 2, "Graph should have 2 edges");
}
