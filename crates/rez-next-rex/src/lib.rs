//! # Rex (Rez Execution) Command Language
//!
//! Rex is rez's DSL for modifying the shell environment when entering a package context.
//! This crate implements the Rex command executor, compatible with rez's rex module.
//!
//! ## Supported Commands
//!
//! - `env.setenv("VAR", "value")` — set an environment variable
//! - `env.unsetenv("VAR")` — unset an environment variable
//! - `env.prepend_path("PATH", "/usr/bin")` — prepend to a path variable
//! - `env.append_path("PATH", "/usr/bin")` — append to a path variable
//! - `env.setenv_if_empty("VAR", "default")` — set only if not already set
//! - `setenv("VAR", "value")` — shorthand
//! - `appendenv("VAR", "value")` — append to any env var
//! - `prependenv("VAR", "value")` — prepend to any env var
//! - `unsetenv("VAR")` — shorthand
//! - `alias("name", "command")` — create a command alias
//! - `command("cmd arg1 arg2")` — run a command

use std::collections::HashMap;

pub mod actions;
pub mod executor;
pub mod parser;
pub mod shell;

pub use actions::{RexAction, RexActionType};
pub use executor::RexExecutor;
pub use parser::RexParser;
pub use shell::{generate_shell_script, ShellType};

/// Environment state after applying Rex commands
#[derive(Debug, Clone, Default)]
pub struct RexEnvironment {
    /// Environment variables (name -> value)
    pub vars: HashMap<String, String>,
    /// Aliases (name -> command)
    pub aliases: HashMap<String, String>,
    /// Commands to execute on shell startup
    pub startup_commands: Vec<String>,
    /// Scripts sourced during environment setup
    pub sourced_scripts: Vec<String>,
    /// Info messages emitted by info() calls
    pub info_messages: Vec<String>,
    /// Whether a stop() was encountered
    pub stopped: bool,
    /// Stop message if any
    pub stop_message: Option<String>,
}

impl RexEnvironment {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply all Rex actions to this environment
    pub fn apply(&mut self, actions: &[RexAction]) {
        for action in actions {
            self.apply_action(action);
        }
    }

    fn apply_action(&mut self, action: &RexAction) {
        match &action.action_type {
            RexActionType::Setenv { name, value } => {
                self.vars.insert(name.clone(), value.clone());
            }
            RexActionType::Unsetenv { name } => {
                self.vars.remove(name);
            }
            RexActionType::PrependPath {
                name,
                value,
                separator,
            } => {
                let sep = separator.as_deref().unwrap_or(get_path_sep());
                let current = self.vars.get(name).cloned().unwrap_or_default();
                let new_value = if current.is_empty() {
                    value.clone()
                } else {
                    format!("{}{}{}", value, sep, current)
                };
                self.vars.insert(name.clone(), new_value);
            }
            RexActionType::AppendPath {
                name,
                value,
                separator,
            } => {
                let sep = separator.as_deref().unwrap_or(get_path_sep());
                let current = self.vars.get(name).cloned().unwrap_or_default();
                let new_value = if current.is_empty() {
                    value.clone()
                } else {
                    format!("{}{}{}", current, sep, value)
                };
                self.vars.insert(name.clone(), new_value);
            }
            RexActionType::SetenvIfEmpty { name, value } => {
                if !self.vars.contains_key(name) {
                    self.vars.insert(name.clone(), value.clone());
                }
            }
            RexActionType::Alias { name, value } => {
                self.aliases.insert(name.clone(), value.clone());
            }
            RexActionType::Command { cmd } => {
                self.startup_commands.push(cmd.clone());
            }
            RexActionType::Source { path } => {
                self.sourced_scripts.push(path.clone());
            }
            RexActionType::Comment { .. } => {} // Ignore comments
            RexActionType::Resetenv { name } => {
                // Reset to empty (original value restoration requires process env snapshot)
                self.vars.remove(name);
            }
            RexActionType::Info { message } => {
                self.info_messages.push(message.clone());
            }
            RexActionType::Error { message } => {
                // In non-strict mode, record as info message
                self.info_messages.push(format!("[error] {}", message));
            }
            RexActionType::Stop { message } => {
                self.stopped = true;
                self.stop_message = message.clone();
            }
        }
    }

    /// Merge with base environment (base values are not overwritten by existing env)
    pub fn merge_with_base(&mut self, base: &HashMap<String, String>) {
        for (key, value) in base {
            if !self.vars.contains_key(key) {
                self.vars.insert(key.clone(), value.clone());
            }
        }
    }
}

fn get_path_sep() -> &'static str {
    if cfg!(windows) {
        ";"
    } else {
        ":"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            assert!(val.starts_with("/first"), "Should start with /first: {}", val);
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
        use crate::actions::RexActionType;

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
            assert!(env.info_messages.contains(&"Package python loaded".to_string()));
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
        use crate::actions::RexActionType;

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
}
