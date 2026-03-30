//! Suite tool definitions and conflict resolution

use serde::{Deserialize, Serialize};

/// How to handle tool name conflicts between contexts
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolConflictMode {
    /// Raise an error if there is a conflict (default)
    Error,
    /// Use the tool from the context added first
    First,
    /// Use the tool from the context added last
    Last,
    /// Alias conflicting tools with context prefix: `<context>_<tool>`
    Prefix,
}

impl Default for ToolConflictMode {
    fn default() -> Self {
        Self::Error
    }
}

impl std::str::FromStr for ToolConflictMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "error" => Ok(Self::Error),
            "first" => Ok(Self::First),
            "last" => Ok(Self::Last),
            "prefix" => Ok(Self::Prefix),
            _ => Err(format!("Unknown conflict mode: {}", s)),
        }
    }
}

/// A tool exposed by a suite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteTool {
    /// Tool name as exposed in the suite
    pub name: String,
    /// Original tool name in the package (may differ if aliased)
    pub original_name: String,
    /// Name of the context this tool belongs to
    pub context_name: String,
    /// Package that provides this tool (e.g., "python-3.9")
    pub package: String,
    /// Whether this tool is an alias
    pub is_alias: bool,
    /// Whether this tool is hidden (not exposed by default)
    pub is_hidden: bool,
}

impl SuiteTool {
    /// Create a new suite tool
    pub fn new(
        name: impl Into<String>,
        original_name: impl Into<String>,
        context_name: impl Into<String>,
        package: impl Into<String>,
    ) -> Self {
        let name = name.into();
        let original_name = original_name.into();
        let is_alias = name != original_name;
        Self {
            name,
            original_name,
            context_name: context_name.into(),
            package: package.into(),
            is_alias,
            is_hidden: false,
        }
    }

    /// Create a hidden tool
    pub fn hidden(mut self) -> Self {
        self.is_hidden = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_conflict_mode_default() {
        assert_eq!(ToolConflictMode::default(), ToolConflictMode::Error);
    }

    #[test]
    fn test_tool_conflict_mode_from_str() {
        assert_eq!("error".parse::<ToolConflictMode>().unwrap(), ToolConflictMode::Error);
        assert_eq!("first".parse::<ToolConflictMode>().unwrap(), ToolConflictMode::First);
        assert_eq!("last".parse::<ToolConflictMode>().unwrap(), ToolConflictMode::Last);
        assert_eq!("prefix".parse::<ToolConflictMode>().unwrap(), ToolConflictMode::Prefix);
        assert!("unknown".parse::<ToolConflictMode>().is_err());
    }

    #[test]
    fn test_suite_tool_creation() {
        let tool = SuiteTool::new("my_tool", "my_tool", "maya", "maya-2023");
        assert_eq!(tool.name, "my_tool");
        assert_eq!(tool.original_name, "my_tool");
        assert_eq!(tool.context_name, "maya");
        assert!(!tool.is_alias);
        assert!(!tool.is_hidden);
    }

    #[test]
    fn test_suite_tool_alias_detected() {
        let tool = SuiteTool::new("my_alias", "original_tool", "maya", "maya-2023");
        assert_eq!(tool.name, "my_alias");
        assert_eq!(tool.original_name, "original_tool");
        assert!(tool.is_alias, "Renamed tool should be marked as alias");
    }

    #[test]
    fn test_suite_tool_hidden() {
        let tool = SuiteTool::new("my_tool", "my_tool", "ctx", "pkg").hidden();
        assert!(tool.is_hidden);
    }

    #[test]
    fn test_tool_conflict_mode_case_insensitive() {
        assert_eq!("ERROR".parse::<ToolConflictMode>().unwrap(), ToolConflictMode::Error);
        assert_eq!("PREFIX".parse::<ToolConflictMode>().unwrap(), ToolConflictMode::Prefix);
    }
}
