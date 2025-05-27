//! Basic usage example for rez-core
//!
//! This example demonstrates the basic functionality of rez-core components.
//! Note: This is a placeholder implementation for the MVP.

use rez_core::common::{RezCoreConfig, RezCoreResult};
use rez_core::version::{Version, VersionRange};

fn main() -> RezCoreResult<()> {
    println!("🦀 Rez Core Basic Usage Example");
    println!("================================");

    // Create and display versions
    println!("\n📦 Version Examples:");
    let v1 = Version::new("1.2.3")?;
    let v2 = Version::new("2.0.0")?;

    println!("Version 1: {}", v1.as_str());
    println!("Version 2: {}", v2.as_str());

    // Compare versions (placeholder implementation)
    println!("Comparison: v1 < v2 = {}", v1 < v2);

    // Create version ranges
    println!("\n📊 Version Range Examples:");
    let range1 = VersionRange::new("1.0.0..2.0.0")?;
    let range2 = VersionRange::new("1.5.0+")?;

    println!("Range 1: {}", range1.as_str());
    println!("Range 2: {}", range2.as_str());

    // Test version tokens
    println!("\n🔤 Version Token Examples:");
    let numeric_token = VersionToken::from_str("123");
    let alpha_token = VersionToken::from_str("alpha");

    println!("Numeric token: {}", numeric_token.as_str());
    println!("Alpha token: {}", alpha_token.as_str());
    println!(
        "Token comparison: numeric < alpha = {}",
        numeric_token < alpha_token
    );

    // Show configuration
    println!("\n⚙️ Configuration:");
    let config = RezCoreConfig::default();
    println!("Use Rust version: {}", config.use_rust_version);
    println!("Use Rust solver: {}", config.use_rust_solver);
    println!("Use Rust repository: {}", config.use_rust_repository);
    println!("Rust fallback enabled: {}", config.rust_fallback);

    println!("\n✅ Example completed successfully!");
    println!("⚠️  Note: This is a placeholder implementation for the MVP.");
    println!("   Full functionality will be implemented in future phases.");

    Ok(())
}
