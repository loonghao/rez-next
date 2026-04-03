use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Suite management tests ─────────────────────────────────────────────────

#[test]
fn test_suite_vfx_pipeline_setup() {
    // Simulate a typical VFX pipeline suite
    let mut suite = Suite::new()
        .with_description("VFX Pipeline Suite v2024")
        .with_conflict_mode(ToolConflictMode::Last);

    suite
        .add_context(
            "maya",
            vec![
                "maya-2024".to_string(),
                "python-3.9".to_string(),
                "mtoa-5".to_string(),
            ],
        )
        .unwrap();

    suite
        .add_context(
            "nuke",
            vec!["nuke-14".to_string(), "python-3.9".to_string()],
        )
        .unwrap();

    suite
        .add_context(
            "houdini",
            vec!["houdini-20".to_string(), "python-3.10".to_string()],
        )
        .unwrap();

    // Set up aliases
    suite.alias_tool("maya", "maya2024", "maya").unwrap();
    suite.alias_tool("nuke", "nuke14", "nuke").unwrap();

    assert_eq!(suite.len(), 3);
    assert_eq!(
        suite.context_names().len(),
        3,
        "Suite should have 3 contexts"
    );
    assert!(suite.get_context("maya").is_some());
    assert!(suite.get_context("nuke").is_some());
    assert!(suite.get_context("houdini").is_some());
}

#[test]
fn test_suite_save_load_roundtrip() {
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let suite_path = tmp.path().join("vfx_pipeline");

    let mut suite = Suite::new().with_description("VFX pipeline suite");

    suite
        .add_context("dcc", vec!["maya-2024".to_string()])
        .unwrap();
    suite
        .add_context("render", vec!["arnold-7".to_string()])
        .unwrap();
    suite.alias_tool("dcc", "maya24", "maya").unwrap();
    suite.save(&suite_path).unwrap();

    // Reload and verify
    let loaded = Suite::load(&suite_path).unwrap();
    assert_eq!(loaded.description, Some("VFX pipeline suite".to_string()));
    assert_eq!(loaded.len(), 2);
    assert!(loaded.context_names().contains(&"dcc"));
    assert!(loaded.context_names().contains(&"render"));
}

#[test]
fn test_suite_is_suite_detection() {
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();

    // Empty directory is not a suite
    assert!(
        !Suite::is_suite(tmp.path()),
        "Empty dir should not be a suite"
    );

    // After saving, it becomes a suite
    let suite_path = tmp.path().join("my_suite");
    let mut suite = Suite::new();
    suite.add_context("ctx", vec![]).unwrap();
    suite.save(&suite_path).unwrap();

    assert!(
        Suite::is_suite(&suite_path),
        "Saved suite dir should be detected as suite"
    );
}

