# Benchmark identity and comparison rules

Criterion stores baselines by benchmark ID under `target/criterion`, not by
benchmark executable. IDs therefore must be unique across executables run in
one suite. `package_benchmark` uses `package_full_*` groups;
`simple_package_benchmark` retains the existing `package_*` groups. Both use
the same sampling policy in `benches/support/package_criterion.rs`.

The output parser rejects duplicate IDs with exit code 2, even without
`--fail-on-regression`. This prevents ambiguous results from being published
or counted as distinct measurements. The renamed full-harness IDs start new
baselines; old artifacts are preserved and are not reclassified as evidence
of a code regression.

Run the lightweight identity and parser checks with:

```sh
python -m unittest discover -s metrics/benchmarking/scripts -p 'test_*.py' -v
```

A Criterion change flag compares measurements, not necessarily commits.
Claims about a code regression additionally require a known baseline commit,
the same harness/configuration, and comparable runner conditions. Shared
sampling settings alone do not establish those conditions.

## Result publication and concurrent main updates

The full measurement job saves artifacts before a separate publication job
attempts to update `main`. Artifact names include the measured full commit SHA,
workflow run, and attempt; earlier attempts are not overwritten. Raw full-suite
artifacts are also uploaded when a measurement or parsing step fails, when the
runner can still execute cleanup steps.

Publication is limited to pushes to `main` or manual runs on `main`. Manual
measurements on other branches cannot start the write-enabled publication job.

`publish_results.py` checks the measured SHA against both the checkout and the
saved report/summary. It publishes only when `main` still equals that SHA. If
`main` has advanced, including between the check and a normal push, the old run
keeps its artifacts and exits without modifying `main`. It never force-pushes or
rebases old measurements onto newer code. Other push/fetch errors remain errors.

Retry a failed **Publish Benchmark Results** job on its own; it downloads the
original measurement job's immutable result bundle by artifact ID. This does
not rerun the benchmarks. An obsolete run does not need a publication retry.
