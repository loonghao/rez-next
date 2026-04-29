//! Tests for RexExecutor — stop() and error() boundary behaviour.
//! Split from executor_tests.rs (Cycle 145) to keep file size ≤400 lines.

use crate::executor::RexExecutor;

// ── Phase 120 / Cycle 62: stop() / error() boundary tests ────────────────────

/// stop() with no message sets stopped=true and stop_message=None
#[test]
fn test_stop_no_message_sets_stopped_flag() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(r#"stop()"#, "mypkg", None, None)
        .unwrap();
    assert!(env.stopped, "stop() should set stopped=true");
    assert!(
        env.stop_message.is_none(),
        "stop() with no message should leave stop_message=None"
    );
}

/// stop("msg") sets both stopped=true and stop_message
#[test]
fn test_stop_with_message_sets_both_fields() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(r#"stop("hard stop triggered")"#, "mypkg", None, None)
        .unwrap();
    assert!(env.stopped, "stop() should set stopped=true");
    assert_eq!(
        env.stop_message.as_deref(),
        Some("hard stop triggered"),
        "stop message should match"
    );
}

/// stop() message with {root} variable expansion
#[test]
fn test_stop_message_variable_expansion() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"stop("conflict at {root}")"#,
            "mypkg",
            Some("/opt/mypkg/1.0"),
            None,
        )
        .unwrap();
    assert!(env.stopped);
    assert_eq!(
        env.stop_message.as_deref(),
        Some("conflict at /opt/mypkg/1.0"),
        "stop message should have {{root}} expanded"
    );
}

/// stop() message with {version} expansion
#[test]
fn test_stop_message_version_expansion() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"stop("version {version} not supported")"#,
            "mypkg",
            None,
            Some("2.0.0"),
        )
        .unwrap();
    assert!(env.stopped);
    assert_eq!(
        env.stop_message.as_deref(),
        Some("version 2.0.0 not supported")
    );
}

/// stop() aborts processing — actions after stop() are NOT applied
#[test]
fn test_actions_after_stop_are_not_applied() {
    let mut exec = RexExecutor::new();
    let commands = r#"
env.setenv("BEFORE_STOP", "yes")
stop()
env.setenv("AFTER_STOP", "yes")
"#;
    let env = exec
        .execute_commands(commands, "mypkg", None, None)
        .unwrap();
    assert!(env.stopped);
    assert_eq!(
        env.vars.get("BEFORE_STOP"),
        Some(&"yes".to_string()),
        "BEFORE_STOP should be set"
    );
    assert!(
        !env.vars.contains_key("AFTER_STOP"),
        "AFTER_STOP must not be set because stop() aborts processing"
    );
}

/// error() records message with [error] prefix in info_messages
#[test]
fn test_error_action_recorded_in_info_messages() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(r#"error("something went wrong")"#, "mypkg", None, None)
        .unwrap();
    assert!(
        !env.info_messages.is_empty(),
        "error() should produce an info message"
    );
    let msg = &env.info_messages[0];
    assert!(
        msg.contains("something went wrong"),
        "info_messages should contain the error text: {}",
        msg
    );
    assert!(
        msg.starts_with("[error]"),
        "error message should be prefixed with [error]: {}",
        msg
    );
}

/// error() message with {root} variable expansion
#[test]
fn test_error_action_variable_expansion() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"error("failed to find {root}/lib")"#,
            "mypkg",
            Some("/opt/mypkg/3.0"),
            None,
        )
        .unwrap();
    assert!(!env.info_messages.is_empty());
    let msg = &env.info_messages[0];
    assert!(
        msg.contains("/opt/mypkg/3.0/lib"),
        "error message should have {{root}} expanded: {}",
        msg
    );
}

/// Multiple error() calls all recorded in order
#[test]
fn test_multiple_error_actions_all_recorded() {
    let mut exec = RexExecutor::new();
    let commands = r#"
error("first error")
error("second error")
error("third error")
"#;
    let env = exec
        .execute_commands(commands, "mypkg", None, None)
        .unwrap();
    assert_eq!(
        env.info_messages.len(),
        3,
        "All three error() calls should be recorded"
    );
    assert!(env.info_messages[0].contains("first error"));
    assert!(env.info_messages[1].contains("second error"));
    assert!(env.info_messages[2].contains("third error"));
}

/// error() does NOT set stopped=true
#[test]
fn test_error_action_does_not_set_stopped() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(r#"error("non-fatal issue")"#, "mypkg", None, None)
        .unwrap();
    assert!(
        !env.stopped,
        "error() should not set stopped flag (only stop() does)"
    );
    assert!(
        env.stop_message.is_none(),
        "error() should not set stop_message"
    );
}

/// stop() does NOT add to info_messages
#[test]
fn test_stop_action_does_not_populate_info_messages() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(r#"stop("quit now")"#, "mypkg", None, None)
        .unwrap();
    assert!(
        env.info_messages.is_empty(),
        "stop() should not produce info messages, only set stopped flag"
    );
}

/// Combined: error() followed by stop() — both effects recorded independently
#[test]
fn test_error_then_stop_combined() {
    let mut exec = RexExecutor::new();
    let commands = r#"
error("pre-stop warning")
stop("final halt")
"#;
    let env = exec
        .execute_commands(commands, "mypkg", None, None)
        .unwrap();
    assert_eq!(env.info_messages.len(), 1);
    assert!(env.info_messages[0].contains("pre-stop warning"));
    assert!(env.stopped);
    assert_eq!(env.stop_message.as_deref(), Some("final halt"));
}

/// stop() with {name} (package name) expansion
#[test]
fn test_stop_message_name_expansion() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"stop("{name} requires Python 3")"#,
            "mypackage",
            None,
            None,
        )
        .unwrap();
    assert!(env.stopped);
    assert_eq!(
        env.stop_message.as_deref(),
        Some("mypackage requires Python 3")
    );
}

/// error() with custom context variable expansion
#[test]
fn test_error_action_custom_context_var_expansion() {
    let mut exec = RexExecutor::new();
    exec.set_context_var("expected_arch", "x86_64");
    let env = exec
        .execute_commands(
            r#"error("expected arch: {expected_arch}")"#,
            "mypkg",
            None,
            None,
        )
        .unwrap();
    assert!(!env.info_messages.is_empty());
    assert!(
        env.info_messages[0].contains("x86_64"),
        "Custom context var should be expanded in error message: {}",
        env.info_messages[0]
    );
}
