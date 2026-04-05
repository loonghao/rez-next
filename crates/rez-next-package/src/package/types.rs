//! Package type definition.

use rez_next_version::Version;
use std::collections::HashMap;

/// High-performance package representation compatible with rez
#[derive(Debug, Clone)]
pub struct Package {
    /// Package name
    pub name: String,
    /// Package version
    pub version: Option<Version>,
    /// Package description
    pub description: Option<String>,
    /// Package authors
    pub authors: Vec<String>,
    /// Package requirements
    pub requires: Vec<String>,
    /// Build requirements
    pub build_requires: Vec<String>,
    /// Private build requirements
    pub private_build_requires: Vec<String>,
    /// Package variants
    pub variants: Vec<Vec<String>>,
    /// Package tools
    pub tools: Vec<String>,
    /// Package commands (rex script string, set from `def commands():` body)
    pub commands: Option<String>,
    /// Package commands function body (alias for commands; used by validation layer)
    pub commands_function: Option<String>,
    /// Build command for custom builds
    pub build_command: Option<String>,
    /// Build system type
    pub build_system: Option<String>,
    /// Pre commands (executed before main commands)
    pub pre_commands: Option<String>,
    /// Post commands (executed after main commands)
    pub post_commands: Option<String>,
    /// Pre test commands (executed before tests)
    pub pre_test_commands: Option<String>,
    /// Pre build commands (executed before build)
    pub pre_build_commands: Option<String>,
    /// Package tests
    pub tests: HashMap<String, String>,
    /// Required rez version
    pub requires_rez_version: Option<String>,
    /// Package UUID
    pub uuid: Option<String>,
    /// Package config
    pub config: HashMap<String, String>,
    /// Package help
    pub help: Option<String>,
    /// Package relocatable flag
    pub relocatable: Option<bool>,
    /// Package cachable flag
    pub cachable: Option<bool>,
    /// Package timestamp
    pub timestamp: Option<i64>,
    /// Package revision
    pub revision: Option<String>,
    /// Package changelog
    pub changelog: Option<String>,
    /// Package release message
    pub release_message: Option<String>,
    /// Previous version
    pub previous_version: Option<Version>,
    /// Previous revision
    pub previous_revision: Option<String>,
    /// VCS type
    pub vcs: Option<String>,
    /// Package format version
    pub format_version: Option<i32>,
    /// Package base
    pub base: Option<String>,
    /// Package has plugins
    pub has_plugins: Option<bool>,
    /// Plugin for packages
    pub plugin_for: Vec<String>,
    /// Package hashed variants
    pub hashed_variants: Option<bool>,
    /// Package preprocess function
    pub preprocess: Option<String>,
}
