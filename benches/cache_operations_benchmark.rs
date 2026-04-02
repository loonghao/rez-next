//! Cache Operations Benchmark
//!
//! Benchmarks for rez-next-cache IntelligentCacheManager covering:
//! - Single get/put latency (L1 warm, L1 cold)
//! - Throughput under concurrent-style repeated access
//! - Cache eviction cost (capacity-constrained inserts)
//! - Batch insert throughput at various sizes

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rez_next_cache::{IntelligentCacheManager, L1CacheConfig, UnifiedCache, UnifiedCacheConfig};

/// Build a runtime + cache pair for async benchmarks
fn make_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

fn make_cache(capacity: usize) -> IntelligentCacheManager<String, String> {
    let config = UnifiedCacheConfig {
        l1_config: L1CacheConfig {
            max_entries: capacity,
            ..Default::default()
        },
        ..Default::default()
    };
    IntelligentCacheManager::new(config)
}

// ── Bench 1: single put latency ───────────────────────────────────────────────

fn bench_single_put(c: &mut Criterion) {
    let rt = make_runtime();
    let cache = make_cache(100_000);

    c.bench_function("cache/put_single", |b| {
        let mut counter: u64 = 0;
        b.iter(|| {
            let key = format!("key_{}", counter);
            let value = format!("value_{}", counter);
            counter += 1;
            rt.block_on(async {
                cache.put(key, value).await.unwrap();
            });
        });
    });
}

// ── Bench 2: single get (warm — key present in L1) ───────────────────────────

fn bench_single_get_warm(c: &mut Criterion) {
    let rt = make_runtime();
    let cache = make_cache(100_000);

    // Pre-populate 1 000 entries
    rt.block_on(async {
        for i in 0u32..1_000 {
            cache
                .put(format!("warm_{}", i), format!("v_{}", i))
                .await
                .unwrap();
        }
    });

    c.bench_function("cache/get_warm", |b| {
        let mut counter: u64 = 0;
        b.iter(|| {
            let key = format!("warm_{}", counter % 1_000);
            counter += 1;
            rt.block_on(async {
                let _ = std::hint::black_box(cache.get(&key).await);
            });
        });
    });
}

// ── Bench 3: single get (cold — key absent) ──────────────────────────────────

fn bench_single_get_cold(c: &mut Criterion) {
    let rt = make_runtime();
    let cache = make_cache(100_000);

    c.bench_function("cache/get_cold_miss", |b| {
        let mut counter: u64 = 0;
        b.iter(|| {
            let key = format!("absent_{}", counter);
            counter += 1;
            rt.block_on(async {
                let _ = std::hint::black_box(cache.get(&key).await);
            });
        });
    });
}

// ── Bench 4: batch insert throughput at various sizes ────────────────────────

fn bench_batch_insert(c: &mut Criterion) {
    let rt = make_runtime();
    let mut group = c.benchmark_group("cache/batch_insert");

    for size in [10usize, 100, 500, 1_000, 5_000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &n| {
            b.iter(|| {
                let cache = make_cache(n + 100);
                rt.block_on(async {
                    for i in 0..n {
                        cache
                            .put(format!("k_{}", i), format!("v_{}", i))
                            .await
                            .unwrap();
                    }
                });
            });
        });
    }
    group.finish();
}

// ── Bench 5: eviction cost — inserts beyond L1 capacity ──────────────────────

fn bench_eviction_cost(c: &mut Criterion) {
    let rt = make_runtime();
    let mut group = c.benchmark_group("cache/eviction_cost");

    for cap in [10usize, 50, 200] {
        let insert_count = cap * 3; // triple capacity to force evictions
        group.throughput(Throughput::Elements(insert_count as u64));
        group.bench_with_input(
            BenchmarkId::new("cap", cap),
            &(cap, insert_count),
            |b, &(c, n)| {
                b.iter(|| {
                    let cache = make_cache(c);
                    rt.block_on(async {
                        for i in 0..n {
                            cache
                                .put(format!("ek_{}", i), format!("ev_{}", i))
                                .await
                                .unwrap();
                        }
                    });
                });
            },
        );
    }
    group.finish();
}

// ── Bench 6: repeated access pattern (simulate solver hot path) ───────────────

fn bench_hot_path_access(c: &mut Criterion) {
    let rt = make_runtime();
    let cache = make_cache(100_000);

    // Simulate "package version list" values — larger payloads
    rt.block_on(async {
        let pkgs = ["python", "maya", "houdini", "nuke", "katana", "mari", "clarisse"];
        for pkg in &pkgs {
            let versions: Vec<String> = (0..20).map(|i| format!("{}.{}.0", i / 10 + 1, i % 10)).collect();
            let payload = versions.join(",");
            cache.put(pkg.to_string(), payload).await.unwrap();
        }
    });

    c.bench_function("cache/hot_path_pkg_lookup", |b| {
        let pkgs = ["python", "maya", "houdini", "nuke", "katana", "mari", "clarisse"];
        let mut counter: usize = 0;
        b.iter(|| {
            let key = pkgs[counter % pkgs.len()].to_string();
            counter += 1;
            rt.block_on(async {
                let _ = std::hint::black_box(cache.get(&key).await);
            });
        });
    });
}

// ── Bench 7: contains_key check overhead ────────────────────────────────────

fn bench_contains_key(c: &mut Criterion) {
    let rt = make_runtime();
    let cache = make_cache(100_000);

    rt.block_on(async {
        for i in 0u32..500 {
            cache.put(format!("ck_{}", i), "v".to_string()).await.unwrap();
        }
    });

    c.bench_function("cache/contains_key", |b| {
        let mut counter: u64 = 0;
        b.iter(|| {
            let key = format!("ck_{}", counter % 500);
            counter += 1;
            rt.block_on(async {
                let _ = std::hint::black_box(cache.contains_key(&key).await);
            });
        });
    });
}

criterion_group!(
    cache_benches,
    bench_single_put,
    bench_single_get_warm,
    bench_single_get_cold,
    bench_batch_insert,
    bench_eviction_cost,
    bench_hot_path_access,
    bench_contains_key,
);
criterion_main!(cache_benches);
