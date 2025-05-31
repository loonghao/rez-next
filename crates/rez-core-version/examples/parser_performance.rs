//! Performance test for the state machine parser

use rez_core_version::parser::StateMachineParser;
use std::time::Instant;

fn main() {
    println!("🚀 Rez-Core Version Parser Performance Test");
    println!("============================================");

    let parser = StateMachineParser::new();
    
    // Test cases
    let test_versions = vec![
        "1.2.3",
        "1.2.3-alpha.1",
        "2.0.0-beta.2",
        "1.0.0-rc.1",
        "3.1.4-dev.123",
        "10.20.30",
        "1.2.3-alpha1.beta2.gamma3",
        "0.1.0",
        "1.0.0-alpha",
        "2.1.0-beta.1",
    ];

    println!("\n📊 Testing State Machine Parser Performance");
    println!("-------------------------------------------");

    // Warm up
    for _ in 0..1000 {
        for version_str in &test_versions {
            let _ = parser.parse_tokens(version_str);
        }
    }

    // Performance test
    let iterations = 10000;
    let start = Instant::now();
    
    for _ in 0..iterations {
        for version_str in &test_versions {
            match parser.parse_tokens(version_str) {
                Ok((tokens, separators)) => {
                    // Verify basic parsing worked
                    if tokens.is_empty() && !version_str.is_empty() {
                        eprintln!("❌ Failed to parse: {}", version_str);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error parsing '{}': {}", version_str, e);
                }
            }
        }
    }
    
    let duration = start.elapsed();
    let total_parses = iterations * test_versions.len();
    let parses_per_second = total_parses as f64 / duration.as_secs_f64();
    
    println!("✅ Completed {} parses in {:?}", total_parses, duration);
    println!("🎯 Performance: {:.0} parses/second", parses_per_second);
    println!("📈 Average time per parse: {:.2} μs", duration.as_micros() as f64 / total_parses as f64);

    // Test individual version parsing
    println!("\n🔍 Individual Version Analysis");
    println!("------------------------------");
    
    for version_str in &test_versions {
        let start = Instant::now();
        match parser.parse_tokens(version_str) {
            Ok((tokens, separators)) => {
                let duration = start.elapsed();
                println!("✅ '{}' -> {} tokens, {} separators ({:.2} μs)", 
                    version_str, tokens.len(), separators.len(), duration.as_micros());
                
                // Print token details
                for (i, token) in tokens.iter().enumerate() {
                    println!("   Token {}: {:?}", i, token);
                }
            }
            Err(e) => {
                println!("❌ '{}' -> Error: {}", version_str, e);
            }
        }
    }

    // Test error cases
    println!("\n🚨 Error Handling Test");
    println!("----------------------");
    
    let error_cases = vec![
        ".1.2.3",           // starts with separator
        "1.2.3.",           // ends with separator
        "1.2.3@",           // invalid character
        "_invalid",         // starts with underscore
        "invalid_",         // ends with underscore
        "",                 // empty string (should be OK)
        "1..2",             // double separator
        "not",              // reserved word
        "version",          // reserved word
    ];

    for error_case in &error_cases {
        match parser.parse_tokens(error_case) {
            Ok((tokens, separators)) => {
                if error_case.is_empty() {
                    println!("✅ '{}' -> {} tokens, {} separators (empty is OK)", 
                        error_case, tokens.len(), separators.len());
                } else {
                    println!("⚠️  '{}' -> {} tokens, {} separators (expected error)", 
                        error_case, tokens.len(), separators.len());
                }
            }
            Err(e) => {
                println!("✅ '{}' -> Error: {} (expected)", error_case, e);
            }
        }
    }

    println!("\n🎉 Performance test completed!");
    
    // Performance target check
    if parses_per_second > 5000.0 {
        println!("🎯 SUCCESS: Achieved target of >5000 parses/second!");
    } else {
        println!("⚠️  Target not met: {} < 5000 parses/second", parses_per_second as u64);
    }
}
