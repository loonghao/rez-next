//! # Rez Core Build
//!
//! Build system for Rez Core.
//!
//! This crate provides:
//! - Build system abstraction and implementation
//! - Build process management and execution
//! - Build environment setup and configuration
//! - Build artifact management
//! - Release workflow orchestration

mod artifacts;
mod builder;
mod environment;
mod process;
pub mod release;
mod sources;
mod systems;
mod tests;
pub mod vcs;

pub use artifacts::*;
pub use builder::*;
pub use environment::*;
pub use process::*;
pub use release::*;
pub use sources::*;
pub use systems::*;
pub use vcs::*;

/// Build type enumeration (local or central build)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildType {
    /// Local build (installed to local packages path)
    Local,
    /// Central build (installed to central release repository)
    Central,
}

/// Error type for BuildType parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildTypeParseError;

impl std::fmt::Display for BuildTypeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid build type: expected 'local' or 'central'")
    }
}

impl std::str::FromStr for BuildType {
    type Err = BuildTypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "local" => Ok(BuildType::Local),
            "central" => Ok(BuildType::Central),
            _ => Err(BuildTypeParseError),
        }
    }
}

impl BuildType {
    /// Get the name of the build type
    pub fn name(&self) -> &'static str {
        match self {
            BuildType::Local => "local",
            BuildType::Central => "central",
        }
    }

    /// Create from string (returns Option for convenience)
    pub fn from_str_opt(s: &str) -> Option<Self> {
        s.parse::<BuildType>().ok()
    }
}

/// Get all available build system types
pub fn get_buildsys_types() -> Vec<&'static str> {
    systems::get_buildsys_types()
}

/// Get all available build process types
pub fn get_build_process_types() -> Vec<&'static str> {
    vec!["local", "central"]
}

/// Create a build system by type name
pub fn create_build_system(system_type: &str) -> Option<BuildSystem> {
    match system_type {
        "cmake" => Some(BuildSystem::CMake(systems::CMakeBuildSystem::new())),
        "make" => Some(BuildSystem::Make(systems::MakeBuildSystem::new())),
        "python" => Some(BuildSystem::Python(systems::PythonBuildSystem::new())),
        "nodejs" => Some(BuildSystem::NodeJs(systems::NodeJsBuildSystem::new())),
        "cargo" => Some(BuildSystem::Cargo(systems::CargoBuildSystem::new())),
        "custom" => Some(BuildSystem::Custom(systems::CustomBuildSystem::new("default".to_string()))),
        _ => None,
    }
}
