//! Core types used throughout the Python AST parser.

use std::collections::HashMap;

/// Context for tracking variables and imports during parsing
#[derive(Debug, Default)]
pub(crate) struct ParsingContext {
    /// Variables defined in the current scope
    pub variables: HashMap<String, PythonValue>,
    /// Imported modules and their aliases
    pub imports: HashMap<String, String>,
    /// Current function scope (for nested function handling)
    pub function_scope: Vec<String>,
}

/// Represents a Python value that can be evaluated
#[derive(Debug, Clone)]
pub(crate) enum PythonValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    List(Vec<PythonValue>),
    Dict(HashMap<String, PythonValue>),
    None,
    /// For complex expressions that need runtime evaluation
    Expression(String),
}

/// Intermediate data structure for collecting package information during parsing
#[derive(Debug, Default)]
pub(crate) struct PackageData {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub build_command: Option<String>,
    pub build_system: Option<String>,
    pub pre_commands: Option<String>,
    pub post_commands: Option<String>,
    pub pre_test_commands: Option<String>,
    pub pre_build_commands: Option<String>,
    pub tests: HashMap<String, String>,
    pub requires_rez_version: Option<String>,
    pub uuid: Option<String>,
    pub authors: Vec<String>,
    pub requires: Vec<String>,
    pub build_requires: Vec<String>,
    pub private_build_requires: Vec<String>,
    pub tools: Vec<String>,
    pub variants: Vec<Vec<String>>,
    pub help: Option<String>,
    pub relocatable: Option<bool>,
    pub cachable: Option<bool>,
    pub commands_function: Option<String>,
    pub extra_fields: HashMap<String, String>,
    // New fields for complete rez compatibility
    pub base: Option<String>,
    pub hashed_variants: Option<bool>,
    pub has_plugins: Option<bool>,
    pub plugin_for: Vec<String>,
    pub format_version: Option<i32>,
    pub preprocess: Option<String>,
    // Function definitions for late binding
    pub functions: HashMap<String, String>,
}

impl PackageData {
    pub fn new() -> Self {
        Self::default()
    }
}
