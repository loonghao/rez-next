# Cleanup TODO

## High Priority — Structural Refactoring

### 1. `python-bindings` feature cleanup
- **Status**: COMPLETE ✓
- **Impact**: Originally 119+ `#[cfg(feature = "python-bindings")]` blocks across 10+ crates. ~2400 lines removed total across 7 cycles.
- **Root cause**: Python bindings migrated to `rez-next-python` crate, but old per-crate `#[cfg(feature = "python-bindings")]` code was left behind. The feature was never defined in any `Cargo.toml`, and `pyo3` was not a dependency in non-python crates.
- **Verification**: `grep -r 'cfg.*python.bindings' crates/ --include='*.rs'` returns 0 results (excluding `rez-next-python/`)
- **Note**: `version_token.rs` and `token.rs` have been deleted in cycle 8 — they were dead files not in the module tree.

### 2. Workspace lint configuration tightening
- **Status**: PARTIALLY COMPLETE ✓
- `unexpected_cfgs` changed from `allow` to `warn` (cycle 8)
- `flamegraph` and `quick-benchmarks` features declared in `[features]`
- Remaining `allow` lints: `dead_code`, `unused_imports`, `unused_variables`, `deprecated`, `ambiguous_glob_reexports`, `irrefutable_let_patterns`, `unused_mut`
- **Next**: Tighten `unused_imports` to `warn` and fix warnings

### 3. Duplicate `ResolutionResult` types
- **Status**: COMPLETE ✓
- Removed duplicate `ResolutionResult` from `solver.rs` (exact copy of `resolution.rs`)
- `solver.rs` now imports `crate::resolution::ResolutionResult`
- Renamed `dependency_resolver::ResolutionResult` to `DetailedResolutionResult` (different schema)
- CLI `solve.rs` updated to use `DetailedResolutionResult`

### 4. `#[allow(dead_code)]` helper functions (5 in exceptions_bindings.rs)
- `raise_resolve_error`, `raise_package_not_found`, `raise_config_error`, `raise_build_error`, `raise_rex_error`
- These are utility functions for future use. Keep for now, remove `#[allow(dead_code)]` when actually used.

### 5. Orphan pyo3 files in non-python crates
- **Status**: COMPLETE ✓
- Deleted `version_token.rs` (371 lines), `token.rs` (123 lines), `validation.rs` (1034 lines), `management.rs` (1077 lines), `version_token_tests.rs` (6 lines)
- None were in lib.rs module trees, none were compiled, pyo3 was not a dependency of these crates
- rez-next-python does not reference any types from these files

### 6. Dead .rs files in rez-next-package not in module tree
- **Status**: COMPLETE ✓ (cycle 9)
- Deleted `batch.rs`, `cache.rs`, `dependency.rs`, `variant.rs` — all dead files not in lib.rs module tree

### 7. Further lint tightening
- **Status**: MOSTLY COMPLETE ✓
- `unused_imports`: `allow` → `warn` + 68 imports cleaned (cycle 9)
- `dead_code`: `allow` → `warn` + ~430 lines dead code removed (cycle 10)
- `unused_variables`: `allow` → `warn` (cycle 10, 26 warnings remaining — need manual _ prefix in function signatures)
- Remaining: `ambiguous_glob_reexports`, `unused_mut` still `allow`
- **Action**: Fix remaining unused_variables warnings, then tighten `unused_mut`

## Medium Priority — TODO Audit

35+ TODO comments across the codebase. Key categories:
- **Implementation gaps**: LRU eviction, memory tracking, CPU usage monitoring (cache/repo)
- **CLI stubs**: time-based removal, daemon logic, validation filters
- **Version system**: token comparison, caching, proper type distinction
- None of these TODOs are blocking; they represent future work items.

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
