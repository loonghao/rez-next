//! Simple Demo of Intelligent Cache System
//!
//! This example demonstrates basic usage of the intelligent cache system.

use rez_core_cache::{IntelligentCacheManager, UnifiedCache, UnifiedCacheConfig};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧠 Simple Intelligent Cache Demo");
    println!("=================================\n");

    // Create cache with high-performance configuration
    let config = UnifiedCacheConfig::high_performance();
    let cache = IntelligentCacheManager::<String, String>::new(config);

    println!("📦 Basic Cache Operations:");

    // Put some data
    cache.put("user:1".to_string(), "Alice".to_string()).await?;
    cache.put("user:2".to_string(), "Bob".to_string()).await?;
    cache
        .put("user:3".to_string(), "Charlie".to_string())
        .await?;

    println!("✓ Stored 3 users in cache");

    // Get data
    if let Some(user) = cache.get(&"user:1".to_string()).await {
        println!("✓ Retrieved user:1 = {}", user);
    }

    // Access pattern to trigger learning
    println!("\n🎯 Creating Access Patterns:");
    for i in 0..5 {
        let _ = cache.get(&"user:1".to_string()).await; // Frequent access
        if i % 2 == 0 {
            let _ = cache.get(&"user:2".to_string()).await; // Less frequent
        }
        sleep(Duration::from_millis(10)).await;
    }
    println!("✓ Created access patterns for predictive learning");

    // Check cache statistics
    println!("\n📊 Cache Statistics:");
    let stats = cache.get_stats().await;
    println!("  L1 entries: {}", stats.l1_stats.entries);
    println!("  L1 hit rate: {:.2}%", stats.l1_stats.hit_rate * 100.0);
    println!("  L2 entries: {}", stats.l2_stats.entries);
    println!(
        "  Overall hit rate: {:.2}%",
        stats.overall_stats.overall_hit_rate * 100.0
    );
    println!(
        "  Efficiency score: {:.2}",
        stats.overall_stats.efficiency_score
    );

    // Performance metrics
    println!("\n⚡ Performance Metrics:");
    let metrics = cache.monitor().get_performance_metrics().await;
    println!("  Average GET latency: {:.2}μs", metrics.avg_get_latency_us);
    println!("  Average PUT latency: {:.2}μs", metrics.avg_put_latency_us);
    println!("  Operations per second: {:.2}", metrics.ops_per_second);

    // Preheating statistics
    println!("\n🔮 Predictive Preheating:");
    let preheating_stats = cache.preheater().get_stats();
    println!("  Patterns learned: {}", preheating_stats.patterns_learned);
    println!("  Predictions made: {}", preheating_stats.predictions_made);
    println!(
        "  Average confidence: {:.2}",
        preheating_stats.avg_confidence
    );

    // Adaptive tuning
    println!("\n⚙️ Adaptive Tuning:");
    let tuning_stats = cache.tuner().get_stats();
    println!("  Tuning operations: {}", tuning_stats.tuning_operations);
    println!("  Success rate: {:.2}%", tuning_stats.success_rate * 100.0);
    println!(
        "  Current confidence: {:.2}",
        tuning_stats.current_confidence
    );

    // Test cache capacity and eviction
    println!("\n🗂️ Testing Cache Capacity:");
    for i in 0..20 {
        let key = format!("temp_key_{}", i);
        let value = format!("temp_value_{}", i);
        cache.put(key, value).await?;
    }

    let final_stats = cache.get_stats().await;
    println!(
        "  Final total entries: {}",
        final_stats.overall_stats.total_entries
    );
    println!("  Promotions: {}", final_stats.overall_stats.promotions);
    println!("  Demotions: {}", final_stats.overall_stats.demotions);

    println!("\n✅ Demo completed successfully!");
    println!("\n📋 Summary:");
    println!("{}", final_stats.summary_report());

    Ok(())
}
