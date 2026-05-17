//! Package implementation

mod help;
mod methods;
pub mod requirement;
mod serialize;
pub mod test_runner;
pub mod types;

pub use help::{HelpSection, PackageHelp};
pub use requirement::PackageRequirement;
pub use test_runner::{
    PackageTestResults, PackageTestRunner, TestDefinition, TestResult, TestStatus,
};
pub use types::Package;

#[cfg(test)]
#[path = "tests.rs"]
mod package_tests;
