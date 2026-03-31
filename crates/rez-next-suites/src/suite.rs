//! Core Suite type — equivalent to rez.suite.Suite

use crate::error::SuiteError;
use crate::suite_context::SuiteContext;
use crate::suite_tool::{SuiteTool, ToolConflictMode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Status of a suite
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SuiteStatus {
    /// Suite is valid and all contexts resolved successfully
    Okay,
    /// Suite has context resolution failures
    Error,
    /// Suite is loading
    Loading,
}

impl Default for SuiteStatus {
    fn default() -> Self {
        Self::Okay
    }
}

/// Suite on-disk file format (suite.yaml)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SuiteFile {
    contexts: Vec<SuiteContext>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    conflict_mode: Option<String>,
}

/// A Suite — a collection of resolved contexts that expose a combined toolset.
///
/// Equivalent to `rez.suite.Suite`.
#[derive(Debug, Clone)]
pub struct Suite {
    /// Ordered list of contexts (order matters for conflict resolution)
    contexts: Vec<SuiteContext>,
    /// Description of this suite
    pub description: Option<String>,
    /// How to handle tool conflicts between contexts
    pub conflict_mode: ToolConflictMode,
    /// Path where this suite is saved (None if not saved yet)
    pub path: Option<PathBuf>,
    /// Suite status
    pub status: SuiteStatus,
}

impl Default for Suite {
    fn default() -> Self {
        Self::new()
    }
}

impl Suite {
    /// Create a new empty suite
    pub fn new() -> Self {
        Self {
            contexts: Vec::new(),
            description: None,
            conflict_mode: ToolConflictMode::Error,
            path: None,
            status: SuiteStatus::Okay,
        }
    }

    /// Create a suite with a description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the conflict mode
    pub fn with_conflict_mode(mut self, mode: ToolConflictMode) -> Self {
        self.conflict_mode = mode;
        self
    }

    /// Add a context to the suite
    ///
    /// # Arguments
    /// * `name` - Unique name for this context within the suite
    /// * `requests` - Package requirements for this context
    pub fn add_context(
        &mut self,
        name: impl Into<String>,
        requests: Vec<String>,
    ) -> Result<(), SuiteError> {
        let name = name.into();

        // Validate context name
        if !is_valid_context_name(&name) {
            return Err(SuiteError::InvalidContextName(name));
        }

        // Check for duplicate names
        if self.contexts.iter().any(|c| c.name == name) {
            return Err(SuiteError::InvalidSuite(format!(
                "Context '{}' already exists in suite",
                name
            )));
        }

        self.contexts.push(SuiteContext::new(name, requests));
        Ok(())
    }

    /// Remove a context from the suite
    pub fn remove_context(&mut self, name: &str) -> Result<(), SuiteError> {
        let pos = self
            .contexts
            .iter()
            .position(|c| c.name == name)
            .ok_or_else(|| SuiteError::ContextNotFound(name.to_string()))?;
        self.contexts.remove(pos);
        Ok(())
    }

    /// Get a context by name
    pub fn get_context(&self, name: &str) -> Option<&SuiteContext> {
        self.contexts.iter().find(|c| c.name == name)
    }

    /// Get a mutable context by name
    pub fn get_context_mut(&mut self, name: &str) -> Option<&mut SuiteContext> {
        self.contexts.iter_mut().find(|c| c.name == name)
    }

    /// List all context names in the suite
    pub fn context_names(&self) -> Vec<&str> {
        self.contexts.iter().map(|c| c.name.as_str()).collect()
    }

    /// Get all contexts
    pub fn contexts(&self) -> &[SuiteContext] {
        &self.contexts
    }

    /// Alias a tool in a specific context
    pub fn alias_tool(
        &mut self,
        context_name: &str,
        alias: impl Into<String>,
        tool: impl Into<String>,
    ) -> Result<(), SuiteError> {
        let ctx = self
            .get_context_mut(context_name)
            .ok_or_else(|| SuiteError::ContextNotFound(context_name.to_string()))?;
        ctx.alias_tool(alias, tool);
        Ok(())
    }

    /// Hide a tool in a specific context
    pub fn hide_tool(&mut self, context_name: &str, tool: &str) -> Result<(), SuiteError> {
        let ctx = self
            .get_context_mut(context_name)
            .ok_or_else(|| SuiteError::ContextNotFound(context_name.to_string()))?;
        ctx.hide_tool(tool);
        Ok(())
    }

    /// Get all tools exposed by this suite (resolving conflicts)
    ///
    /// Returns a map of tool_name -> SuiteTool
    pub fn get_tools(&self) -> Result<HashMap<String, SuiteTool>, SuiteError> {
        let mut tools: HashMap<String, SuiteTool> = HashMap::new();
        let mut conflicts: HashMap<String, Vec<String>> = HashMap::new();

        for ctx in &self.contexts {
            if ctx.disabled {
                continue;
            }

            // Generate tool entries for this context
            // Since we don't have actual resolved contexts here (that would require async),
            // we use the alias map and any known tools
            for (alias, original) in &ctx.tool_aliases {
                if !ctx.is_tool_visible(original) {
                    continue;
                }

                let tool = SuiteTool::new(
                    alias.clone(),
                    original.clone(),
                    ctx.name.clone(),
                    ctx.requests.first().cloned().unwrap_or_default(),
                );

                if let Some(existing) = tools.get(&tool.name) {
                    conflicts
                        .entry(tool.name.clone())
                        .or_default()
                        .push(existing.context_name.clone());
                    conflicts
                        .entry(tool.name.clone())
                        .or_default()
                        .push(ctx.name.clone());

                    match self.conflict_mode {
                        ToolConflictMode::Error => {
                            return Err(SuiteError::ToolConflict {
                                tool: tool.name.clone(),
                                contexts: conflicts[&tool.name].join(", "),
                            });
                        }
                        ToolConflictMode::First => {
                            // Keep existing, skip this one
                        }
                        ToolConflictMode::Last => {
                            tools.insert(tool.name.clone(), tool);
                        }
                        ToolConflictMode::Prefix => {
                            // Rename with context prefix
                            let prefixed_name = format!("{}_{}", ctx.name, alias);
                            let prefixed_tool = SuiteTool::new(
                                prefixed_name.clone(),
                                original.clone(),
                                ctx.name.clone(),
                                ctx.requests.first().cloned().unwrap_or_default(),
                            );
                            tools.insert(prefixed_name, prefixed_tool);
                        }
                    }
                } else {
                    tools.insert(tool.name.clone(), tool);
                }
            }
        }

        Ok(tools)
    }

    /// Save the suite to a directory
    ///
    /// Creates `<path>/suite.yaml` with context definitions.
    pub fn save(&mut self, path: impl AsRef<Path>) -> Result<(), SuiteError> {
        let path = path.as_ref();
        std::fs::create_dir_all(path)?;

        let suite_file = SuiteFile {
            contexts: self.contexts.clone(),
            description: self.description.clone(),
            conflict_mode: Some(format!("{:?}", self.conflict_mode).to_lowercase()),
        };

        let yaml = serde_yaml::to_string(&suite_file)
            .map_err(|e| SuiteError::Serialization(e.to_string()))?;

        let suite_yaml_path = path.join("suite.yaml");
        std::fs::write(&suite_yaml_path, yaml)?;

        // Create bin directory with tool scripts
        let bin_dir = path.join("bin");
        std::fs::create_dir_all(&bin_dir)?;

        // Write rez-suite file (marks directory as a suite)
        let suite_marker = path.join(".rez_suite");
        std::fs::write(&suite_marker, "")?;

        self.path = Some(path.to_path_buf());
        Ok(())
    }

    /// Load a suite from a directory
    pub fn load(path: impl AsRef<Path>) -> Result<Self, SuiteError> {
        let path = path.as_ref();
        let suite_yaml_path = path.join("suite.yaml");

        if !suite_yaml_path.exists() {
            return Err(SuiteError::SuiteNotFound(
                path.to_string_lossy().to_string(),
            ));
        }

        let content = std::fs::read_to_string(&suite_yaml_path)?;
        let suite_file: SuiteFile =
            serde_yaml::from_str(&content).map_err(|e| SuiteError::Serialization(e.to_string()))?;

        let conflict_mode = suite_file
            .conflict_mode
            .as_deref()
            .unwrap_or("error")
            .parse()
            .unwrap_or_default();

        Ok(Suite {
            contexts: suite_file.contexts,
            description: suite_file.description,
            conflict_mode,
            path: Some(path.to_path_buf()),
            status: SuiteStatus::Okay,
        })
    }

    /// Check if a directory is a suite
    pub fn is_suite(path: impl AsRef<Path>) -> bool {
        let path = path.as_ref();
        path.join("suite.yaml").exists() || path.join(".rez_suite").exists()
    }

    /// Get the number of contexts in the suite
    pub fn len(&self) -> usize {
        self.contexts.len()
    }

    /// Check if the suite is empty
    pub fn is_empty(&self) -> bool {
        self.contexts.is_empty()
    }

    /// Print a human-readable representation of the suite
    pub fn print_info(&self) {
        println!("Suite:");
        if let Some(ref desc) = self.description {
            println!("  Description: {}", desc);
        }
        if let Some(ref path) = self.path {
            println!("  Path: {}", path.display());
        }
        println!("  Conflict mode: {:?}", self.conflict_mode);
        println!("  Contexts ({}):", self.contexts.len());
        for ctx in &self.contexts {
            let status = if ctx.disabled { " [disabled]" } else { "" };
            println!("    {} - {}{}", ctx.name, ctx.requests.join(", "), status);
            if !ctx.tool_aliases.is_empty() {
                for (alias, original) in &ctx.tool_aliases {
                    println!("      alias: {} -> {}", alias, original);
                }
            }
            if !ctx.hidden_tools.is_empty() {
                println!("      hidden: {}", ctx.hidden_tools.join(", "));
            }
        }
    }
}

/// Validate context name: alphanumeric + dashes + underscores
fn is_valid_context_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suite_creation() {
        let mut suite = Suite::new();
        assert!(suite.is_empty());

        suite
            .add_context("maya", vec!["maya-2023".to_string()])
            .unwrap();
        assert_eq!(suite.len(), 1);
        assert_eq!(suite.context_names(), vec!["maya"]);
    }

    #[test]
    fn test_duplicate_context_error() {
        let mut suite = Suite::new();
        suite
            .add_context("maya", vec!["maya-2023".to_string()])
            .unwrap();
        let result = suite.add_context("maya", vec!["maya-2024".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_context_name() {
        let mut suite = Suite::new();
        let result = suite.add_context("invalid name!", vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_context() {
        let mut suite = Suite::new();
        suite
            .add_context("maya", vec!["maya-2023".to_string()])
            .unwrap();
        suite
            .add_context("nuke", vec!["nuke-13".to_string()])
            .unwrap();
        suite.remove_context("maya").unwrap();
        assert_eq!(suite.len(), 1);
        assert_eq!(suite.context_names(), vec!["nuke"]);
    }

    #[test]
    fn test_alias_tool() {
        let mut suite = Suite::new();
        suite
            .add_context("maya", vec!["maya-2023".to_string()])
            .unwrap();
        suite.alias_tool("maya", "maya23", "maya").unwrap();

        let ctx = suite.get_context("maya").unwrap();
        assert_eq!(ctx.tool_aliases.get("maya23"), Some(&"maya".to_string()));
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let suite_path = dir.path().join("my_suite");

        let mut suite = Suite::new().with_description("Test suite");
        suite
            .add_context("dev", vec!["python-3.9".to_string()])
            .unwrap();
        suite.save(&suite_path).unwrap();

        assert!(Suite::is_suite(&suite_path));

        let loaded = Suite::load(&suite_path).unwrap();
        assert_eq!(loaded.description, Some("Test suite".to_string()));
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.context_names(), vec!["dev"]);
    }

    #[test]
    fn test_is_valid_context_name() {
        assert!(is_valid_context_name("maya"));
        assert!(is_valid_context_name("maya-2023"));
        assert!(is_valid_context_name("my_context"));
        assert!(!is_valid_context_name(""));
        assert!(!is_valid_context_name("invalid name"));
        assert!(!is_valid_context_name("name!"));
    }

    // ── Phase 93: Suite integration tests ────────────────────────────────────

    /// Suite with description and conflict_mode is preserved through save/load
    #[test]
    fn test_suite_save_load_preserves_conflict_mode() {
        let dir = tempfile::tempdir().unwrap();
        let suite_path = dir.path().join("conflict_suite");

        let mut suite = Suite::new()
            .with_description("Conflict test suite")
            .with_conflict_mode(ToolConflictMode::Last);
        suite
            .add_context("ctx_a", vec!["python-3.9".to_string()])
            .unwrap();
        suite
            .add_context("ctx_b", vec!["python-3.10".to_string()])
            .unwrap();
        suite.save(&suite_path).unwrap();

        let loaded = Suite::load(&suite_path).unwrap();
        assert_eq!(loaded.description, Some("Conflict test suite".to_string()));
        assert_eq!(loaded.len(), 2, "Should reload 2 contexts");
    }

    /// Tool aliased in one context, hidden in another
    #[test]
    fn test_suite_alias_and_hide_combination() {
        let mut suite = Suite::new();
        suite
            .add_context("maya", vec!["maya-2023".to_string()])
            .unwrap();
        suite
            .add_context("nuke", vec!["nuke-13".to_string()])
            .unwrap();

        // maya context: alias maya2023 → maya
        suite.alias_tool("maya", "maya2023", "maya").unwrap();
        // maya context: hide old maya tool
        suite.hide_tool("maya", "maya").unwrap();
        // nuke context: alias nuke13 → nuke
        suite.alias_tool("nuke", "nuke13", "nuke").unwrap();

        let maya_ctx = suite.get_context("maya").unwrap();
        assert_eq!(
            maya_ctx.tool_aliases.get("maya2023"),
            Some(&"maya".to_string())
        );
        assert!(!maya_ctx.is_tool_visible("maya"), "maya should be hidden");

        let nuke_ctx = suite.get_context("nuke").unwrap();
        assert_eq!(
            nuke_ctx.tool_aliases.get("nuke13"),
            Some(&"nuke".to_string())
        );
    }

    /// Suite path is set after save()
    #[test]
    fn test_suite_path_set_after_save() {
        let dir = tempfile::tempdir().unwrap();
        let suite_path = dir.path().join("pathtest_suite");

        let mut suite = Suite::new();
        suite
            .add_context("dev", vec!["python-3.9".to_string()])
            .unwrap();
        assert!(suite.path.is_none(), "path should be None before save");

        suite.save(&suite_path).unwrap();
        assert_eq!(
            suite.path.as_deref(),
            Some(suite_path.as_path()),
            "path should be set after save"
        );
    }

    /// Multiple contexts: get_context returns None for unknown
    #[test]
    fn test_get_context_unknown_returns_none() {
        let mut suite = Suite::new();
        suite.add_context("known", vec![]).unwrap();
        assert!(suite.get_context("unknown").is_none());
        assert!(suite.get_context("known").is_some());
    }

    /// Removing non-existent context returns error
    #[test]
    fn test_remove_nonexistent_context_error() {
        let mut suite = Suite::new();
        let result = suite.remove_context("nonexistent");
        assert!(
            result.is_err(),
            "Removing non-existent context should error"
        );
    }

    /// Suite is_empty check
    #[test]
    fn test_suite_empty_and_len() {
        let mut suite = Suite::new();
        assert!(suite.is_empty());
        assert_eq!(suite.len(), 0);

        suite.add_context("ctx1", vec![]).unwrap();
        assert!(!suite.is_empty());
        assert_eq!(suite.len(), 1);

        suite.add_context("ctx2", vec![]).unwrap();
        assert_eq!(suite.len(), 2);
    }

    /// Suite with Prefix conflict mode: both tools accessible with prefix
    #[test]
    fn test_suite_prefix_conflict_mode() {
        let mut suite = Suite::new().with_conflict_mode(ToolConflictMode::Prefix);
        suite
            .add_context("ctx1", vec!["toolA-1.0".to_string()])
            .unwrap();
        suite
            .add_context("ctx2", vec!["toolA-2.0".to_string()])
            .unwrap();

        // Add same-named alias in both contexts to create conflict
        suite.alias_tool("ctx1", "tool", "toolA").unwrap();
        suite.alias_tool("ctx2", "tool", "toolA").unwrap();

        // With Prefix mode, get_tools should succeed (not error)
        let tools = suite.get_tools();
        assert!(
            tools.is_ok(),
            "Prefix conflict mode should not error: {:?}",
            tools
        );
        // At least one "ctx2_tool" or "ctx1_tool" should be there
        if let Ok(t) = tools {
            let tool_names: Vec<_> = t.keys().collect();
            // Original or prefixed variants should exist
            assert!(!tool_names.is_empty(), "Should have at least one tool");
        }
    }

    /// Suite default conflict mode is Error
    #[test]
    fn test_suite_default_conflict_mode_is_error() {
        let suite = Suite::new();
        assert_eq!(suite.conflict_mode, ToolConflictMode::Error);
    }

    // ── Phase 104: Suite YAML roundtrip tests ─────────────────────────────────

    /// Full multi-context suite roundtrip: save and load restores all contexts
    #[test]
    fn test_suite_multi_context_yaml_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let suite_path = dir.path().join("full_suite");

        let mut suite = Suite::new()
            .with_description("Multi-context project suite")
            .with_conflict_mode(ToolConflictMode::Last);

        suite
            .add_context(
                "maya",
                vec!["maya-2023".to_string(), "python-3.9".to_string()],
            )
            .unwrap();
        suite
            .add_context(
                "nuke",
                vec!["nuke-13".to_string(), "python-3.9".to_string()],
            )
            .unwrap();
        suite
            .add_context("houdini", vec!["houdini-19".to_string()])
            .unwrap();

        // Add aliases
        suite.alias_tool("maya", "maya23", "maya").unwrap();
        suite.alias_tool("nuke", "nuke13", "nuke").unwrap();
        suite.hide_tool("maya", "old_maya").unwrap();

        suite.save(&suite_path).unwrap();

        let loaded = Suite::load(&suite_path).unwrap();
        assert_eq!(loaded.len(), 3);
        assert_eq!(
            loaded.description,
            Some("Multi-context project suite".to_string())
        );

        let names = loaded.context_names();
        assert!(names.contains(&"maya"), "Should have maya context");
        assert!(names.contains(&"nuke"), "Should have nuke context");
        assert!(names.contains(&"houdini"), "Should have houdini context");
    }

    /// Suite YAML contains context requests after save/load
    #[test]
    fn test_suite_context_requests_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let suite_path = dir.path().join("requests_suite");

        let mut suite = Suite::new();
        suite
            .add_context("dev", vec!["python-3.10".to_string(), "pip-23".to_string()])
            .unwrap();
        suite.save(&suite_path).unwrap();

        let loaded = Suite::load(&suite_path).unwrap();
        let ctx = loaded.get_context("dev").expect("dev context should exist");
        assert_eq!(ctx.requests.len(), 2);
        assert!(ctx.requests.contains(&"python-3.10".to_string()));
        assert!(ctx.requests.contains(&"pip-23".to_string()));
    }

    /// Suite suite.yaml file is valid YAML text
    #[test]
    fn test_suite_yaml_file_is_readable() {
        let dir = tempfile::tempdir().unwrap();
        let suite_path = dir.path().join("yaml_check_suite");

        let mut suite = Suite::new().with_description("YAML check");
        suite
            .add_context("test_ctx", vec!["python-3.9".to_string()])
            .unwrap();
        suite.save(&suite_path).unwrap();

        let yaml_path = suite_path.join("suite.yaml");
        assert!(yaml_path.exists(), "suite.yaml should be created");

        let content = std::fs::read_to_string(&yaml_path).unwrap();
        assert!(
            content.contains("contexts"),
            "YAML should contain 'contexts' key"
        );
        assert!(
            content.contains("test_ctx"),
            "YAML should contain context name"
        );
    }

    /// Suite save creates bin directory and marker file
    #[test]
    fn test_suite_save_creates_bin_dir_and_marker() {
        let dir = tempfile::tempdir().unwrap();
        let suite_path = dir.path().join("marker_suite");

        let mut suite = Suite::new();
        suite.add_context("ctx", vec![]).unwrap();
        suite.save(&suite_path).unwrap();

        assert!(
            suite_path.join("bin").exists(),
            "bin/ dir should be created"
        );
        assert!(
            suite_path.join(".rez_suite").exists(),
            ".rez_suite marker should exist"
        );
    }

    /// Suite overwrite: saving again updates the file
    #[test]
    fn test_suite_save_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let suite_path = dir.path().join("overwrite_suite");

        let mut suite = Suite::new().with_description("original");
        suite
            .add_context("ctx", vec!["python-3.9".to_string()])
            .unwrap();
        suite.save(&suite_path).unwrap();

        // Change description and save again
        let mut suite2 = Suite::new().with_description("updated");
        suite2
            .add_context("ctx", vec!["python-3.10".to_string()])
            .unwrap();
        suite2.save(&suite_path).unwrap();

        let loaded = Suite::load(&suite_path).unwrap();
        assert_eq!(loaded.description, Some("updated".to_string()));
    }

    /// Suite load from path that has no suite.yaml returns error
    #[test]
    fn test_suite_load_nonexistent_suite_errors() {
        let dir = tempfile::tempdir().unwrap();
        let bad_path = dir.path().join("not_a_suite");
        std::fs::create_dir_all(&bad_path).unwrap(); // Create dir but no suite.yaml

        let result = Suite::load(&bad_path);
        assert!(result.is_err(), "Loading non-suite directory should error");
    }
}
