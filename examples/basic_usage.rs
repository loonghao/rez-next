//! Basic usage example for rez-core
//!
//! Demonstrates version parsing/comparison, version ranges, and configuration.

use rez_core::common::{RezCoreConfig, RezCoreResult};
use rez_core::version::{Version, VersionRange};

fn main() -> RezCoreResult<()> {
    println!("Rez Core Basic Usage Example");
    println!("==============================");

    // Version parsing and comparison
    println!("\nVersion Examples:");
    let v1 = Version::parse("1.2.3")?;
    let v2 = Version::parse("2.0.0")?;

    println!("Version 1: {}", v1.as_str());
    println!("Version 2: {}", v2.as_str());
    println!("Comparison: v1 < v2 = {}", v1 < v2);

    // Version ranges
    println!("\nVersion Range Examples:");
    let range1 = VersionRange::parse("1.0.0..2.0.0")?;
    let range2 = VersionRange::parse("1.5.0+")?;

    println!("Range 1: {}", range1.as_str());
    println!("Range 2: {}", range2.as_str());

    // Configuration
    println!("\nConfiguration:");
    let config = RezCoreConfig::default();
    println!("Use Rust version: {}", config.use_rust_version);
    println!("Use Rust solver: {}", config.use_rust_solver);
    println!("Use Rust repository: {}", config.use_rust_repository);
    println!("Rust fallback enabled: {}", config.rust_fallback);
    println!("\nExample completed successfully!");

    Ok(())
}
