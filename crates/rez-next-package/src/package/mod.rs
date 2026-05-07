//! Package implementation

mod methods;
pub mod requirement;
mod serialize;
pub mod test_runner;
pub mod types;
mod help;

pub use requirement::PackageRequirement;
pub use test_runner::{
    PackageTestResults, PackageTestRunner, TestDefinition, TestResult, TestStatus,
};
pub use types::Package;
pub use help::{HelpSection, PackageHelp};

#[cfg(test)]
#[path = "tests.rs"]
mod package_tests;
