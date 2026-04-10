//! Tests for RexParser — extracted from parser.rs (Cycle 183)

use super::*;
use crate::actions::RexActionType;

#[test]
fn test_parse_setenv() {
    let parser = RexParser::new();
    let commands = r#"
env.setenv('MAYA_VERSION', '2024')
env.setenv("MY_ROOT", "{root}")
"#;
    let actions = parser.parse(commands).unwrap();
    assert_eq!(actions.len(), 2);
}

#[test]
fn test_parse_prepend_path() {
    let parser = RexParser::new();
    let commands = r#"
env.prepend_path('PATH', '{root}/bin')
prependenv('LD_LIBRARY_PATH', '{root}/lib')
"#;
    let actions = parser.parse(commands).unwrap();
    assert_eq!(actions.len(), 2);
}

#[test]
fn test_parse_export() {
    let parser = RexParser::new();
    let commands = r#"
export MYVAR="hello world"
export PATH=/usr/local/bin
"#;
    let actions = parser.parse(commands).unwrap();
    assert_eq!(actions.len(), 2);
}

#[test]
fn test_parse_mixed() {
    let parser = RexParser::new();
    let commands = r#"
# Set the root
env.setenv('PYTHON_ROOT', '{root}')
env.prepend_path('PATH', '{root}/bin')
env.unsetenv('OLD_PYTHON')
alias('python3', '{root}/bin/python3')
"#;
    let actions = parser.parse(commands).unwrap();
    // 1 comment + 4 real actions
    assert_eq!(actions.len(), 5);
}

// ── Phase 82: command() statement parsing ──────────────────────────────

#[test]
fn test_parse_command_double_quotes() {
    let parser = RexParser::new();
    let actions = parser.parse(r#"command("echo hello world")"#).unwrap();
    assert_eq!(actions.len(), 1);
    match &actions[0].action_type {
        RexActionType::Command { cmd } => assert_eq!(cmd, "echo hello world"),
        other => panic!("Expected Command, got {:?}", other),
    }
}

#[test]
fn test_parse_command_single_quotes() {
    let parser = RexParser::new();
    let actions = parser.parse(r#"command('ldconfig')"#).unwrap();
    assert_eq!(actions.len(), 1);
    match &actions[0].action_type {
        RexActionType::Command { cmd } => assert_eq!(cmd, "ldconfig"),
        other => panic!("Expected Command, got {:?}", other),
    }
}

#[test]
fn test_parse_command_with_path_args() {
    let parser = RexParser::new();
    let commands = r#"
env.setenv('PKG_ROOT', '/opt/pkg')
command('/opt/pkg/bin/pkg-config --init')
"#;
    let actions = parser.parse(commands).unwrap();
    assert_eq!(actions.len(), 2);
    match &actions[1].action_type {
        RexActionType::Command { cmd } => {
            assert!(
                cmd.contains("pkg-config"),
                "cmd should contain pkg-config: {}",
                cmd
            );
        }
        other => panic!("Expected Command, got {:?}", other),
    }
}

#[test]
fn test_parse_command_executed_in_rex_environment() {
    use crate::executor::RexExecutor;
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"command('source /etc/profile.d/modules.sh')"#,
            "modulepkg",
            None,
            None,
        )
        .unwrap();
    assert_eq!(
        env.startup_commands,
        vec!["source /etc/profile.d/modules.sh".to_string()]
    );
}

#[test]
fn test_parse_multiple_commands() {
    let parser = RexParser::new();
    let commands = r#"
command('ldconfig')
command('pkg-config --update-cache')
command('update-alternatives --install /usr/bin/python python /usr/bin/python3 10')
"#;
    let actions = parser.parse(commands).unwrap();
    assert_eq!(actions.len(), 3);
    for action in &actions {
        assert!(
            matches!(&action.action_type, RexActionType::Command { .. }),
            "All actions should be Command type"
        );
    }
}

#[test]
fn test_parse_command_with_context_expansion() {
    use crate::executor::RexExecutor;
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"command('{root}/bin/setup.sh')"#,
            "mypkg",
            Some("/opt/mypkg/1.0"),
            Some("1.0"),
        )
        .unwrap();
    assert_eq!(
        env.startup_commands,
        vec!["/opt/mypkg/1.0/bin/setup.sh".to_string()]
    );
}

#[test]
fn test_parse_command_empty_string() {
    let parser = RexParser::new();
    let actions = parser.parse(r#"command('')"#).unwrap();
    assert_eq!(actions.len(), 1);
    match &actions[0].action_type {
        RexActionType::Command { cmd } => assert_eq!(cmd, ""),
        other => panic!("Expected Command, got {:?}", other),
    }
}

// ── Phase 97: source() statement parsing ──────────────────────────────────

#[test]
fn test_parse_source_single_quotes() {
    let parser = RexParser::new();
    let actions = parser.parse(r#"source('/opt/setup.sh')"#).unwrap();
    assert_eq!(actions.len(), 1);
    match &actions[0].action_type {
        RexActionType::Source { path } => assert_eq!(path, "/opt/setup.sh"),
        other => panic!("Expected Source, got {:?}", other),
    }
}

#[test]
fn test_parse_source_double_quotes() {
    let parser = RexParser::new();
    let actions = parser
        .parse(r#"source("/etc/profile.d/myapp.sh")"#)
        .unwrap();
    assert_eq!(actions.len(), 1);
    match &actions[0].action_type {
        RexActionType::Source { path } => assert_eq!(path, "/etc/profile.d/myapp.sh"),
        other => panic!("Expected Source, got {:?}", other),
    }
}

#[test]
fn test_parse_source_with_root_variable() {
    let parser = RexParser::new();
    let actions = parser.parse(r#"source('{root}/etc/setup.sh')"#).unwrap();
    assert_eq!(actions.len(), 1);
    match &actions[0].action_type {
        RexActionType::Source { path } => assert_eq!(path, "{root}/etc/setup.sh"),
        other => panic!("Expected Source, got {:?}", other),
    }
}

#[test]
fn test_parse_source_mixed_with_other_commands() {
    let parser = RexParser::new();
    let commands = r#"
env.setenv('MY_VAR', 'value')
source('/opt/setup.sh')
env.prepend_path('PATH', '{root}/bin')
"#;
    let actions = parser.parse(commands).unwrap();
    assert_eq!(actions.len(), 3);
    assert!(matches!(
        &actions[0].action_type,
        RexActionType::Setenv { .. }
    ));
    assert!(matches!(
        &actions[1].action_type,
        RexActionType::Source { .. }
    ));
    assert!(matches!(
        &actions[2].action_type,
        RexActionType::PrependPath { .. }
    ));
}

#[test]
fn test_parse_source_expands_via_executor() {
    use crate::executor::RexExecutor;
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"source('{root}/etc/setup.sh')"#,
            "mypkg",
            Some("/opt/mypkg/1.0"),
            None,
        )
        .unwrap();
    assert_eq!(env.sourced_scripts.len(), 1);
    assert_eq!(env.sourced_scripts[0], "/opt/mypkg/1.0/etc/setup.sh");
}

// ── resetenv / info / error / stop ─────────────────────────────────────

#[test]
fn test_parse_resetenv_bare() {
    let parser = RexParser::new();
    let actions = parser.parse(r#"resetenv('PATH')"#).unwrap();
    assert_eq!(actions.len(), 1);
    match &actions[0].action_type {
        RexActionType::Resetenv { name } => assert_eq!(name, "PATH"),
        other => panic!("Expected Resetenv, got {:?}", other),
    }
}

#[test]
fn test_parse_resetenv_env_dotted() {
    let parser = RexParser::new();
    let actions = parser.parse(r#"env.resetenv("MY_VAR")"#).unwrap();
    assert_eq!(actions.len(), 1);
    assert!(matches!(
        &actions[0].action_type,
        RexActionType::Resetenv { .. }
    ));
}

#[test]
fn test_resetenv_removes_var_from_env() {
    use crate::executor::RexExecutor;
    let mut exec = RexExecutor::new();
    exec.execute_commands(r#"env.setenv('LEGACY', 'old')"#, "pkg", None, None)
        .unwrap();
    let env = exec
        .execute_commands(r#"resetenv('LEGACY')"#, "pkg", None, None)
        .unwrap();
    assert!(
        !env.vars.contains_key("LEGACY"),
        "resetenv should remove the var"
    );
}

#[test]
fn test_parse_info_message() {
    let parser = RexParser::new();
    let actions = parser.parse(r#"info("package activated")"#).unwrap();
    assert_eq!(actions.len(), 1);
    match &actions[0].action_type {
        RexActionType::Info { message } => assert_eq!(message, "package activated"),
        other => panic!("Expected Info, got {:?}", other),
    }
}

#[test]
fn test_info_message_recorded_in_env() {
    use crate::executor::RexExecutor;
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(r#"info("myapp 1.0 loaded")"#, "myapp", None, None)
        .unwrap();
    assert_eq!(env.info_messages.len(), 1);
    assert_eq!(env.info_messages[0], "myapp 1.0 loaded");
}

#[test]
fn test_parse_error_message() {
    let parser = RexParser::new();
    let actions = parser.parse(r#"error("missing dependency")"#).unwrap();
    assert_eq!(actions.len(), 1);
    match &actions[0].action_type {
        RexActionType::Error { message } => assert_eq!(message, "missing dependency"),
        other => panic!("Expected Error, got {:?}", other),
    }
}

#[test]
fn test_parse_stop_no_message() {
    let parser = RexParser::new();
    let actions = parser.parse(r#"stop()"#).unwrap();
    assert_eq!(actions.len(), 1);
    match &actions[0].action_type {
        RexActionType::Stop { message } => assert!(message.is_none()),
        other => panic!("Expected Stop, got {:?}", other),
    }
}

#[test]
fn test_parse_stop_with_message() {
    let parser = RexParser::new();
    let actions = parser.parse(r#"stop("build failed")"#).unwrap();
    assert_eq!(actions.len(), 1);
    match &actions[0].action_type {
        RexActionType::Stop { message } => assert_eq!(message.as_deref(), Some("build failed")),
        other => panic!("Expected Stop, got {:?}", other),
    }
}

#[test]
fn test_stop_sets_stopped_flag() {
    use crate::executor::RexExecutor;
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(r#"stop("abort")"#, "mypkg", None, None)
        .unwrap();
    assert!(env.stopped, "stop() should set stopped=true");
    assert_eq!(env.stop_message.as_deref(), Some("abort"));
}

#[test]
fn test_info_with_root_expansion() {
    use crate::executor::RexExecutor;
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"info("root is {root}")"#,
            "mypkg",
            Some("/opt/mypkg/2.0"),
            None,
        )
        .unwrap();
    assert_eq!(env.info_messages.len(), 1);
    assert_eq!(env.info_messages[0], "root is /opt/mypkg/2.0");
}

// ── comment() function parsing tests ─────────────────────────────────────

#[test]
fn test_parse_comment_fn_single_quotes() {
    let parser = RexParser::new();
    let actions = parser
        .parse(r#"comment('Set up mylib environment')"#)
        .unwrap();
    assert_eq!(
        actions.len(),
        1,
        "comment() should produce exactly 1 action"
    );
    match &actions[0].action_type {
        RexActionType::Comment { text } => assert_eq!(text, "Set up mylib environment"),
        other => panic!("Expected Comment, got {:?}", other),
    }
}

#[test]
fn test_parse_comment_fn_double_quotes() {
    let parser = RexParser::new();
    let actions = parser
        .parse(r#"comment("Package environment initialized")"#)
        .unwrap();
    assert_eq!(actions.len(), 1);
    match &actions[0].action_type {
        RexActionType::Comment { text } => assert_eq!(text, "Package environment initialized"),
        other => panic!("Expected Comment, got {:?}", other),
    }
}

#[test]
fn test_parse_comment_fn_mixed_with_other_commands() {
    let parser = RexParser::new();
    let commands = r#"
comment('Begin setup')
env.setenv('MY_PKG_ROOT', '{root}')
comment('PATH updated')
env.prepend_path('PATH', '{root}/bin')
"#;
    let actions = parser.parse(commands).unwrap();
    // 2 comment() + 2 real actions = 4 total
    assert_eq!(
        actions.len(),
        4,
        "Should have 4 actions (2 comments + 2 env ops)"
    );
    assert!(
        matches!(&actions[0].action_type, RexActionType::Comment { .. }),
        "first should be Comment"
    );
    assert!(
        matches!(&actions[1].action_type, RexActionType::Setenv { .. }),
        "second should be Setenv"
    );
    assert!(
        matches!(&actions[2].action_type, RexActionType::Comment { .. }),
        "third should be Comment"
    );
    assert!(
        matches!(&actions[3].action_type, RexActionType::PrependPath { .. }),
        "fourth should be PrependPath"
    );
}

#[test]
fn test_comment_fn_and_hash_comment_both_produce_comment_action() {
    let parser = RexParser::new();
    let commands = r#"
# hash-style comment
comment('function-style comment')
"#;
    let actions = parser.parse(commands).unwrap();
    assert_eq!(
        actions.len(),
        2,
        "Both # and comment() should produce Comment actions"
    );
    for action in &actions {
        assert!(
            matches!(&action.action_type, RexActionType::Comment { .. }),
            "Expected Comment, got {:?}",
            action.action_type
        );
    }
}
