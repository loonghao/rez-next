//! Suite context — wraps a ResolvedContext with suite-specific metadata

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A context within a suite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteContext {
    /// Name of this context within the suite
    pub name: String,

    /// Package requests used to create this context
    pub requests: Vec<String>,

    /// Tool aliases: maps alias name -> original tool name
    pub tool_aliases: HashMap<String, String>,

    /// Hidden tools: tool names that should not be exposed
    pub hidden_tools: Vec<String>,

    /// Suffix to prepend to all tools from this context (e.g., "maya_")
    pub prefix: Option<String>,

    /// Priority for tool conflict resolution (higher wins)
    pub priority: i32,

    /// Whether this context is disabled
    pub disabled: bool,
}

impl SuiteContext {
    /// Create a new suite context
    pub fn new(name: impl Into<String>, requests: Vec<String>) -> Self {
        Self {
            name: name.into(),
            requests,
            tool_aliases: HashMap::new(),
            hidden_tools: Vec::new(),
            prefix: None,
            priority: 0,
            disabled: false,
        }
    }

    /// Add a tool alias
    pub fn alias_tool(&mut self, alias: impl Into<String>, tool: impl Into<String>) {
        self.tool_aliases.insert(alias.into(), tool.into());
    }

    /// Hide a tool from the suite
    pub fn hide_tool(&mut self, tool: impl Into<String>) {
        let tool = tool.into();
        if !self.hidden_tools.contains(&tool) {
            self.hidden_tools.push(tool);
        }
    }

    /// Get the effective name of a tool (resolving aliases)
    pub fn get_effective_tool_name<'a>(&self, tool: &'a str) -> &'a str {
        // If tool is in our aliases map (forward: alias -> original), return alias
        for original in self.tool_aliases.values() {
            if original == tool {
                return tool;
            }
        }
        tool
    }

    /// Check if a tool is visible (not hidden)
    pub fn is_tool_visible(&self, tool: &str) -> bool {
        !self.hidden_tools.contains(&tool.to_string())
    }

    /// Get the prefixed name of a tool
    pub fn get_prefixed_tool_name(&self, tool: &str) -> String {
        match &self.prefix {
            Some(p) => format!("{}{}", p, tool),
            None => tool.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod test_new {
        use super::*;

        #[test]
        fn test_new_sets_name_and_requests() {
            let ctx = SuiteContext::new("maya", vec!["maya-2023".to_string()]);
            assert_eq!(ctx.name, "maya");
            assert_eq!(ctx.requests, vec!["maya-2023".to_string()]);
        }

        #[test]
        fn test_new_defaults_empty_aliases() {
            let ctx = SuiteContext::new("ctx", vec![]);
            assert!(ctx.tool_aliases.is_empty());
        }

        #[test]
        fn test_new_defaults_no_hidden_tools() {
            let ctx = SuiteContext::new("ctx", vec![]);
            assert!(ctx.hidden_tools.is_empty());
        }

        #[test]
        fn test_new_defaults_no_prefix() {
            let ctx = SuiteContext::new("ctx", vec![]);
            assert!(ctx.prefix.is_none());
        }

        #[test]
        fn test_new_defaults_priority_zero() {
            let ctx = SuiteContext::new("ctx", vec![]);
            assert_eq!(ctx.priority, 0);
        }

        #[test]
        fn test_new_defaults_not_disabled() {
            let ctx = SuiteContext::new("ctx", vec![]);
            assert!(!ctx.disabled);
        }

        #[test]
        fn test_new_with_multiple_requests() {
            let requests = vec!["python-3.9".to_string(), "numpy-1.24".to_string()];
            let ctx = SuiteContext::new("py_env", requests.clone());
            assert_eq!(ctx.requests, requests);
        }
    }

    mod test_alias_tool {
        use super::*;

        #[test]
        fn test_alias_tool_inserts_mapping() {
            let mut ctx = SuiteContext::new("ctx", vec![]);
            ctx.alias_tool("py", "python");
            assert_eq!(ctx.tool_aliases.get("py"), Some(&"python".to_string()));
        }

        #[test]
        fn test_alias_tool_multiple_aliases() {
            let mut ctx = SuiteContext::new("ctx", vec![]);
            ctx.alias_tool("py", "python");
            ctx.alias_tool("pip3", "pip");
            assert_eq!(ctx.tool_aliases.len(), 2);
        }

        #[test]
        fn test_alias_tool_overwrites_same_alias() {
            let mut ctx = SuiteContext::new("ctx", vec![]);
            ctx.alias_tool("py", "python2");
            ctx.alias_tool("py", "python3");
            assert_eq!(ctx.tool_aliases.get("py"), Some(&"python3".to_string()));
        }
    }

    mod test_hide_tool {
        use super::*;

        #[test]
        fn test_hide_tool_adds_to_hidden() {
            let mut ctx = SuiteContext::new("ctx", vec![]);
            ctx.hide_tool("internal_cmd");
            assert!(ctx.hidden_tools.contains(&"internal_cmd".to_string()));
        }

        #[test]
        fn test_hide_tool_no_duplicates() {
            let mut ctx = SuiteContext::new("ctx", vec![]);
            ctx.hide_tool("tool");
            ctx.hide_tool("tool");
            assert_eq!(ctx.hidden_tools.len(), 1);
        }

        #[test]
        fn test_hide_multiple_tools() {
            let mut ctx = SuiteContext::new("ctx", vec![]);
            ctx.hide_tool("a");
            ctx.hide_tool("b");
            assert_eq!(ctx.hidden_tools.len(), 2);
        }
    }

    mod test_is_tool_visible {
        use super::*;

        #[test]
        fn test_visible_tool_returns_true() {
            let ctx = SuiteContext::new("ctx", vec![]);
            assert!(ctx.is_tool_visible("any_tool"));
        }

        #[test]
        fn test_hidden_tool_returns_false() {
            let mut ctx = SuiteContext::new("ctx", vec![]);
            ctx.hide_tool("secret");
            assert!(!ctx.is_tool_visible("secret"));
        }

        #[test]
        fn test_visible_tool_after_hiding_different_tool() {
            let mut ctx = SuiteContext::new("ctx", vec![]);
            ctx.hide_tool("hidden_tool");
            assert!(ctx.is_tool_visible("visible_tool"));
        }
    }

    mod test_get_prefixed_tool_name {
        use super::*;

        #[test]
        fn test_no_prefix_returns_original_name() {
            let ctx = SuiteContext::new("ctx", vec![]);
            assert_eq!(ctx.get_prefixed_tool_name("python"), "python");
        }

        #[test]
        fn test_with_prefix_prepends_correctly() {
            let mut ctx = SuiteContext::new("ctx", vec![]);
            ctx.prefix = Some("maya_".to_string());
            assert_eq!(ctx.get_prefixed_tool_name("python"), "maya_python");
        }

        #[test]
        fn test_empty_prefix_gives_tool_name() {
            let mut ctx = SuiteContext::new("ctx", vec![]);
            ctx.prefix = Some(String::new());
            assert_eq!(ctx.get_prefixed_tool_name("tool"), "tool");
        }
    }

    mod test_get_effective_tool_name {
        use super::*;

        #[test]
        fn test_no_aliases_returns_tool_unchanged() {
            let ctx = SuiteContext::new("ctx", vec![]);
            assert_eq!(ctx.get_effective_tool_name("python"), "python");
        }

        #[test]
        fn test_tool_is_original_in_alias_returns_tool() {
            let mut ctx = SuiteContext::new("ctx", vec![]);
            ctx.alias_tool("py", "python");
            // "python" is in values; get_effective_tool_name("python") should still return "python"
            assert_eq!(ctx.get_effective_tool_name("python"), "python");
        }

        #[test]
        fn test_unknown_tool_returns_itself() {
            let mut ctx = SuiteContext::new("ctx", vec![]);
            ctx.alias_tool("py", "python");
            // "cmake" is neither alias nor original; returns itself
            assert_eq!(ctx.get_effective_tool_name("cmake"), "cmake");
        }
    }

    mod test_serialization {
        use super::*;

        #[test]
        fn test_roundtrip_serde() {
            let mut ctx = SuiteContext::new("test_ctx", vec!["python-3.9".to_string()]);
            ctx.alias_tool("py", "python");
            ctx.hide_tool("internal");
            ctx.prefix = Some("test_".to_string());
            ctx.priority = 5;

            let json = serde_json::to_string(&ctx).unwrap();
            let decoded: SuiteContext = serde_json::from_str(&json).unwrap();

            assert_eq!(decoded.name, "test_ctx");
            assert_eq!(decoded.requests, vec!["python-3.9".to_string()]);
            assert_eq!(decoded.tool_aliases.get("py"), Some(&"python".to_string()));
            assert!(decoded.hidden_tools.contains(&"internal".to_string()));
            assert_eq!(decoded.prefix, Some("test_".to_string()));
            assert_eq!(decoded.priority, 5);
        }

        #[test]
        fn test_disabled_field_roundtrip() {
            let mut ctx = SuiteContext::new("ctx", vec![]);
            ctx.disabled = true;
            let json = serde_json::to_string(&ctx).unwrap();
            let decoded: SuiteContext = serde_json::from_str(&json).unwrap();
            assert!(decoded.disabled);
        }
    }
}
