//! Package implementation

mod methods;
pub mod requirement;
mod serialize;
pub mod types;

pub use requirement::PackageRequirement;
pub use types::Package;

#[cfg(test)]
#[path = "tests.rs"]
mod package_tests;
