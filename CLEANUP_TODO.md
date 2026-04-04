# Cleanup TODO

## High Priority — Structural Refactoring

### 1. `python-bindings` feature cleanup
- **Status**: COMPLETE ✓
- **Impact**: Originally 119+ `#[cfg(feature = "python-bindings")]` blocks across 10+ crates. ~2400 lines removed total across 7 cycles.
- **Root cause**: Python bindings migrated to `rez-next-python` crate, but old per-crate `#[cfg(feature = "python-bindings")]` code was left behind. The feature was never defined in any `Cargo.toml`, and `pyo3` was not a dependency in non-python crates.
- **Verification**: `grep -r 'cfg.*python.bindings' crates/ --include='*.rs'` returns 0 results (excluding `rez-next-python/`)
- **Note**: `version_token.rs` and `token.rs` have been deleted in cycle 8 — they were dead files not in the module tree.

### 2. Workspace lint configuration tightening
- **Status**: COMPLETE ✓ (cycle 12)
- All Rust lints tightened to `warn` level: `unexpected_cfgs`, `unused_imports`, `dead_code`, `unused_variables`, `unused_mut`, `deprecated`, `ambiguous_glob_reexports`, `irrefutable_let_patterns`
- All 30 clippy `allow` rules removed (zero instances in codebase) — clippy defaults now enforced
- Only category-level clippy config remains: `complexity=warn`, `correctness=deny`, `suspicious=deny`, `perf=warn`

### 3. Duplicate `ResolutionResult` types
- **Status**: COMPLETE ✓
- Removed duplicate `ResolutionResult` from `solver.rs` (exact copy of `resolution.rs`)
- `solver.rs` now imports `crate::resolution::ResolutionResult`
- Renamed `dependency_resolver::ResolutionResult` to `DetailedResolutionResult` (different schema)
- CLI `solve.rs` updated to use `DetailedResolutionResult`

### 4. `#[allow(dead_code)]` helper functions (5 in exceptions_bindings.rs)
- **Status**: COMPLETE ✓ (cycle 16)
- Removed all 5 `raise_*` functions and their 6 unit tests — none were called outside the file
- Exception types remain available via `create_exception!` macro (Python `raise rez.ResolveError(...)` works directly)

### 5. Orphan pyo3 files in non-python crates
- **Status**: COMPLETE ✓
- Deleted `version_token.rs` (371 lines), `token.rs` (123 lines), `validation.rs` (1034 lines), `management.rs` (1077 lines), `version_token_tests.rs` (6 lines)
- None were in lib.rs module trees, none were compiled, pyo3 was not a dependency of these crates
- rez-next-python does not reference any types from these files

### 6. Dead .rs files in rez-next-package not in module tree
- **Status**: COMPLETE ✓ (cycle 9)
- Deleted `batch.rs`, `cache.rs`, `dependency.rs`, `variant.rs` — all dead files not in lib.rs module tree

### 7. Further lint tightening
- **Status**: COMPLETE ✓ (cycle 12)
- `unused_imports`: `allow` → `warn` + 68 imports cleaned (cycle 9)
- `dead_code`: `allow` → `warn` + ~430 lines dead code removed (cycle 10)
- `unused_variables`: `allow` → `warn` + 24 function-signature warnings fixed (cycle 11)
- `unused_mut`: `allow` → `warn` (cycle 11, zero instances found)
- `deprecated`: `allow` → `warn` + fixed `base64::decode`/`encode` deprecated API (cycle 12)
- `ambiguous_glob_reexports`: `allow` → `warn` + fixed `RepositoryManager` glob conflict (cycle 12)
- `irrefutable_let_patterns`: `allow` → `warn` + fixed scanner.rs `if let` pattern (cycle 12)
- 30 clippy allow rules removed — all had zero instances (cycle 12)

### 8. Dead `repository::RepositoryManager` type
- **Status**: COMPLETE ✓ (cycle 14)
- Renamed to `AsyncRepositoryManager` in cycle 13 (upstream rename for clarity)
- `AsyncRepositoryManager` struct deleted in cycle 14 (~220 lines removed)
- `deduplicate_packages` extracted as public free function in `repository.rs`
- Exported via `lib.rs` as `rez_next_repository::deduplicate_packages`
- All 8 tests updated to call free function directly (removed `test_repository_manager_initial_count_is_zero`)

## Medium Priority — TODO Audit

1 TODO comment across the codebase (cycle 20 audit, unchanged from cycle 19):
- **CLI stubs** (1): `view.rs` (1, context package viewing)
- The remaining TODO is a non-blocking stub implementation for future features.

### 14. Disabled benchmark files removal
- **Status**: COMPLETE ✓ (cycle 20)
- Deleted 13 disabled benchmark files (~7400 lines, ~220KB): build_cache_benchmark, comprehensive_benchmark_suite, solver_benchmark, context_benchmark, simple_*_benchmark, performance_validation_*
- These files were not in Cargo.toml `[[bench]]` entries and referenced deleted/renamed types (would not compile)
- Updated `benches/README.md` to remove "Disabled" section

### 15. Mock simulation tests removal
- **Status**: COMPLETE ✓ (cycle 20)
- Deleted `tests/integration/test_performance_optimizations.rs` (315 lines) — not in module tree, 0 project imports, all tests were `format!()` string operations
- Deleted 5 mock simulation tests from `tests/integration_tests.rs::performance_tests` module — same pattern, no actual project code tested

### 16. eprintln in library code — needs tracing dependency
- **Status**: COMPLETE ✓ (cycle 35 / iteration agent)
- Added `tracing = "0.1"` to workspace dependencies and as a direct dep to `rez-next-cache` and `rez-next-repository`
- Replaced 3 library-code `eprintln!` calls with `tracing::warn!`:
  - `intelligent_manager.rs:391` — L1 cache promotion failure
  - `filesystem.rs:404` — package load failure during repo scan
  - `scanner.rs:378` — path preload failure
- `eprintln!` calls in `bin/` and `examples/` are intentional CLI/demo output and remain unchanged

### 17. `pyo3` version drift between workspace and `rez-next-python`
- **Status**: COMPLETE ✓ (cycle 22)
- Previous cycle-21 note was stale: root `Cargo.toml` and `crates/rez-next-python/Cargo.toml` currently both pin `pyo3 = 0.25`
- No active workspace-vs-crate drift remains to clean up; this item is closed as an outdated cleanup record rather than a dependency change
- Future `pyo3` upgrades should be handled as normal dependency work with wheel/build validation, not as existing cleanup debt

### 18. Platform mismatch solver test has weak assertion
- **Status**: COMPLETE ✓ (cycle 37)
- `test_solver_platform_mismatch_fails_or_empty` renamed and split into two tests:
  - `test_solver_platform_mismatch_lenient_records_failure`: asserts `maya_linux` not cleanly resolved without failed_requirements
  - `test_solver_platform_mismatch_strict_returns_err`: asserts strict mode returns Err
- Both tests carry observable contract assertions instead of `let _ = ...`

### 19. Split solver test files still duplicate repository/runtime helpers
- **Status**: COMPLETE ✓ (cycle 36)
- Extracted `build_test_repo` into `tests/solver_helpers.rs`; all four solver test files now use `#[path = "solver_helpers.rs"] mod solver_helpers` — no drift after future test splits

### 20. Cargo.lock policy note no longer matches repository state
- **Status**: COMPLETE ✓ (cycle 24)
- `.gitignore` no longer claims that `Cargo.lock` is tracked for reproducible binary builds
- Current repository policy is now documented accurately: the workspace does **not** currently track a root `Cargo.lock`

### 21. Additional vacuous compatibility assertions remain in tests
- **Status**: COMPLETE ✓ (cycle 37)
- Replaced `let _ = result` / `let _ = r.resolved_packages` style vacuous assertions across 5 test files:
  - `rez_solver_platform_tests.rs`: mismatch + conflict Ok branches
  - `rez_solver_edge_case_tests.rs`: conflicting transitive requirements Ok branch
  - `rez_solver_graph_tests.rs`: strict mode Ok fallback branch
  - `rez_compat_misc_tests.rs`: version conflict empty repo + large version component
  - `rez_compat_solver_tests.rs`: empty repo single requirement
- Each replaced assertion now verifies an observable contract (resolved count, failed_requirements presence, version prefix)

### 22. Alpha token ordering not rez-compatible
- **Status**: COMPLETE ✓ (cycle 38)
- rez spec: alpha tokens sort *less than* numeric tokens — `1.0.alpha < 1.0.0`
- Fixed `compare_single_token` in `rez-next-version/src/version.rs`:
  - Added fast paths for purely alpha vs purely numeric tokens (alpha → `Less`, numeric → `Greater`)
  - Updated segment-by-segment comparison to use `(false, true) => Less` / `(true, false) => Greater` when one segment is alpha and the other numeric
- Updated `test_version_alphanumeric_ordering` in `rez_compat_late_tests.rs`: removed TODO placeholder, added real assertion `va < vz`
- Updated `test_version_prerelease_less_than_release` in `version_tests.rs`: added `assert!(pre < rel)`
- All 125 version crate tests + full test suite (~715 tests) pass

### 23. Large mixed-responsibility files remain in CLI and build/parser modules
- **Status**: TODO (cycle 24)
- `src/cli/commands/bind.rs`, `crates/rez-next-build/src/systems.rs`, `crates/rez-next-package/src/python_ast_parser.rs`, `src/cli/commands/search_v2.rs`, and `src/cli/commands/pkg_cache.rs` are still ~800-1300 lines and mix orchestration with parsing/formatting/IO
- Follow-up: split by responsibility before adding more behavior to these files

### 24. CLI helper logic is still duplicated across commands
- **Status**: TODO (cycle 24)
- Home-path expansion is duplicated across `bind.rs`, `build.rs`, `cp.rs`, `mv.rs`, `rm.rs`, `search_v2.rs`, `status.rs`, `test.rs`, `view.rs`, and others
- Time-filter parsing is duplicated between `search_v2.rs` and `rm.rs`
- Follow-up: extract shared CLI helpers for path expansion and timestamp parsing

### 25. Public compatibility stubs still need explicit product decisions
- **Status**: PARTIAL (cycle 39)
- `get_pip_dependencies()` — **FIXED**: now raises `NotImplementedError` instead of returning empty list silently (cycle 39)
- `pip_install()` — **FIXED**: now raises `NotImplementedError` instead of fake-installing packages (cycle 39)
- `optimized_solver.rs` — **DELETED**: dead file not in module tree, `detect_conflicts_optimized()` was only reachable via this dead code (cycle 39)
- `crates/rez-next-build/src/systems.rs` still contains placeholder stub implementations for `PythonBuildSystem`, `NodeJsBuildSystem`, `CargoBuildSystem` — these call external tools (python, npm, cargo) but the invocation logic is incomplete / not tested
- Follow-up: add integration tests for build system detection and execution





- **Status**: COMPLETE ✓ (cycle 19)
- Fixed `handle_grouped_command` in `rez-next.rs`: clap returns `Err` for `--help`/`--version` display; now uses `e.use_stderr()` to decide exit code (0 for help/version, 1 for real errors)
- Previously `eprintln!` + `exit(1)` swallowed the help output and returned wrong exit code

### 13. Dead regex fields in RequirementPatterns
- **Status**: COMPLETE ✓ (cycle 19)
- Removed 3 unused fields: `range`, `platform_condition`, `env_condition`
- Only `basic_version`, `namespace`, `wildcard` are actually used in parsing
- `#[allow(dead_code)]` annotation removed entirely

### 11. PerformanceMonitor::reset() incomplete counter reset
- **Status**: COMPLETE ✓ (cycle 18)
- Fixed `reset()` method in `performance_monitor.rs` — 5 counters were missing from reset: `eviction_operations`, `total_eviction_latency_us`, `hit_count`, `miss_count`, `total_bytes_allocated`
- Added temp file patterns (`*_output.txt`, `*_test.txt`) to `.gitignore`
- Removed double blank lines in `Cargo.toml` and `crates/rez-next-build/Cargo.toml`

### 10. Duplicate code in serialization.rs
- **Status**: COMPLETE ✓ (cycle 17)
- Extracted shared `load_from_json_data()` — `load_from_data` and `load_from_yaml_data` now delegate to it (~90 lines deduped)
- `save_to_python()` now delegates to `save_to_python_with_options()` (~57 lines deduped)
- Removed 2 stale comments (lines 18-19, leftover from PyO3 import removal)
- Removed redundant `use serde_json;` in `search_v2.rs` (unnecessary in Rust 2018+)
- Net: -145 lines

## Medium Priority — Clippy Warnings

Clippy warnings: **0** (cycle 20, `--all-targets`)
- Fixed items-after-test-module in `cache/lib.rs` and `solver/astar/mod.rs` (cycle 20)

### 9. Orphan CLI files
- **Status**: COMPLETE ✓ (cycle 16)
- Deleted `src/cli/commands/search.rs` (592 lines) — replaced by `search_v2.rs`, `mod.rs` reference was already commented out
- Removed stale `// pub mod search;` and TODO comment from `commands/mod.rs`

## Completed (2026-04-02, cycle 16)

- [x] Removed 5 dead `raise_*` helper functions + 6 unit tests from `exceptions_bindings.rs` (-93 lines)
- [x] Deleted orphan `search.rs` (592 lines) — not in module tree, replaced by `search_v2.rs`
- [x] Removed stale `// TODO: Add more commands` comment and `// pub mod search;` from `commands/mod.rs`
- [x] Updated `CLEANUP_TODO.md`: mark #4 complete, update TODO audit (35→24), update clippy (~50→~0)

## Completed (2026-04-02, cycle 15)

- [x] Implemented `Display` trait for `PackageRequirement`, replacing manual `to_string()` (clippy::inherent_to_string fix)
- [x] Fixed `serialize_struct("Package", 24)` → `PACKAGE_SERIALIZED_FIELD_COUNT = 35` — field count was stale after struct growth
- [x] Replaced manual `Clone` impl for `Package` with `#[derive(Clone)]` — removed 42 lines of boilerplate
- [x] Fixed `PyPackageRequirement::__eq__` and `__hash__` to include `conflict` and `weak` fields — semantic bug fix
- [x] Fixed `conflict_requirement()` to avoid `!!` double prefix when called on already-conflict requirements
- [x] Normalized error formatting: `format!("{:?}", e)` → `e.to_string()` in `PyVersionRange::new()` and `from_str()`
- [x] Removed redundant `'static` lifetime from `FIELDS` constant in `Package::deserialize`
- [x] Used `strip_prefix` in `PackageRequirement::parse()` and `check_single_constraint()` — replaced 9 byte-index slices
- [x] Derived `Default` for `PackageSearchCriteria` and `RepositoryStats` — removed 2 manual impls
- [x] Removed double blank lines in `package_bindings.rs`

- [x] Tightened `deprecated` from `allow` to `warn`, fixed `base64::decode`/`encode` → `Engine::decode`/`encode` API
- [x] Tightened `ambiguous_glob_reexports` from `allow` to `warn`, fixed `RepositoryManager` conflict via explicit re-exports in `lib.rs`
- [x] Tightened `irrefutable_let_patterns` from `allow` to `warn`, fixed `if let` → `let` in `scanner.rs`
- [x] Removed all 30 clippy `allow` rules — all had zero instances in codebase
- [x] Deleted dead `reconstruct_string` function from `version.rs`
- [x] Added field-level `#[allow(dead_code)]` annotations to `AdvancedCacheEntry` (previously struct-level)
- [x] All Rust lints now at `warn` level — lint configuration tightening COMPLETE
- [x] Updated `CLEANUP_TODO.md` with cycle 12 progress, added #8 (dead `repository::RepositoryManager`)

## Completed (2026-04-01, cycle 11)

- [x] Fixed 24 `unused_variables` warnings: prefix with `_` across 11 files:
  - `serialization.rs`: `options` → `_options` in `load_from_file_with_options`
  - `high_performance_scanner.rs`: `results` → `_results`
  - `filesystem.rs`: `version_str` → `_version_str` in loop destructuring
  - `dependency_resolver.rs`: `package_name` → `_package_name` in `mark_requirement_satisfied`
  - `environment.rs`: `tool` → `_tool` in loop
  - `process.rs`: 8 params (`build_id`, `request`×4, `config`×6) prefixed with `_`
  - `systems.rs`: `request`×2, `cmd` → `_`-prefixed
  - `artifacts.rs`: `metadata` → `_metadata` in `get_file_permissions`
  - `status.rs`, `view.rs`, `build.rs`, `bundle.rs`, `pip.rs`: 6 CLI params prefixed
- [x] `unused_mut` lint: changed from `allow` to `warn` (zero instances in codebase)
- [x] Updated `CLEANUP_TODO.md` with cycle 11 progress

## Completed (2026-04-01, cycle 10)


- [x] Fixed compilation error: missing `StatePool` import in `test_framework.rs`
- [x] `dead_code` lint: changed from `allow` to `warn`
- [x] Removed 17 dead code items (~430 lines) across 19 files:
  - `range.rs`: `collect_probe_versions` (replaced by `_with_other`), `negate_bound_set` (unused approximation)
  - `requirement.rs`: `increment_last_token` (unused helper)
  - `cache.rs`: `save_cache_index` (never called)
  - `scanner.rs`: `cached_at` field, `scan_directory_recursive` + `scan_package_file` (legacy dead methods)
  - `dependency_resolver.rs`: `stats` field (initialized never read), `filter_candidates` (legacy alias)
  - `solver.rs`: `stats` field (initialized never read)
  - `astar_search.rs`: `state_pool` field (initialized never used)
  - `environment.rs`: 4 dead methods (`parse_commands_for_env_vars` cluster)
  - `release.rs`: `parse_variants`, `build.rs`: `view_preprocessed_package` + `generate_package_content`
  - `cp.rs` + `mv.rs`: `package_exists_at_destination` (2x, never called)
  - `pip.rs`: `location` + `home_page` fields (written never read)
- [x] Added `#[allow(dead_code)]` to 5 items (public API / cache metadata): `RequirementPatterns`, `AdvancedCacheEntry`, `CompositeHeuristic.config`, `AdaptiveHeuristic.base_heuristic`
- [x] Removed 5 unused imports: `SolverStats`, `StatePool`, `JoinSet`, `Path` (binder), `Package` (depends, bundle) + `HashMap` (bundle)
- [x] `unused_variables` lint: changed from `allow` to `warn` (26 warnings remaining — function signatures)
- [x] Updated `CLEANUP_TODO.md` with cycle 10 progress

## Completed (2026-04-01, cycle 9)

- [x] Deleted `batch.rs` (656 lines) — dead file, not in lib.rs module tree, no external references
- [x] Deleted `cache.rs` (798 lines) — dead file, not in lib.rs module tree, no external references
- [x] Deleted `dependency.rs` (851 lines) — dead file, not in lib.rs module tree, no external references
- [x] Deleted `variant.rs` (716 lines) — dead file, not in lib.rs module tree, no external references
- [x] Removed unused deps from rez-next-package: `lru`, `rayon`, `num_cpus`
- [x] `unused_imports` lint: changed from `allow` to `warn`
- [x] Removed 68 unused imports across 26 files (crates + CLI)
- [x] Added `[lints] workspace = true` to `rez-next-python` and `rez-next-search` Cargo.toml

## Completed (2026-04-01, cycle 8)

- [x] Deleted `version_token.rs` (371 lines) — dead pyo3 file, not in module tree
- [x] Deleted `token.rs` (123 lines) — dead pyo3 file, not in module tree
- [x] Deleted `validation.rs` (1034 lines) — dead pyo3 file, not in module tree, pyo3 commented out in Cargo.toml
- [x] Deleted `management.rs` (1077 lines) — dead pyo3 file, not in module tree, pyo3 commented out in Cargo.toml
- [x] Deleted `version_token_tests.rs` (6 lines) — empty test file for deleted module
- [x] Removed `pub mod version_token_tests` from tests/mod.rs
- [x] `unexpected_cfgs` lint: changed from `allow` to `warn`
- [x] Declared `flamegraph` and `quick-benchmarks` features in root Cargo.toml
- [x] Updated stale `unused_imports` comment
- [x] Removed duplicate `ResolutionResult` from `solver.rs` (12 lines) — was exact copy of `resolution.rs`
- [x] Renamed `dependency_resolver::ResolutionResult` → `DetailedResolutionResult` to eliminate glob ambiguity
- [x] Updated CLI `solve.rs` to use `DetailedResolutionResult`

## Completed (2026-04-01, cycle 7)

- [x] `version.rs`: full dual-fork merge — removed ~850 lines: dual struct fields, dual `Clone`, dual `parse()`, dual `compare_rez()`, dual `is_prerelease()`, dual `compare_token_strings()`, dual `reconstruct_string()`, entire `#[pymethods]` impl (230 lines), `create_version_with_python_tokens`, `extract_token_strings_gil_free`, `parse_optimized`, `parse_legacy_simulation`, `parse_with_gil_release`, `cmp_with_gil_release`, `OPTIMIZED_PARSER` static, imports for pyo3/PyTuple/AlphanumericVersionToken/once_cell/StateMachineParser
- [x] `parser.rs`: removed `#[cfg(feature = "python-bindings")] use VersionToken` and `parse_tokens()` dead method
- [x] `environment.rs`: removed commented-out `#[pyclass]`, entire `/* #[pymethods] ... */` block
- [x] `shell.rs`: removed `// use pyo3::prelude::*;` comment
- [x] `context/lib.rs`: removed `// use pyo3::prelude::*;` comment and `/* #[pymodule] ... */` block
- [x] `batch.rs`: removed `#[cfg(feature = "python-bindings")] use pyo3` and 12 `cfg_attr` annotations
- [x] `cache.rs`: removed `#[cfg(feature = "python-bindings")] use pyo3` and 6 `cfg_attr` annotations
- [x] `dependency.rs`: removed 3 `cfg_attr(python-bindings, pyclass)` annotations
- [x] `version_token_tests.rs`: updated comment to reflect current state
- [x] `lib.rs` (version): removed `Python bindings for version operations` doc line

## Completed (2026-04-01, cycle 6)

- [x] `dependency.rs`: removed 14 `cfg_attr(python-bindings, pyclass/pymethods/new/staticmethod)` annotations and `use pyo3`
- [x] `cache.rs`: removed 9 `cfg_attr(python-bindings, ...)` annotations and `use pyo3`
- [x] `batch.rs`: removed 12 `cfg_attr(python-bindings, ...)` annotations and `use pyo3`
- [x] `serialization.rs`: removed 2 `cfg_attr(python-bindings, pyclass)` annotations
- [x] `variant.rs`: full dual-fork merge
- [x] `package.rs`: full dual-fork merge
- [x] `test_package_management_rust.rs`: deleted entire file

## Completed (2026-04-01, cycle 5)

- [x] 6 lib.rs files: removed `#[pymodule]`, `use pyo3`, conditional `pub mod`, conditional re-exports
- [x] `rez-next-common/error.rs`: removed `PyO3` error variant and `create_exception!`
- [x] `rez-next-common/config.rs`: merged dual pyclass/not-pyclass config impls
- [x] `rez-next-version/tests/version_token_tests.rs`: cleared dead test module
- [x] `rez-next-package/lib.rs`: removed 6 conditional mod, 7 re-exports, pymodule block, 6 dead tests
- [x] `rez-next-solver/solver.rs`, `rez-next-build/builder.rs`, `process.rs`: removed pymethods impls
- [x] `rez-next-repository/repository.rs`, `filesystem.rs`: removed cfg_attr pyclass/pymethods
- [x] `rez-next-context/context.rs`: removed pymethods impl, 6 dual-gated struct fields

## Completed (2026-03-31)

- [x] Removed commented-out `_rez_core` PyModule function from `src/lib.rs`
- [x] Removed commented-out `from_resolution_result` method from context.rs
- [x] Removed `// mod cache` and `// mod optimized_solver` from solver/lib.rs
- [x] Removed commented-out `// pub use cache::*` and `// pub use optimized_solver::*` from solver/lib.rs
- [x] Removed `// use rez_next_repository::...` from optimized_solver.rs
