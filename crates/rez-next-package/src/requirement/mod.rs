//! Package requirement parsing and handling.

mod display;
pub mod parser;
pub mod types;

pub use parser::RequirementParser;
pub use types::{
    EnvCondition, PlatformCondition, Requirement, VersionConstraint,
};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
