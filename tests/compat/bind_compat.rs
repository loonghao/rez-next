use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

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

    binder.bind("testtool", &opts).unwrap();
    let second = binder.bind("testtool", &opts);
    assert!(
        matches!(second, Err(BindError::AlreadyExists(_))),
        "Second bind without force must return AlreadyExists"
    );
}

/// rez bind: force overwrite succeeds
#[test]
fn test_bind_force_replaces_existing() {
    use rez_next_bind::{BindOptions, PackageBinder};
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let binder = PackageBinder::new();

    let base_opts = BindOptions {
        version_override: Some("2.0.0".to_string()),
        install_path: Some(tmp.path().to_path_buf()),
        force: false,
        search_path: false,
        extra_metadata: Vec::new(),
    };

    binder.bind("myapp", &base_opts).unwrap();

    let force_opts = BindOptions {
        force: true,
        ..base_opts
    };
    let result = binder.bind("myapp", &force_opts);
    assert!(result.is_ok(), "Force overwrite must succeed");
}

/// rez bind: version not found returns VersionNotFound error
#[test]
fn test_bind_no_version_no_executable_fails() {
    use rez_next_bind::{BindOptions, PackageBinder};
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let binder = PackageBinder::new();

    let opts = BindOptions {
        version_override: None, // No override
        install_path: Some(tmp.path().to_path_buf()),
        force: false,
        search_path: false, // Don't search PATH
        extra_metadata: Vec::new(),
    };

    // Unlikely tool name — version detection should fail
    let result = binder.bind("rez_next_nonexistent_tool_xyz_12345", &opts);
    assert!(
        result.is_err(),
        "Bind without version and without executable should fail"
    );
}

/// rez bind: list_builtin_binders returns expected tools
#[test]
fn test_bind_builtin_list() {
    use rez_next_bind::list_builtin_binders;

    let binders = list_builtin_binders();
    let expected = ["python", "cmake", "git", "node", "rust", "go"];
    for tool in &expected {
        assert!(
            binders.contains(tool),
            "Built-in binder '{}' should be in list",
            tool
        );
    }
}

/// rez bind: get_builtin_binder returns correct description
#[test]
fn test_bind_builtin_binder_metadata() {
    use rez_next_bind::get_builtin_binder;

    let b = get_builtin_binder("cmake").unwrap();
    assert_eq!(b.name, "cmake");
    assert!(!b.description.is_empty());
    assert!(!b.help_url.is_empty());
    assert!(!b.executables.is_empty());
}

