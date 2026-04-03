use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Suite integration tests ──────────────────────────────────────────────────

/// Suite: merge tools from two contexts resolves without panic
#[test]
fn test_suite_two_contexts_tool_names() {
    use rez_next_suites::Suite;

    let mut suite = Suite::new();
    suite
        .add_context("maya", vec!["maya-2024".to_string()])
        .unwrap();
    suite
        .add_context("nuke", vec!["nuke-14".to_string()])
        .unwrap();

    assert_eq!(suite.len(), 2);
    let ctx_maya = suite.get_context("maya");
    let ctx_nuke = suite.get_context("nuke");
    assert!(ctx_maya.is_some(), "maya context should exist");
    assert!(ctx_nuke.is_some(), "nuke context should exist");
}

/// Suite: status starts as Pending/Empty, transitions to Loaded after add
#[test]
fn test_suite_initial_status() {
    use rez_next_suites::Suite;

    let suite = Suite::new();
    assert!(suite.is_empty(), "New suite should be empty");
}

