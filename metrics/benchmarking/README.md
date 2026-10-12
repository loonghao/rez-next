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
