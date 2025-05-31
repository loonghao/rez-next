//! Standalone test for Rex cache system
//!
//! This binary tests the Rex cache system independently

use rez_core_rex::cache::{RexCache, RexCacheConfig, EvictionStrategy};
use rez_core_rex::{RexCommand, ExecutionResult};
use std::collections::HashMap;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Rex Cache System Test ===\n");
    
    // Test 1: Basic cache operations
    test_basic_cache_operations().await?;
    
    // Test 2: Cache eviction
    test_cache_eviction().await?;
    
    // Test 3: Cache cleanup
    test_cache_cleanup().await?;
    
    // Test 4: Cache statistics
    test_cache_statistics().await?;
    
    // Test 5: Memory estimation
    test_memory_estimation().await?;
    
    println!("\n=== All Cache Tests Passed! ===");
    Ok(())
}

async fn test_basic_cache_operations() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Test 1: Basic Cache Operations ---");
    
    let cache = RexCache::new();
    
    // Test parse cache
    let command = RexCommand::SetEnv {
        name: "TEST_VAR".to_string(),
        value: "test_value".to_string(),
    };
    let line = "setenv TEST_VAR test_value";
    
    // Test cache miss
    let result = cache.get_parsed_command(line).await;
    assert!(result.is_none(), "Expected cache miss");
    
    // Test cache put and hit
    cache.put_parsed_command(line.to_string(), command.clone()).await;
    let result = cache.get_parsed_command(line).await;
    assert!(result.is_some(), "Expected cache hit");
    
    // Test execution cache
    let exec_result = ExecutionResult {
        success: true,
        output: vec!["test output".to_string()],
        errors: vec![],
        env_changes: HashMap::new(),
        execution_time_ms: 0,
    };
    let key = "test_execution_key";
    
    // Test cache miss
    let cached_result = cache.get_execution_result(key).await;
    assert!(cached_result.is_none(), "Expected cache miss");
    
    // Test cache put and hit
    cache.put_execution_result(key.to_string(), exec_result.clone()).await;
    let cached_result = cache.get_execution_result(key).await;
    assert!(cached_result.is_some(), "Expected cache hit");
    
    let stats = cache.get_stats().await;
    println!("Parse hits: {}, misses: {}", stats.parse_hits, stats.parse_misses);
    println!("Execution hits: {}, misses: {}", stats.execution_hits, stats.execution_misses);
    println!("✓ Basic cache operations test passed\n");
    
    Ok(())
}

async fn test_cache_eviction() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Test 2: Cache Eviction ---");
    
    let config = RexCacheConfig {
        max_parse_entries: 3,
        max_execution_entries: 2,
        eviction_strategy: EvictionStrategy::LRU,
        ..Default::default()
    };
    let cache = RexCache::with_config(config);
    
    // Fill parse cache beyond capacity
    for i in 0..5 {
        let line = format!("setenv TEST_VAR_{} value_{}", i, i);
        let command = RexCommand::SetEnv {
            name: format!("TEST_VAR_{}", i),
            value: format!("value_{}", i),
        };
        cache.put_parsed_command(line, command).await;
    }
    
    let stats = cache.get_stats().await;
    println!("Parse entries after eviction: {}", stats.parse_entries);
    println!("Total evictions: {}", stats.evictions);
    assert!(stats.parse_entries <= 3, "Parse cache should have evicted entries");
    assert!(stats.evictions > 0, "Should have performed evictions");
    
    println!("✓ Cache eviction test passed\n");
    Ok(())
}

async fn test_cache_cleanup() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Test 3: Cache Cleanup ---");
    
    let config = RexCacheConfig {
        parse_ttl: 1, // 1 second TTL
        execution_ttl: 1,
        ..Default::default()
    };
    let cache = RexCache::with_config(config);
    
    // Add entries
    let command = RexCommand::SetEnv {
        name: "TTL_TEST".to_string(),
        value: "test_value".to_string(),
    };
    let result = ExecutionResult {
        success: true,
        output: vec!["ttl test output".to_string()],
        errors: vec![],
        env_changes: HashMap::new(),
        execution_time_ms: 0,
    };
    
    cache.put_parsed_command("ttl_test_line".to_string(), command).await;
    cache.put_execution_result("ttl_test_key".to_string(), result).await;
    
    let stats_before = cache.get_stats().await;
    println!("Entries before TTL expiration - Parse: {}, Execution: {}", 
             stats_before.parse_entries, stats_before.execution_entries);
    
    // Wait for expiration
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Cleanup should remove expired entries
    cache.cleanup().await;
    
    let stats_after = cache.get_stats().await;
    println!("Entries after cleanup - Parse: {}, Execution: {}", 
             stats_after.parse_entries, stats_after.execution_entries);
    
    assert_eq!(stats_after.parse_entries, 0, "Parse cache should be empty after cleanup");
    assert_eq!(stats_after.execution_entries, 0, "Execution cache should be empty after cleanup");
    
    println!("✓ Cache cleanup test passed\n");
    Ok(())
}

async fn test_cache_statistics() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Test 4: Cache Statistics ---");
    
    let cache = RexCache::new();
    let command = RexCommand::SetEnv {
        name: "STATS_TEST".to_string(),
        value: "test_value".to_string(),
    };
    let line = "stats_test_line";
    
    // Add to cache
    cache.put_parsed_command(line.to_string(), command).await;
    
    // Generate hits and misses
    cache.get_parsed_command(line).await; // hit
    cache.get_parsed_command(line).await; // hit
    cache.get_parsed_command("missing_line").await; // miss
    
    let stats = cache.get_stats().await;
    println!("Parse hits: {}, misses: {}, hit rate: {:.2}", 
             stats.parse_hits, stats.parse_misses, stats.parse_hit_rate);
    
    assert_eq!(stats.parse_hits, 2, "Should have 2 hits");
    assert_eq!(stats.parse_misses, 1, "Should have 1 miss");
    
    let expected_hit_rate = 2.0 / 3.0;
    assert!((stats.parse_hit_rate - expected_hit_rate).abs() < 0.001, 
            "Hit rate should be approximately 0.667");
    
    println!("✓ Cache statistics test passed\n");
    Ok(())
}

async fn test_memory_estimation() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Test 5: Memory Estimation ---");
    
    let cache = RexCache::new();
    
    // Add some entries
    for i in 0..5 {
        let command = RexCommand::SetEnv {
            name: format!("MEM_TEST_{}", i),
            value: format!("value_{}", i),
        };
        let line = format!("setenv MEM_TEST_{} value_{}", i, i);
        cache.put_parsed_command(line, command).await;
    }
    
    let memory_usage = cache.estimate_memory_usage().await;
    let stats = cache.get_stats().await;
    
    println!("Estimated memory usage: {} bytes", memory_usage);
    println!("Stats memory usage: {} bytes", stats.memory_usage_bytes);
    
    assert!(memory_usage > 0, "Memory usage should be greater than 0");
    assert_eq!(stats.memory_usage_bytes, memory_usage, "Stats should match estimated memory");
    
    println!("✓ Memory estimation test passed\n");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cache_consistency() {
        let cache = RexCache::new();
        let command = RexCommand::SetEnv {
            name: "CONSISTENCY_TEST".to_string(),
            value: "test_value".to_string(),
        };
        let line = "consistency_test_line";
        
        // Add to cache
        cache.put_parsed_command(line.to_string(), command).await;
        
        // Multiple calls should return the same value
        let result1 = cache.get_parsed_command(line).await;
        let result2 = cache.get_parsed_command(line).await;
        let result3 = cache.get_parsed_command(line).await;
        
        assert!(result1.is_some());
        assert!(result2.is_some());
        assert!(result3.is_some());
        
        // Results should be identical
        let cmd1 = result1.unwrap();
        let cmd2 = result2.unwrap();
        let cmd3 = result3.unwrap();
        
        match (&cmd1, &cmd2, &cmd3) {
            (
                RexCommand::SetEnv { name: n1, value: v1 },
                RexCommand::SetEnv { name: n2, value: v2 },
                RexCommand::SetEnv { name: n3, value: v3 }
            ) => {
                assert_eq!(n1, n2);
                assert_eq!(n2, n3);
                assert_eq!(v1, v2);
                assert_eq!(v2, v3);
            }
            _ => panic!("Commands should be SetEnv variants"),
        }
    }
    
    #[tokio::test]
    async fn test_cache_clear() {
        let cache = RexCache::new();
        let command = RexCommand::SetEnv {
            name: "CLEAR_TEST".to_string(),
            value: "test_value".to_string(),
        };
        let result = ExecutionResult {
            success: true,
            output: vec!["clear test output".to_string()],
            errors: vec![],
            env_changes: HashMap::new(),
            execution_time_ms: 0,
        };
        
        // Add entries
        cache.put_parsed_command("clear_test_line".to_string(), command).await;
        cache.put_execution_result("clear_test_key".to_string(), result).await;
        
        // Verify entries exist
        let stats_before = cache.get_stats().await;
        assert!(stats_before.parse_entries > 0);
        assert!(stats_before.execution_entries > 0);
        
        // Clear cache
        cache.clear().await;
        
        // Verify cache is empty
        let stats_after = cache.get_stats().await;
        assert_eq!(stats_after.parse_entries, 0);
        assert_eq!(stats_after.execution_entries, 0);
        assert_eq!(stats_after.parse_hits, 0);
        assert_eq!(stats_after.parse_misses, 0);
    }
}
