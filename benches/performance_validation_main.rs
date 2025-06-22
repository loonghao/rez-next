//! Performance Validation Main Entry Point
//!
//! This is the main entry point for running performance validation benchmarks
//! to verify the claimed performance improvements (117x version parsing, 75x Rex parsing).

use criterion::{criterion_group, criterion_main, Criterion};

// Import all validation benchmark functions
mod performance_validation_benchmark;
use performance_validation_benchmark::*;

/// Configure Criterion for performance validation
fn configure_criterion() -> Criterion {
    Criterion::default()
        .measurement_time(std::time::Duration::from_secs(15))
        .sample_size(200)
        .warm_up_time(std::time::Duration::from_secs(3))
}

criterion_group! {
    name = performance_validation_benches;
    config = configure_criterion();
    targets = validate_version_parsing_117x,
              validate_rex_parsing_75x,
              comprehensive_performance_validation,
              stress_test_validation,
              memory_efficiency_validation,
              real_world_scenario_validation
}

criterion_main!(performance_validation_benches);
