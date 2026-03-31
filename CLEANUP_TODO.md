# Cleanup TODO

## High Priority — Structural Refactoring

### 1. `python-bindings` feature cleanup
- **Status**: IN PROGRESS — Phase 1 complete, Phase 2 nearly complete, `version.rs` + `parser.rs` remaining
- **Impact**: Originally 119+ `#[cfg(feature = "python-bindings")]` blocks across 10+ crates. ~1400 lines removed so far.
- **Root cause**: Python bindings migrated to `rez-next-python` crate, but old per-crate `#[cfg(feature = "python-bindings")]` code was left behind. The feature is never defined in any `Cargo.toml`, and `pyo3` is not a dependency.
- **Completed** (2026-04-01, cycle 6):
  - `dependency.rs`: removed 14 `cfg_attr(python-bindings, pyclass/pymethods/new/staticmethod)` annotations and `use pyo3`
  - `cache.rs`: removed 9 `cfg_attr(python-bindings, ...)` annotations and `use pyo3`
  - `batch.rs`: removed 12 `cfg_attr(python-bindings, ...)` annotations and `use pyo3`
  - `serialization.rs`: removed 2 `cfg_attr(python-bindings, pyclass)` annotations
  - `variant.rs`: full dual-fork merge — removed dual struct fields (4 pairs), dual Clone impl, `#[pymethods]` block (+moved useful methods to regular impl), `cfg_attr(pyclass)`, dead test cfg gate
  - `package.rs`: full dual-fork merge — removed dual struct fields (6 pairs), dual Clone impl, `#[pymethods]` block (+moved useful methods), `from_dict` dead impl, `cfg_attr(pyclass)`, `use pyo3`
  - `test_package_management_rust.rs`: deleted entire file (all test functions gated by never-defined feature)
- **Completed** (2026-04-01, cycle 5):
  - 6 lib.rs files: removed `#[pymodule]`, `use pyo3`, conditional `pub mod`, conditional re-exports
  - `rez-next-common/error.rs`: removed `PyO3` error variant and `create_exception!`
  - `rez-next-common/config.rs`: removed `cfg_attr(pyclass)`, merged dual `#[pymethods]` / `#[cfg(not(...))]` impls
  - `rez-next-version/tests/version_token_tests.rs`: cleared dead test module
  - `rez-next-package/lib.rs`: removed 6 conditional `pub mod`, 7 conditional re-exports, `#[pymodule]` block, 6 dead tests
  - `rez-next-solver/solver.rs`: removed `#[pymethods]` impl, `use pyo3`, `cfg_attr(pyclass)`
  - `rez-next-build/builder.rs`: removed `#[pymethods]` impl (build_package_py, get_build_status_py, stats getter)
  - `rez-next-build/process.rs`: removed `#[pymethods]` impl (build_id/status/package_name getters)
  - `rez-next-repository/repository.rs`: removed `cfg_attr(pyclass/pymethods/new/getter)`
  - `rez-next-repository/filesystem.rs`: removed `cfg_attr(pyclass/pymethods/new/getter)`
  - `rez-next-context/context.rs`: removed `#[pymethods]` impl, 6 dual-gated struct fields
- **Remaining** (next cycle — FINAL):
  - `version.rs`: ~19+13 blocks — most complex file. Has dual `parse()`, `Clone`, `is_prerelease`, `reconstruct_string`, `compare_rez`, `compare_token_strings`, `#[pymethods]` impl (230 lines), dual struct fields, Python-specific helpers (`create_version_with_python_tokens`, `extract_token_strings_gil_free`, `parse_optimized`, `parse_legacy_simulation`, `parse_with_gil_release`, `cmp_with_gil_release`). Needs careful line-by-line merge.
  - `parser.rs`: 2 blocks (`use VersionToken`, `parse_tokens()` method) — simple removal
  - `environment.rs`: 1 block inside `/* ... */` comment — cleanup comment block
- **Risk**: `version.rs` is high-risk due to dual-forked parse/compare logic; recommend dedicated focused session

### 2. Workspace lint configuration tightening
- **Status**: Recorded for next cleanup cycle
- Root `Cargo.toml` sets `dead_code = "allow"`, `unused_imports = "allow"`, `unused_variables = "allow"`, `unexpected_cfgs = "allow"` globally
- This suppresses all dead code warnings and hides the `python-bindings` cfg issue
- **Action**: Progressively tighten to `warn` level, fix warnings, then consider `deny` for `dead_code`

### 3. Duplicate `ResolutionResult` types
- Three separate `ResolutionResult` structs exist in:
  - `crates/rez-next-solver/src/resolution.rs` (used by tests)
  - `crates/rez-next-solver/src/dependency_resolver.rs` (different fields)
  - `crates/rez-next-solver/src/solver.rs` (duplicate of resolution.rs)
- Glob re-exports (`pub use *`) cause ambiguity
- **Action**: Consolidate into a single canonical type, remove duplicates

### 4. `#[allow(dead_code)]` helper functions (5 in exceptions_bindings.rs)
- `raise_resolve_error`, `raise_package_not_found`, `raise_config_error`, `raise_build_error`, `raise_rex_error`
- These are utility functions for future use. Keep for now, remove `#[allow(dead_code)]` when actually used.

## Medium Priority — TODO Audit

35+ TODO comments across the codebase. Key categories:
- **Implementation gaps**: LRU eviction, memory tracking, CPU usage monitoring (cache/repo)
- **CLI stubs**: time-based removal, daemon logic, validation filters
- **Version system**: token comparison, caching, proper type distinction
- None of these TODOs are blocking; they represent future work items.

## Completed (2026-04-01)

- [x] Removed python-bindings gates from 6 lib.rs files (pymodule, use pyo3, conditional mods/re-exports)
- [x] Removed PyO3 error variant, create_exception from rez-next-common/error.rs
- [x] Merged dual pyclass/not-pyclass config impls in rez-next-common/config.rs
- [x] Cleared dead version_token_tests.rs (entire file gated by python-bindings)
- [x] Removed 6 dead test functions from rez-next-package/lib.rs
- [x] Removed pymethods impls from solver.rs, builder.rs, process.rs, context.rs
- [x] Removed cfg_attr pyclass/pymethods from repository.rs, filesystem.rs
- [x] Removed 6 dual-gated struct fields from context.rs

## Completed (2026-03-31)

- [x] Removed commented-out `_rez_core` PyModule function from `src/lib.rs`
- [x] Removed commented-out `from_resolution_result` method from context.rs
- [x] Removed `// mod cache` and `// mod optimized_solver` from solver/lib.rs
- [x] Removed commented-out `// pub use cache::*` and `// pub use optimized_solver::*` from solver/lib.rs
- [x] Removed `// use rez_next_repository::...` from optimized_solver.rs
