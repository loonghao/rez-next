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
        for (_alias, original) in &self.tool_aliases {
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
