//! Repository scanner performance benchmark

use rez_core_repository::scanner::{RepositoryScanner, ScannerConfig};
use std::path::PathBuf;
use std::time::Instant;
use tokio::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Repository Scanner Performance Benchmark");
    println!("==========================================");

    // Create test directory structure
    let test_dir = create_test_repository().await?;
    println!("📁 Created test repository at: {}", test_dir.display());

    // Test configurations
    let configs = vec![
        ("Legacy Config", create_legacy_config()),
        ("Optimized Config", create_optimized_config()),
        ("High Performance Config", create_high_performance_config()),
    ];

    for (config_name, config) in configs {
        println!("\n🔧 Testing configuration: {}", config_name);
        println!("-----------------------------------");
        
        let scanner = RepositoryScanner::new(config);
        
        // Warm up
        println!("🔥 Warming up...");
        let _ = scanner.scan_repository(&test_dir).await?;
        
        // Performance test
        let iterations = 5;
        let mut total_duration = 0u64;
        let mut total_packages = 0usize;
        
        for i in 1..=iterations {
            scanner.clear_cache(); // Clear cache for fair comparison
            
            let start = Instant::now();
            let result = scanner.scan_repository(&test_dir).await?;
            let duration = start.elapsed();
            
            total_duration += duration.as_millis() as u64;
            total_packages = result.packages.len();
            
            println!("  Iteration {}: {}ms, {} packages, {} dirs, {} files", 
                i, 
                duration.as_millis(),
                result.packages.len(),
                result.directories_scanned,
                result.files_examined
            );
            
            // Print performance metrics
            let metrics = &result.performance_metrics;
            println!("    I/O: {}ms, Parse: {}ms, Cache hits: {}, Memory mapped: {}", 
                metrics.io_time_ms,
                metrics.parsing_time_ms,
                metrics.cache_hits,
                metrics.memory_mapped_files
            );
        }
        
        let avg_duration = total_duration / iterations as u64;
        let packages_per_second = if avg_duration > 0 {
            (total_packages as f64 * 1000.0) / avg_duration as f64
        } else {
            0.0
        };
        
        println!("📊 Average: {}ms, {:.1} packages/second", avg_duration, packages_per_second);
        println!("🎯 Cache size: {} entries", scanner.cache_size());
    }

    // Cleanup
    fs::remove_dir_all(&test_dir).await?;
    println!("\n🧹 Cleaned up test directory");
    
    println!("\n🎉 Benchmark completed!");
    Ok(())
}

async fn create_test_repository() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let test_dir = std::env::temp_dir().join("rez_scanner_test");
    
    // Remove existing test directory
    if test_dir.exists() {
        fs::remove_dir_all(&test_dir).await?;
    }
    
    fs::create_dir_all(&test_dir).await?;
    
    // Create test packages
    for i in 1..=100 {
        let package_dir = test_dir.join(format!("package_{:03}", i));
        fs::create_dir_all(&package_dir).await?;
        
        // Create different package file formats
        match i % 4 {
            0 => {
                // Python package
                let content = format!(r#"
name = "test_package_{}"
version = "1.{}.0"
description = "Test package for benchmarking"
authors = ["Test Author"]

def commands():
    import os
    env.PATH.prepend(os.path.join(this.root, "bin"))
"#, i, i % 10);
                fs::write(package_dir.join("package.py"), content).await?;
            }
            1 => {
                // YAML package
                let content = format!(r#"
name: test_package_{}
version: 1.{}.0
description: Test package for benchmarking
authors:
  - Test Author

commands: |
  export PATH=${{this.root}}/bin:$PATH
"#, i, i % 10);
                fs::write(package_dir.join("package.yaml"), content).await?;
            }
            2 => {
                // JSON package
                let content = format!(r#"{{
  "name": "test_package_{}",
  "version": "1.{}.0",
  "description": "Test package for benchmarking",
  "authors": ["Test Author"],
  "commands": "export PATH=${{this.root}}/bin:$PATH"
}}"#, i, i % 10);
                fs::write(package_dir.join("package.json"), content).await?;
            }
            _ => {
                // YML package
                let content = format!(r#"
name: test_package_{}
version: 1.{}.0
description: Test package for benchmarking
authors:
  - Test Author

commands: |
  export PATH=${{this.root}}/bin:$PATH
"#, i, i % 10);
                fs::write(package_dir.join("package.yml"), content).await?;
            }
        }
        
        // Create some subdirectories to test depth scanning
        if i % 10 == 0 {
            let subdir = package_dir.join("subdir");
            fs::create_dir_all(&subdir).await?;
            
            let sub_content = format!(r#"
name: sub_package_{}
version: 1.0.0
description: Sub package for testing
"#, i);
            fs::write(subdir.join("package.yaml"), sub_content).await?;
        }
    }
    
    // Create some directories that should be excluded
    let excluded_dirs = vec![".git", "__pycache__", "node_modules", ".vscode"];
    for excluded in excluded_dirs {
        let excluded_dir = test_dir.join(excluded);
        fs::create_dir_all(&excluded_dir).await?;
        fs::write(excluded_dir.join("package.py"), "# This should be excluded").await?;
    }
    
    Ok(test_dir)
}

fn create_legacy_config() -> ScannerConfig {
    ScannerConfig {
        max_concurrent_scans: 5,
        use_memory_mapping: false,
        enable_scan_cache: false,
        smart_file_detection: false,
        directory_batch_size: 10,
        ..Default::default()
    }
}

fn create_optimized_config() -> ScannerConfig {
    ScannerConfig {
        max_concurrent_scans: 15,
        use_memory_mapping: true,
        enable_scan_cache: true,
        smart_file_detection: true,
        directory_batch_size: 30,
        ..Default::default()
    }
}

fn create_high_performance_config() -> ScannerConfig {
    ScannerConfig {
        max_concurrent_scans: 25,
        use_memory_mapping: true,
        memory_mapping_threshold: 512, // Lower threshold
        enable_scan_cache: true,
        smart_file_detection: true,
        directory_batch_size: 50,
        max_cache_size_mb: 200,
        ..Default::default()
    }
}
