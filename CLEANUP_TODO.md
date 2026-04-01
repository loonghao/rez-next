# Cleanup TODO

## High Priority — Structural Refactoring

### 1. `python-bindings` feature cleanup
- **Status**: COMPLETE ✓
- **Impact**: Originally 119+ `#[cfg(feature = "python-bindings")]` blocks across 10+ crates. ~2400 lines removed total across 7 cycles.
- **Root cause**: Python bindings migrated to `rez-next-python` crate, but old per-crate `#[cfg(feature = "python-bindings")]` code was left behind. The feature was never defined in any `Cargo.toml`, and `pyo3` was not a dependency in non-python crates.
- **Verification**: `grep -r 'cfg.*python.bindings' crates/ --include='*.rs'` returns 0 results (excluding `rez-next-python/`)
- **Note**: `version_token.rs` and `token.rs` still have unconditional `use pyo3` — these files are used by `rez-next-python` crate and are legitimate pyo3 types. They are NOT referenced from `rez-next-version/lib.rs`.

### 2. Workspace lint configuration tightening
- **Status**: Recorded for next cleanup cycle
- Root `Cargo.toml` sets `dead_code = "allow"`, `unused_imports = "allow"`, `unused_variables = "allow"`, `unexpected_cfgs = "allow"` globally
- Now that python-bindings cleanup is complete, `unexpected_cfgs` can be changed to `warn`
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

### 5. Orphan pyo3 files in non-python crates
- `crates/rez-next-version/src/version_token.rs`: entire file uses pyo3 unconditionally (pyclass/pymethods/PyResult). Not exported from lib.rs.
- `crates/rez-next-version/src/token.rs`: has `PyVersionToken` wrapper with pyo3 attrs. `VersionToken` enum is used by parser.
- `crates/rez-next-package/src/validation.rs`: all structs are `#[pyclass]` with `#[pyo3(get)]` — unconditional pyo3 usage
- `crates/rez-next-package/src/management.rs`: all structs are `#[pyclass]` with `#[pyo3(get)]` — unconditional pyo3 usage
- **Action**: These files need pyo3 stripped or moved to rez-next-python. High risk — deep refactor required.

## Medium Priority — TODO Audit

35+ TODO comments across the codebase. Key categories:
- **Implementation gaps**: LRU eviction, memory tracking, CPU usage monitoring (cache/repo)
- **CLI stubs**: time-based removal, daemon logic, validation filters
- **Version system**: token comparison, caching, proper type distinction
- None of these TODOs are blocking; they represent future work items.

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
