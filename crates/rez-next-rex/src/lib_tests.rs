//! Tests for RexEnvironment — extracted from lib.rs (Cycle 183)

use super::*;
use crate::actions::RexActionType;

mod test_rex_environment_new {
    use super::*;

    #[test]
    fn test_new_is_empty() {
        let env = RexEnvironment::new();
        assert!(env.vars.is_empty());
        assert!(env.aliases.is_empty());
        assert!(env.startup_commands.is_empty());
        assert!(env.sourced_scripts.is_empty());
        assert!(env.info_messages.is_empty());
        assert!(!env.stopped);
        assert!(env.stop_message.is_none());
    }

    #[test]
    fn test_default_equals_new() {
        let a = RexEnvironment::new();
        let b = RexEnvironment::default();
        assert_eq!(a.vars.len(), b.vars.len());
        assert_eq!(a.stopped, b.stopped);
    }
}

mod test_apply_setenv {
    use super::*;

    #[test]
    fn test_setenv_sets_variable() {
        let mut env = RexEnvironment::new();
        env.apply(&[RexAction::setenv("FOO", "bar")]);
        assert_eq!(env.vars.get("FOO"), Some(&"bar".to_string()));
    }

    #[test]
    fn test_setenv_overwrites_existing() {
        let mut env = RexEnvironment::new();
        env.apply(&[RexAction::setenv("FOO", "first")]);
        env.apply(&[RexAction::setenv("FOO", "second")]);
        assert_eq!(env.vars.get("FOO"), Some(&"second".to_string()));
    }

    #[test]
    fn test_unsetenv_removes_variable() {
        let mut env = RexEnvironment::new();
        env.apply(&[RexAction::setenv("BAR", "value")]);
        env.apply(&[RexAction::unsetenv("BAR")]);
        assert!(!env.vars.contains_key("BAR"));
    }

    #[test]
    fn test_unsetenv_nonexistent_is_noop() {
        let mut env = RexEnvironment::new();
        env.apply(&[RexAction::unsetenv("NONEXISTENT")]);
        assert!(env.vars.is_empty());
    }
}

mod test_apply_paths {
    use super::*;

    #[test]
    fn test_prepend_path_on_empty_var() {
        let mut env = RexEnvironment::new();
        env.apply(&[RexAction::prepend_path("PATH", "/new/path")]);
        assert_eq!(env.vars.get("PATH"), Some(&"/new/path".to_string()));
    }

    #[test]
    fn test_prepend_path_on_existing_var() {
        let mut env = RexEnvironment::new();
        env.apply(&[RexAction::setenv("PATH", "/existing")]);
        env.apply(&[RexAction::prepend_path("PATH", "/prepended")]);
        let sep = if cfg!(windows) { ";" } else { ":" };
        let expected = format!("/prepended{}/existing", sep);
        assert_eq!(env.vars.get("PATH"), Some(&expected));
    }

    #[test]
    fn test_append_path_on_empty_var() {
        let mut env = RexEnvironment::new();
        env.apply(&[RexAction::append_path("PYTHONPATH", "/my/lib")]);
        assert_eq!(env.vars.get("PYTHONPATH"), Some(&"/my/lib".to_string()));
    }

    #[test]
    fn test_append_path_on_existing_var() {
        let mut env = RexEnvironment::new();
        env.apply(&[RexAction::setenv("PYTHONPATH", "/first")]);
        env.apply(&[RexAction::append_path("PYTHONPATH", "/second")]);
        let sep = if cfg!(windows) { ";" } else { ":" };
        let expected = format!("/first{}/second", sep);
        assert_eq!(env.vars.get("PYTHONPATH"), Some(&expected));
    }

    #[test]
    fn test_prepend_then_append_order() {
        let mut env = RexEnvironment::new();
        env.apply(&[RexAction::setenv("PATH", "/mid")]);
        env.apply(&[RexAction::prepend_path("PATH", "/first")]);
        env.apply(&[RexAction::append_path("PATH", "/last")]);
        let sep = if cfg!(windows) { ";" } else { ":" };
        let val = env.vars.get("PATH").unwrap();
        assert!(
            val.starts_with("/first"),
            "Should start with /first: {}",
            val
        );
        assert!(val.ends_with("/last"), "Should end with /last: {}", val);
        assert!(val.contains("/mid"), "Should contain /mid: {}", val);
        let _ = sep;
    }

    #[test]
    fn test_setenv_if_empty_does_not_overwrite() {
        let mut env = RexEnvironment::new();
        env.apply(&[RexAction::setenv("MYVAR", "existing")]);
        env.apply(&[RexAction {
            action_type: crate::actions::RexActionType::SetenvIfEmpty {
                name: "MYVAR".to_string(),
                value: "new_value".to_string(),
            },
            source_package: None,
        }]);
        assert_eq!(env.vars.get("MYVAR"), Some(&"existing".to_string()));
    }

    #[test]
    fn test_setenv_if_empty_sets_when_absent() {
        let mut env = RexEnvironment::new();
        env.apply(&[RexAction {
            action_type: crate::actions::RexActionType::SetenvIfEmpty {
                name: "FRESH_VAR".to_string(),
                value: "default".to_string(),
            },
            source_package: None,
        }]);
        assert_eq!(env.vars.get("FRESH_VAR"), Some(&"default".to_string()));
    }
}

mod test_apply_misc {
    use super::*;

    #[test]
    fn test_alias_recorded() {
        let mut env = RexEnvironment::new();
        env.apply(&[RexAction {
            action_type: RexActionType::Alias {
                name: "ll".to_string(),
                value: "ls -la".to_string(),
            },
            source_package: None,
        }]);
        assert_eq!(env.aliases.get("ll"), Some(&"ls -la".to_string()));
    }

    #[test]
    fn test_command_added_to_startup() {
        let mut env = RexEnvironment::new();
        env.apply(&[RexAction {
            action_type: RexActionType::Command {
                cmd: "echo loaded".to_string(),
            },
            source_package: None,
        }]);
        assert_eq!(env.startup_commands, vec!["echo loaded"]);
    }

    #[test]
    fn test_source_added_to_sourced_scripts() {
        let mut env = RexEnvironment::new();
        env.apply(&[RexAction {
            action_type: RexActionType::Source {
                path: "/etc/profile.d/myenv.sh".to_string(),
            },
            source_package: None,
        }]);
        assert_eq!(env.sourced_scripts, vec!["/etc/profile.d/myenv.sh"]);
    }

    #[test]
    fn test_comment_is_ignored() {
        let mut env = RexEnvironment::new();
        env.apply(&[RexAction {
            action_type: RexActionType::Comment {
                text: "Just a comment".to_string(),
            },
            source_package: None,
        }]);
        assert!(env.vars.is_empty());
        assert!(env.startup_commands.is_empty());
    }

    #[test]
    fn test_resetenv_removes_var() {
        let mut env = RexEnvironment::new();
        env.apply(&[RexAction::setenv("OLD", "value")]);
        env.apply(&[RexAction {
            action_type: RexActionType::Resetenv {
                name: "OLD".to_string(),
            },
            source_package: None,
        }]);
        assert!(!env.vars.contains_key("OLD"));
    }

    #[test]
    fn test_info_recorded_in_messages() {
        let mut env = RexEnvironment::new();
        env.apply(&[RexAction {
            action_type: RexActionType::Info {
                message: "Package python loaded".to_string(),
            },
            source_package: None,
        }]);
        assert!(env
            .info_messages
            .contains(&"Package python loaded".to_string()));
    }

    #[test]
    fn test_error_recorded_as_info_with_prefix() {
        let mut env = RexEnvironment::new();
        env.apply(&[RexAction {
            action_type: RexActionType::Error {
                message: "Version mismatch".to_string(),
            },
            source_package: None,
        }]);
        assert!(env
            .info_messages
            .iter()
            .any(|m| m.contains("[error]") && m.contains("Version mismatch")));
    }

    #[test]
    fn test_stop_sets_stopped_flag() {
        let mut env = RexEnvironment::new();
        env.apply(&[RexAction {
            action_type: RexActionType::Stop { message: None },
            source_package: None,
        }]);
        assert!(env.stopped);
        assert!(env.stop_message.is_none());
    }

    #[test]
    fn test_stop_with_message_stores_message() {
        let mut env = RexEnvironment::new();
        env.apply(&[RexAction {
            action_type: RexActionType::Stop {
                message: Some("Conflict detected".to_string()),
            },
            source_package: None,
        }]);
        assert!(env.stopped);
        assert_eq!(env.stop_message, Some("Conflict detected".to_string()));
    }

    /// stop() aborts apply() — actions after Stop in the slice are never processed
    #[test]
    fn test_stop_aborts_remaining_actions() {
        let mut env = RexEnvironment::new();
        let actions = vec![
            RexAction::setenv("BEFORE", "yes"),
            RexAction {
                action_type: RexActionType::Stop { message: None },
                source_package: None,
            },
            RexAction::setenv("AFTER", "yes"),
        ];
        env.apply(&actions);
        assert!(env.stopped, "stopped flag must be set");
        assert_eq!(
            env.vars.get("BEFORE"),
            Some(&"yes".to_string()),
            "BEFORE must be applied"
        );
        assert!(
            !env.vars.contains_key("AFTER"),
            "AFTER must not be applied because stop() aborts processing"
        );
    }

    /// error() does NOT abort processing — actions after Error continue
    #[test]
    fn test_error_does_not_abort_remaining_actions() {
        let mut env = RexEnvironment::new();
        let actions = vec![
            RexAction {
                action_type: RexActionType::Error {
                    message: "non-fatal".to_string(),
                },
                source_package: None,
            },
            RexAction::setenv("AFTER_ERROR", "yes"),
        ];
        env.apply(&actions);
        assert!(!env.stopped, "error() must not set stopped flag");
        assert_eq!(
            env.vars.get("AFTER_ERROR"),
            Some(&"yes".to_string()),
            "AFTER_ERROR must still be applied after error()"
        );
    }
}

mod test_merge_with_base {
    use super::*;

    #[test]
    fn test_merge_adds_missing_base_vars() {
        let mut env = RexEnvironment::new();
        let mut base = std::collections::HashMap::new();
        base.insert("BASE_VAR".to_string(), "from_base".to_string());
        env.merge_with_base(&base);
        assert_eq!(env.vars.get("BASE_VAR"), Some(&"from_base".to_string()));
    }

    #[test]
    fn test_merge_does_not_overwrite_existing() {
        let mut env = RexEnvironment::new();
        env.apply(&[RexAction::setenv("MY_VAR", "mine")]);
        let mut base = std::collections::HashMap::new();
        base.insert("MY_VAR".to_string(), "from_base".to_string());
        env.merge_with_base(&base);
        assert_eq!(env.vars.get("MY_VAR"), Some(&"mine".to_string()));
    }

    #[test]
    fn test_merge_with_empty_base_is_noop() {
        let mut env = RexEnvironment::new();
        env.apply(&[RexAction::setenv("X", "1")]);
        let base = std::collections::HashMap::new();
        env.merge_with_base(&base);
        assert_eq!(env.vars.len(), 1);
    }

    #[test]
    fn test_merge_multiple_base_vars_all_added() {
        let mut env = RexEnvironment::new();
        let mut base = std::collections::HashMap::new();
        base.insert("A".to_string(), "1".to_string());
        base.insert("B".to_string(), "2".to_string());
        base.insert("C".to_string(), "3".to_string());
        env.merge_with_base(&base);
        assert_eq!(env.vars.len(), 3);
    }
}

mod test_apply_multiple_actions {
    use super::*;

    #[test]
    fn test_apply_batch_of_actions() {
        let mut env = RexEnvironment::new();
        let actions = vec![
            RexAction::setenv("HOME", "/home/user"),
            RexAction::prepend_path("PATH", "/home/user/bin"),
            RexAction {
                action_type: RexActionType::Alias {
                    name: "ll".to_string(),
                    value: "ls -la".to_string(),
                },
                source_package: None,
            },
        ];
        env.apply(&actions);
        assert_eq!(env.vars.get("HOME"), Some(&"/home/user".to_string()));
        assert!(env.vars.contains_key("PATH"));
        assert!(env.aliases.contains_key("ll"));
    }
}
