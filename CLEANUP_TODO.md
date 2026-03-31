# Cleanup TODO

## High Priority — Structural Refactoring

### 1. `python-bindings` feature cleanup (119+ occurrences)
- **Status**: Recorded for next cleanup cycle
- **Impact**: 119+ `#[cfg(feature = "python-bindings")]` blocks across 10+ crates reference a feature that is **never defined** in any `Cargo.toml`
- **Root cause**: Python bindings migrated to `rez-next-python` crate, but old per-crate `#[cfg(feature = "python-bindings")]` code was left behind
- **Action**: Remove all `#[cfg(feature = "python-bindings")]` gated code from non-python crates (version, solver, context, common, repository, build, package)
- **Risk**: Low — this code is never compiled (feature never enabled)
- **Files affected**: `rez-next-version` (18), `rez-next-context` (9), `rez-next-common` (6), `rez-next-solver` (4), `rez-next-repository` (3), `rez-next-build` (3), `rez-next-package` (15+)

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

## Completed (2026-03-31)

- [x] Removed commented-out `_rez_core` PyModule function from `src/lib.rs`
- [x] Removed commented-out `from_resolution_result` method from context.rs
- [x] Removed `// mod cache` and `// mod optimized_solver` from solver/lib.rs
- [x] Removed commented-out `// pub use cache::*` and `// pub use optimized_solver::*` from solver/lib.rs
- [x] Removed `// use rez_next_repository::...` from optimized_solver.rs
