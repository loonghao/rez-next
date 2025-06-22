use rez_core_version::Version;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Testing rez-core crates integration for rez-lsp-server");

    // Test 1: Version parsing and comparison
    println!("\n📦 Testing version management...");
    let v1 = Version::parse("1.2.3")?;
    let v2 = Version::parse("2.0.0-alpha.1")?;
    println!("✅ Parsed versions: {:?} and {:?}", v1, v2);
    println!("✅ Version comparison: {:?} < {:?} = {}", v1, v2, v1 < v2);

    // Test version serialization
    println!("\n🔄 Testing version serialization...");
    let json = serde_json::to_string_pretty(&v1)?;
    println!("✅ Version serialized to JSON: {}", json);

    let deserialized: Version = serde_json::from_str(&json)?;
    println!("✅ Version deserialized successfully: {:?}", deserialized);

    // Test version range parsing
    println!("\n📊 Testing version ranges...");
    let range_tests = vec![
        ">=1.0.0",
        "~1.2.3",
        "^2.0.0",
        "1.0.0-2.0.0",
    ];

    for range_str in range_tests {
        match Version::parse(range_str) {
            Ok(version) => println!("✅ Parsed range '{}' as version: {:?}", range_str, version),
            Err(e) => println!("⚠️  Range '{}' failed to parse as simple version: {}", range_str, e),
        }
    }

    println!("\n🎉 Basic integration tests passed! rez-lsp-server can use rez-core-version crate.");

    Ok(())
}
