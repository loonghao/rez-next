//! Shared sampling policy for both package benchmark executables.

use criterion::Criterion;
use std::time::Duration;

pub fn configured() -> Criterion {
    let quick = std::env::var("CRITERION_QUICK").is_ok();
    Criterion::default()
        .sample_size(if quick { 30 } else { 100 })
        .measurement_time(Duration::from_secs(if quick { 2 } else { 5 }))
        .warm_up_time(Duration::from_millis(if quick { 500 } else { 3000 }))
}
