//! Rex command parser: parses package commands strings into RexActions

use crate::actions::{RexAction, RexActionType};
use regex::Regex;
use rez_next_common::RezCoreError;
use std::sync::OnceLock;

/// Parser for Rex command strings
pub struct RexParser {
    /// Regex for `env.setenv("VAR", "value")`
    setenv_re: Regex,
    /// Regex for `env.unsetenv("VAR")`
    unsetenv_re: Regex,
    /// Regex for `env.prepend_path("VAR", "value")`
    prepend_re: Regex,
    /// Regex for `env.append_path("VAR", "value")`
    append_re: Regex,
    /// Regex for `env.setenv_if_empty("VAR", "value")`
    setenv_if_empty_re: Regex,
    /// Regex for `alias("name", "cmd")`
    alias_re: Regex,
    /// Regex for plain shell `export VAR=value`
    export_re: Regex,
    /// Regex for csh-style `setenv VAR value`
    setenv_sh_re: Regex,
    /// Regex for `command("cmd arg1 arg2")`
    command_re: Regex,
    /// Regex for `source("path/to/script.sh")`
    source_re: Regex,
    /// Regex for `resetenv("VAR")` or `env.resetenv("VAR")`
    resetenv_re: Regex,
    /// Regex for `info("message")`
    info_re: Regex,
    /// Regex for `error("message")`
    error_re: Regex,
    /// Regex for `stop()` or `stop("message")`
    stop_re: Regex,
    /// Regex for `comment('text')` or `comment("text")`
    comment_fn_re: Regex,
}

impl RexParser {
    pub fn new() -> Self {
        Self {
            // env.setenv('VAR', 'value') or env.setenv("VAR", "value")
            setenv_re: Regex::new(
                r#"env\.setenv\s*\(\s*['"]([^'"]+)['"]\s*,\s*['"]([^'"]*)['"]\s*\)"#
            ).unwrap(),
            // env.unsetenv('VAR')
            unsetenv_re: Regex::new(
                r#"env\.unsetenv\s*\(\s*['"]([^'"]+)['"]\s*\)"#
            ).unwrap(),
            // env.prepend_path('VAR', 'value') or prependenv('VAR', 'value')
            prepend_re: Regex::new(
                r#"(?:env\.prepend_path|prependenv)\s*\(\s*['"]([^'"]+)['"]\s*,\s*['"]([^'"]*)['"]\s*\)"#
            ).unwrap(),
            // env.append_path('VAR', 'value') or appendenv('VAR', 'value')
            append_re: Regex::new(
                r#"(?:env\.append_path|appendenv)\s*\(\s*['"]([^'"]+)['"]\s*,\s*['"]([^'"]*)['"]\s*\)"#
            ).unwrap(),
            // env.setenv_if_empty('VAR', 'value')
            setenv_if_empty_re: Regex::new(
                r#"env\.setenv_if_empty\s*\(\s*['"]([^'"]+)['"]\s*,\s*['"]([^'"]*)['"]\s*\)"#
            ).unwrap(),
            // alias('name', 'cmd')
            alias_re: Regex::new(
                r#"alias\s*\(\s*['"]([^'"]+)['"]\s*,\s*['"]([^'"]*)['"]\s*\)"#
            ).unwrap(),
            // export VAR="value" or export VAR=value
            export_re: Regex::new(
                r#"export\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*"?([^"\n]*)"?"#
            ).unwrap(),
            // setenv VAR value (csh style)
            setenv_sh_re: Regex::new(
                r#"setenv\s+([A-Za-z_][A-Za-z0-9_]*)\s+(.+)"#
            ).unwrap(),
            // command('cmd arg1 arg2') or command("cmd")
            command_re: Regex::new(
                r#"command\s*\(\s*['"]([^'"]*)['"]\s*\)"#
            ).unwrap(),
            // source('path') or source("path")
            source_re: Regex::new(
                r#"source\s*\(\s*['"]([^'"]*)['"]\s*\)"#
            ).unwrap(),
            // resetenv('VAR') or env.resetenv('VAR')
            resetenv_re: Regex::new(
                r#"(?:env\.)?resetenv\s*\(\s*['"]([^'"]+)['"]\s*\)"#
            ).unwrap(),
            // info('message') or info("message")
            info_re: Regex::new(
                r#"^info\s*\(\s*['"]([^'"]*)['"]\s*\)"#
            ).unwrap(),
            // error('message') or error("message")
            error_re: Regex::new(
                r#"^error\s*\(\s*['"]([^'"]*)['"]\s*\)"#
            ).unwrap(),
            // stop() or stop('message') or stop("message")
            stop_re: Regex::new(
                r#"^stop\s*\(\s*(?:['"]([^'"]*)['"]\s*)?\)"#
            ).unwrap(),
            // comment('text') or comment("text")
            comment_fn_re: Regex::new(
                r#"^comment\s*\(\s*['"]([^'"]*)['"]\s*\)"#
            ).unwrap(),
        }
    }

    /// Parse a commands string into a list of RexActions
    pub fn parse(&self, commands: &str) -> Result<Vec<RexAction>, RezCoreError> {
        let mut actions = Vec::new();

        for line in commands.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                if line.starts_with('#') || line.starts_with("//") {
                    actions.push(RexAction {
                        action_type: RexActionType::Comment {
                            text: line.to_string(),
                        },
                        source_package: None,
                    });
                }
                continue;
            }

            // Try each pattern
            if let Some(caps) = self.setenv_re.captures(line) {
                actions.push(RexAction {
                    action_type: RexActionType::Setenv {
                        name: caps[1].to_string(),
                        value: caps[2].to_string(),
                    },
                    source_package: None,
                });
            } else if let Some(caps) = self.unsetenv_re.captures(line) {
                actions.push(RexAction {
                    action_type: RexActionType::Unsetenv {
                        name: caps[1].to_string(),
                    },
                    source_package: None,
                });
            } else if let Some(caps) = self.prepend_re.captures(line) {
                actions.push(RexAction {
                    action_type: RexActionType::PrependPath {
                        name: caps[1].to_string(),
                        value: caps[2].to_string(),
                        separator: None,
                    },
                    source_package: None,
                });
            } else if let Some(caps) = self.append_re.captures(line) {
                actions.push(RexAction {
                    action_type: RexActionType::AppendPath {
                        name: caps[1].to_string(),
                        value: caps[2].to_string(),
                        separator: None,
                    },
                    source_package: None,
                });
            } else if let Some(caps) = self.setenv_if_empty_re.captures(line) {
                actions.push(RexAction {
                    action_type: RexActionType::SetenvIfEmpty {
                        name: caps[1].to_string(),
                        value: caps[2].to_string(),
                    },
                    source_package: None,
                });
            } else if let Some(caps) = self.alias_re.captures(line) {
                actions.push(RexAction {
                    action_type: RexActionType::Alias {
                        name: caps[1].to_string(),
                        value: caps[2].to_string(),
                    },
                    source_package: None,
                });
            } else if let Some(caps) = self.command_re.captures(line) {
                actions.push(RexAction {
                    action_type: RexActionType::Command {
                        cmd: caps[1].to_string(),
                    },
                    source_package: None,
                });
            } else if let Some(caps) = self.source_re.captures(line) {
                actions.push(RexAction {
                    action_type: RexActionType::Source {
                        path: caps[1].to_string(),
                    },
                    source_package: None,
                });
            } else if let Some(caps) = self.resetenv_re.captures(line) {
                actions.push(RexAction {
                    action_type: RexActionType::Resetenv {
                        name: caps[1].to_string(),
                    },
                    source_package: None,
                });
            } else if let Some(caps) = self.info_re.captures(line) {
                actions.push(RexAction {
                    action_type: RexActionType::Info {
                        message: caps[1].to_string(),
                    },
                    source_package: None,
                });
            } else if let Some(caps) = self.error_re.captures(line) {
                actions.push(RexAction {
                    action_type: RexActionType::Error {
                        message: caps[1].to_string(),
                    },
                    source_package: None,
                });
            } else if let Some(caps) = self.stop_re.captures(line) {
                actions.push(RexAction {
                    action_type: RexActionType::Stop {
                        message: caps.get(1).map(|m| m.as_str().to_string()),
                    },
                    source_package: None,
                });
            } else if let Some(caps) = self.comment_fn_re.captures(line) {
                actions.push(RexAction {
                    action_type: RexActionType::Comment {
                        text: caps[1].to_string(),
                    },
                    source_package: None,
                });
            } else if let Some(caps) = self.export_re.captures(line) {
                actions.push(RexAction {
                    action_type: RexActionType::Setenv {
                        name: caps[1].to_string(),
                        value: caps[2].trim_matches('"').to_string(),
                    },
                    source_package: None,
                });
            } else if let Some(caps) = self.setenv_sh_re.captures(line) {
                actions.push(RexAction {
                    action_type: RexActionType::Setenv {
                        name: caps[1].to_string(),
                        value: caps[2].trim().trim_matches('"').to_string(),
                    },
                    source_package: None,
                });
            }
            // Lines that don't match any pattern are silently ignored
            // (could be Python code like `def commands(): ...` or `import ...`)
        }

        Ok(actions)
    }
}

impl Default for RexParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Static cached parser for efficient reuse across multiple parse() calls.
/// Eliminates redundant regex compilation overhead.
static CACHED_PARSER: OnceLock<RexParser> = OnceLock::new();

/// Get a reference to the cached RexParser.
/// On first call, initializes and caches the parser.
/// Subsequent calls return the cached instance.
pub fn get_cached_parser() -> &'static RexParser {
    CACHED_PARSER.get_or_init(RexParser::new)
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
