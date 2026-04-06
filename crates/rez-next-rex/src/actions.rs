//! Rex action types

use serde::{Deserialize, Serialize};

/// A single Rex action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RexAction {
    /// The type of action
    pub action_type: RexActionType,
    /// Source package that generated this action (if any)
    pub source_package: Option<String>,
}

/// Types of Rex actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RexActionType {
    /// Set an environment variable
    Setenv { name: String, value: String },
    /// Unset an environment variable
    Unsetenv { name: String },
    /// Prepend to a path-like environment variable
    PrependPath {
        name: String,
        value: String,
        separator: Option<String>,
    },
    /// Append to a path-like environment variable
    AppendPath {
        name: String,
        value: String,
        separator: Option<String>,
    },
    /// Set env var only if not already set
    SetenvIfEmpty { name: String, value: String },
    /// Create an alias
    Alias { name: String, value: String },
    /// Execute a command
    Command { cmd: String },
    /// Source a shell script file
    Source { path: String },
    /// Comment (ignored)
    Comment { text: String },
    /// Reset an environment variable to its original (pre-context) value
    Resetenv { name: String },
    /// Informational message (rez info() - no env effect)
    Info { message: String },
    /// Error message (rez error() - raises on non-strict mode)
    Error { message: String },
    /// Stop execution of commands (rez stop())
    Stop { message: Option<String> },
}

impl RexAction {
    pub fn setenv(name: impl Into<String>, value: impl Into<String>) -> Self {
        RexAction {
            action_type: RexActionType::Setenv {
                name: name.into(),
                value: value.into(),
            },
            source_package: None,
        }
    }

    pub fn unsetenv(name: impl Into<String>) -> Self {
        RexAction {
            action_type: RexActionType::Unsetenv { name: name.into() },
            source_package: None,
        }
    }

    pub fn prepend_path(name: impl Into<String>, value: impl Into<String>) -> Self {
        RexAction {
            action_type: RexActionType::PrependPath {
                name: name.into(),
                value: value.into(),
                separator: None,
            },
            source_package: None,
        }
    }

    pub fn append_path(name: impl Into<String>, value: impl Into<String>) -> Self {
        RexAction {
            action_type: RexActionType::AppendPath {
                name: name.into(),
                value: value.into(),
                separator: None,
            },
            source_package: None,
        }
    }

    pub fn with_source(mut self, package: impl Into<String>) -> Self {
        self.source_package = Some(package.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod test_constructors {
        use super::*;

        #[test]
        fn test_setenv_creates_correct_action() {
            let action = RexAction::setenv("MY_VAR", "hello");
            assert!(action.source_package.is_none());
            match action.action_type {
                RexActionType::Setenv { name, value } => {
                    assert_eq!(name, "MY_VAR");
                    assert_eq!(value, "hello");
                }
                _ => panic!("Expected Setenv variant"),
            }
        }

        #[test]
        fn test_unsetenv_creates_correct_action() {
            let action = RexAction::unsetenv("OLD_VAR");
            assert!(action.source_package.is_none());
            match action.action_type {
                RexActionType::Unsetenv { name } => {
                    assert_eq!(name, "OLD_VAR");
                }
                _ => panic!("Expected Unsetenv variant"),
            }
        }

        #[test]
        fn test_prepend_path_creates_correct_action() {
            let action = RexAction::prepend_path("PATH", "/usr/local/bin");
            assert!(action.source_package.is_none());
            match action.action_type {
                RexActionType::PrependPath {
                    name,
                    value,
                    separator,
                } => {
                    assert_eq!(name, "PATH");
                    assert_eq!(value, "/usr/local/bin");
                    assert!(separator.is_none(), "Default separator should be None");
                }
                _ => panic!("Expected PrependPath variant"),
            }
        }

        #[test]
        fn test_append_path_creates_correct_action() {
            let action = RexAction::append_path("PYTHONPATH", "/opt/mylib");
            assert!(action.source_package.is_none());
            match action.action_type {
                RexActionType::AppendPath {
                    name,
                    value,
                    separator,
                } => {
                    assert_eq!(name, "PYTHONPATH");
                    assert_eq!(value, "/opt/mylib");
                    assert!(separator.is_none());
                }
                _ => panic!("Expected AppendPath variant"),
            }
        }

        #[test]
        fn test_with_source_sets_package() {
            let action = RexAction::setenv("X", "1").with_source("mypkg-1.0");
            assert_eq!(action.source_package, Some("mypkg-1.0".to_string()));
        }

        #[test]
        fn test_with_source_chaining_overwrites() {
            let action = RexAction::setenv("X", "1")
                .with_source("first_pkg")
                .with_source("second_pkg");
            assert_eq!(action.source_package, Some("second_pkg".to_string()));
        }

        #[test]
        fn test_setenv_accepts_string_owned() {
            let name = "VAR".to_string();
            let value = "val".to_string();
            let action = RexAction::setenv(name, value);
            match action.action_type {
                RexActionType::Setenv { name, value } => {
                    assert_eq!(name, "VAR");
                    assert_eq!(value, "val");
                }
                _ => panic!("Expected Setenv"),
            }
        }
    }

    mod test_action_type_variants {
        use super::*;

        #[test]
        fn test_setenv_if_empty_variant_exists() {
            let action = RexActionType::SetenvIfEmpty {
                name: "FOO".to_string(),
                value: "bar".to_string(),
            };
            match action {
                RexActionType::SetenvIfEmpty { name, value } => {
                    assert_eq!(name, "FOO");
                    assert_eq!(value, "bar");
                }
                _ => panic!("Unexpected variant"),
            }
        }

        #[test]
        fn test_alias_variant() {
            let action = RexActionType::Alias {
                name: "ll".to_string(),
                value: "ls -la".to_string(),
            };
            match action {
                RexActionType::Alias { name, value } => {
                    assert_eq!(name, "ll");
                    assert_eq!(value, "ls -la");
                }
                _ => panic!("Unexpected variant"),
            }
        }

        #[test]
        fn test_command_variant() {
            let action = RexActionType::Command {
                cmd: "echo hello".to_string(),
            };
            match action {
                RexActionType::Command { cmd } => assert_eq!(cmd, "echo hello"),
                _ => panic!("Unexpected variant"),
            }
        }

        #[test]
        fn test_source_variant() {
            let action = RexActionType::Source {
                path: "/etc/profile.d/myenv.sh".to_string(),
            };
            match action {
                RexActionType::Source { path } => {
                    assert_eq!(path, "/etc/profile.d/myenv.sh");
                }
                _ => panic!("Unexpected variant"),
            }
        }

        #[test]
        fn test_comment_variant() {
            let action = RexActionType::Comment {
                text: "This is a comment".to_string(),
            };
            match action {
                RexActionType::Comment { text } => assert_eq!(text, "This is a comment"),
                _ => panic!("Unexpected variant"),
            }
        }

        #[test]
        fn test_resetenv_variant() {
            let action = RexActionType::Resetenv {
                name: "RESET_ME".to_string(),
            };
            match action {
                RexActionType::Resetenv { name } => assert_eq!(name, "RESET_ME"),
                _ => panic!("Unexpected variant"),
            }
        }

        #[test]
        fn test_info_variant() {
            let action = RexActionType::Info {
                message: "Package loaded".to_string(),
            };
            match action {
                RexActionType::Info { message } => assert_eq!(message, "Package loaded"),
                _ => panic!("Unexpected variant"),
            }
        }

        #[test]
        fn test_error_variant() {
            let action = RexActionType::Error {
                message: "Something went wrong".to_string(),
            };
            match action {
                RexActionType::Error { message } => {
                    assert_eq!(message, "Something went wrong");
                }
                _ => panic!("Unexpected variant"),
            }
        }

        #[test]
        fn test_stop_with_message() {
            let action = RexActionType::Stop {
                message: Some("Stopping due to conflict".to_string()),
            };
            match action {
                RexActionType::Stop { message } => {
                    assert_eq!(message, Some("Stopping due to conflict".to_string()));
                }
                _ => panic!("Unexpected variant"),
            }
        }

        #[test]
        fn test_stop_without_message() {
            let action = RexActionType::Stop { message: None };
            match action {
                RexActionType::Stop { message } => assert!(message.is_none()),
                _ => panic!("Unexpected variant"),
            }
        }

        #[test]
        fn test_prepend_path_with_custom_separator() {
            let action = RexActionType::PrependPath {
                name: "MYPATH".to_string(),
                value: "/a/b".to_string(),
                separator: Some(",".to_string()),
            };
            match action {
                RexActionType::PrependPath { separator, .. } => {
                    assert_eq!(separator, Some(",".to_string()));
                }
                _ => panic!("Unexpected variant"),
            }
        }

        #[test]
        fn test_append_path_with_custom_separator() {
            let action = RexActionType::AppendPath {
                name: "MYPATH".to_string(),
                value: "/a/b".to_string(),
                separator: Some(":".to_string()),
            };
            match action {
                RexActionType::AppendPath { separator, .. } => {
                    assert_eq!(separator, Some(":".to_string()));
                }
                _ => panic!("Unexpected variant"),
            }
        }
    }

    mod test_serialization {
        use super::*;

        #[test]
        fn test_setenv_serializes_to_json() {
            let action = RexAction::setenv("FOO", "bar");
            let json = serde_json::to_string(&action).unwrap();
            assert!(json.contains("Setenv"));
            assert!(json.contains("FOO"));
            assert!(json.contains("bar"));
        }

        #[test]
        fn test_setenv_roundtrip() {
            let action = RexAction::setenv("MY_VAR", "my_value").with_source("pkg-1.0");
            let json = serde_json::to_string(&action).unwrap();
            let decoded: RexAction = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded.source_package, Some("pkg-1.0".to_string()));
            match decoded.action_type {
                RexActionType::Setenv { name, value } => {
                    assert_eq!(name, "MY_VAR");
                    assert_eq!(value, "my_value");
                }
                _ => panic!("Expected Setenv after roundtrip"),
            }
        }

        #[test]
        fn test_prepend_path_roundtrip() {
            let original = RexAction::prepend_path("PATH", "/usr/bin");
            let json = serde_json::to_string(&original).unwrap();
            let decoded: RexAction = serde_json::from_str(&json).unwrap();
            match decoded.action_type {
                RexActionType::PrependPath { name, value, .. } => {
                    assert_eq!(name, "PATH");
                    assert_eq!(value, "/usr/bin");
                }
                _ => panic!("Expected PrependPath after roundtrip"),
            }
        }

        #[test]
        fn test_stop_with_none_message_roundtrip() {
            let action = RexAction {
                action_type: RexActionType::Stop { message: None },
                source_package: None,
            };
            let json = serde_json::to_string(&action).unwrap();
            let decoded: RexAction = serde_json::from_str(&json).unwrap();
            match decoded.action_type {
                RexActionType::Stop { message } => assert!(message.is_none()),
                _ => panic!("Expected Stop"),
            }
        }
    }
}
