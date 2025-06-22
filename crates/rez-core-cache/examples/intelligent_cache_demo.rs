//! Intelligent Cache System Demo
//!
//! This example demonstrates the key features of the intelligent cache system:
//! - Multi-level caching (L1 memory + L2 disk)
//! - Predictive preheating
//! - Adaptive tuning
//! - Performance monitoring

use rez_core_cache::{
    benchmarks::{run_comprehensive_benchmarks, BenchmarkConfig, CacheBenchmarkSuite},
    IntelligentCacheManager, UnifiedCache, UnifiedCacheConfig,
};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧠 Intelligent Cache System Demo");
    println!("================================\n");

    // Demo 1: Basic cache operations
    demo_basic_operations().await?;

    // Demo 2: Multi-level caching
    demo_multilevel_caching().await?;

    // Demo 3: Predictive preheating
    demo_predictive_preheating().await?;

    // Demo 4: Adaptive tuning
    demo_adaptive_tuning().await?;

    // Demo 5: Performance monitoring
    demo_performance_monitoring().await?;

    // Demo 6: Comprehensive benchmarks
    demo_comprehensive_benchmarks().await?;

    println!("\n✅ All demos completed successfully!");
    Ok(())
}

/// Demo 1: Basic cache operations
async fn demo_basic_operations() -> Result<(), Box<dyn std::error::Error>> {
    println!("📦 Demo 1: Basic Cache Operations");
    println!("---------------------------------");

    // Create cache with default configuration
    let config = UnifiedCacheConfig::default();
    let cache = IntelligentCacheManager::<String, String>::new(config);

    // Put some data
    cache.put("key1".to_string(), "value1".to_string()).await?;
    cache.put("key2".to_string(), "value2".to_string()).await?;
    cache.put("key3".to_string(), "value3".to_string()).await?;

    // Get data
    if let Some(value) = cache.get(&"key1".to_string()).await {
        println!("✓ Retrieved: key1 = {}", value);
    }

    // Check cache statistics
    let stats = cache.get_stats().await;
    println!("📊 Cache Stats:");
    println!("  L1 entries: {}", stats.l1_stats.entries);
    println!("  L1 hit rate: {:.2}%", stats.l1_stats.hit_rate * 100.0);
    println!(
        "  Overall hit rate: {:.2}%",
        stats.overall_stats.overall_hit_rate * 100.0
    );

    println!("✅ Basic operations demo completed\n");
    Ok(())
}

/// Demo 2: Multi-level caching behavior
async fn demo_multilevel_caching() -> Result<(), Box<dyn std::error::Error>> {
    println!("🏗️  Demo 2: Multi-level Caching");
    println!("------------------------------");

    let config = UnifiedCacheConfig {
        l1_config: rez_core_cache::L1CacheConfig {
            max_entries: 5, // Small L1 to trigger L2 usage
            promotion_threshold: 2,
            ..Default::default()
        },
        ..Default::default()
    };

    let cache = IntelligentCacheManager::<String, String>::new(config);

    // Fill L1 cache beyond capacity
    for i in 0..10 {
        let key = format!("item_{}", i);
        let value = format!("value_{}", i);
        cache.put(key, value).await?;
    }

    println!("📊 After filling cache:");
    let stats = cache.get_stats().await;
    println!("  L1 entries: {}", stats.l1_stats.entries);
    println!("  L2 entries: {}", stats.l2_stats.entries);
    println!("  Promotions: {}", stats.overall_stats.promotions);
    println!("  Demotions: {}", stats.overall_stats.demotions);

    // Access some items multiple times to trigger promotion
    for _ in 0..3 {
        let _ = cache.get(&"item_1".to_string()).await;
        let _ = cache.get(&"item_2".to_string()).await;
    }

    println!("📊 After repeated access:");
    let stats = cache.get_stats().await;
    println!("  L1 entries: {}", stats.l1_stats.entries);
    println!("  L2 entries: {}", stats.l2_stats.entries);
    println!("  Promotions: {}", stats.overall_stats.promotions);

    println!("✅ Multi-level caching demo completed\n");
    Ok(())
}

/// Demo 3: Predictive preheating
async fn demo_predictive_preheating() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔮 Demo 3: Predictive Preheating");
    println!("--------------------------------");

    let config = UnifiedCacheConfig {
        preheating_config: rez_core_cache::PreheatingConfig {
            enable_predictive_preheating: true,
            enable_pattern_learning: true,
            min_confidence_threshold: 0.5,
            ..Default::default()
        },
        ..Default::default()
    };

    let cache = IntelligentCacheManager::<String, String>::new(config);

    // Create predictable access patterns
    println!("🎯 Creating access patterns...");
    for cycle in 0..5 {
        for i in 0..10 {
            if i % 3 == cycle % 3 {
                let key = format!("pattern_key_{}", i);
                let value = format!("pattern_value_{}", i);

                if cache.get(&key).await.is_none() {
                    cache.put(key.clone(), value).await?;
                }

                // Record access for pattern learning
                cache.preheater().record_access(&key).await;
            }
        }
        sleep(Duration::from_millis(100)).await;
    }

    // Check preheating statistics
    let preheating_stats = cache.preheater().get_stats();
    println!("📊 Preheating Stats:");
    println!("  Patterns learned: {}", preheating_stats.patterns_learned);
    println!("  Predictions made: {}", preheating_stats.predictions_made);
    println!(
        "  Average confidence: {:.2}",
        preheating_stats.avg_confidence
    );

    // Get preheating recommendations
    let recommendations = cache.preheater().get_preheat_recommendations().await;
    println!("🎯 Preheating recommendations: {}", recommendations.len());
    for (key, score) in recommendations.iter().take(3) {
        println!("  {} (score: {:.3})", key, score);
    }

    println!("✅ Predictive preheating demo completed\n");
    Ok(())
}

/// Demo 4: Adaptive tuning
async fn demo_adaptive_tuning() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚙️  Demo 4: Adaptive Tuning");
    println!("---------------------------");

    let config = UnifiedCacheConfig {
        tuning_config: rez_core_cache::TuningConfig {
            enable_adaptive_tuning: true,
            min_samples_for_tuning: 5,
            target_hit_rate: 0.9,
            ..Default::default()
        },
        ..Default::default()
    };

    let cache = IntelligentCacheManager::<String, String>::new(config);

    // Simulate workload with poor hit rate
    println!("📉 Simulating poor hit rate workload...");
    for i in 0..50 {
        let key = format!("random_key_{}", i);
        let value = format!("random_value_{}", i);

        // Always miss, then put
        let _ = cache.get(&key).await;
        cache.put(key, value).await?;
    }

    // Record performance for tuning
    let stats = cache.get_stats().await;
    cache.tuner().record_performance(&stats).await;

    // Trigger tuning analysis
    let recommendations = cache.tuner().analyze_and_tune().await;
    println!("🎯 Tuning recommendations: {}", recommendations.len());
    for rec in recommendations.iter().take(3) {
        println!(
            "  {}: {:.3} -> {:.3} (confidence: {:.2})",
            rec.parameter, rec.current_value, rec.recommended_value, rec.confidence
        );
    }

    // Check tuning statistics
    let tuning_stats = cache.tuner().get_stats();
    println!("📊 Tuning Stats:");
    println!("  Operations: {}", tuning_stats.tuning_operations);
    println!("  Success rate: {:.2}%", tuning_stats.success_rate * 100.0);
    println!(
        "  Current confidence: {:.2}",
        tuning_stats.current_confidence
    );

    println!("✅ Adaptive tuning demo completed\n");
    Ok(())
}

/// Demo 5: Performance monitoring
async fn demo_performance_monitoring() -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Demo 5: Performance Monitoring");
    println!("----------------------------------");

    let config = UnifiedCacheConfig {
        monitoring_config: rez_core_cache::MonitoringConfig {
            enable_detailed_stats: true,
            enable_performance_metrics: true,
            enable_event_logging: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let cache = IntelligentCacheManager::<String, Vec<u8>>::new(config);
    let monitor = cache.monitor();

    // Perform various operations
    println!("🏃 Performing monitored operations...");
    for i in 0..100 {
        let key = format!("perf_key_{}", i);
        let value = vec![i as u8; 1024]; // 1KB values

        cache.put(key.clone(), value).await?;

        if i % 3 == 0 {
            let _ = cache.get(&key).await;
        }
    }

    // Get performance metrics
    let metrics = monitor.get_performance_metrics().await;
    println!("📊 Performance Metrics:");
    println!("  Avg GET latency: {:.2}μs", metrics.avg_get_latency_us);
    println!("  Avg PUT latency: {:.2}μs", metrics.avg_put_latency_us);
    println!("  Operations/sec: {:.2}", metrics.ops_per_second);
    println!("  Peak memory: {} bytes", metrics.peak_memory_usage);

    // Get recent events
    let events = monitor.get_recent_events(5).await;
    println!("📝 Recent events: {}", events.len());
    for event in events.iter().take(3) {
        println!("  {:?}: {}μs", event.event_type, event.latency_us);
    }

    println!("✅ Performance monitoring demo completed\n");
    Ok(())
}

/// Demo 6: Comprehensive benchmarks
async fn demo_comprehensive_benchmarks() -> Result<(), Box<dyn std::error::Error>> {
    println!("🏁 Demo 6: Comprehensive Benchmarks");
    println!("------------------------------------");

    // Run a subset of benchmarks for demo
    let config = UnifiedCacheConfig::high_performance();
    let bench_config = BenchmarkConfig {
        operations_count: 10_000, // Smaller for demo
        worker_count: 2,
        key_space_size: 1_000,
        value_size: 512,
        ..Default::default()
    };

    let suite = CacheBenchmarkSuite::new(config, bench_config);

    println!("🚀 Running benchmark: Sequential Operations");
    let result = suite.benchmark_sequential_operations().await;
    println!(
        "  Result: {:.2} ops/sec, {:.2}μs avg latency",
        result.ops_per_second, result.avg_latency_us
    );

    println!("🚀 Running benchmark: Concurrent Operations");
    let result = suite.benchmark_concurrent_operations().await;
    println!(
        "  Result: {:.2} ops/sec, {:.2}μs avg latency",
        result.ops_per_second, result.avg_latency_us
    );

    println!("🚀 Running benchmark: Hit Rate Optimization");
    let result = suite.benchmark_hit_rate_optimization().await;
    println!(
        "  Result: {:.2} ops/sec, hit rate: {:.2}%",
        result.ops_per_second,
        result.hit_rate * 100.0
    );

    // Show final cache statistics
    let stats = suite.get_cache_stats().await;
    println!("📊 Final Cache Statistics:");
    println!(
        "  Overall hit rate: {:.2}%",
        stats.overall_stats.overall_hit_rate * 100.0
    );
    println!(
        "  Efficiency score: {:.2}",
        stats.overall_stats.efficiency_score
    );
    println!("  Total entries: {}", stats.overall_stats.total_entries);

    println!("✅ Comprehensive benchmarks demo completed\n");
    Ok(())
}

/// Helper function to format bytes
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}
