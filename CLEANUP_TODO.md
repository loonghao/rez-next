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

### 37. Rust dependency audit still reports 3 unmaintained crates
- **Status**: OPEN
- `cargo audit -q` still reports these unmaintained dependencies:
  - direct: `bincode 2.0.1` via `rez-next-package`
  - transitive via `rustpython-parser`: `paste 1.0.15`, `unic-ucd-version 0.9.0`
- Follow-up: evaluate a dedicated migration path for direct `bincode`, and decide whether the two `rustpython-parser` advisories should be handled via upstream upgrade, patching, or documented acceptance.

### 38. `repository_bindings.rs` tests still accept ambiguous success/error outcomes
- **Status**: OPEN
- Several tests currently allow both `Ok(empty)` and `Err(_)`, which hides repository scanning contract drift instead of documenting it.
- Follow-up: extract a shared temp-repo fixture helper and tighten `find_packages` / `get_latest_package` / `get_package_family_names` assertions around the path layouts we actually support.

### 39. Large Rust files remain above 800 lines after recent iteration growth
- **Status**: OPEN
- Current >800-line candidates (excluding `target/`): `tests/cli_e2e_tests.rs` (955), `tests/rez_compat_late_tests.rs` (942), `tests/rez_compat_search_tests.rs` (884), `crates/rez-next-solver/src/dependency_resolver_tests.rs` (864), `tests/rez_compat_misc_tests.rs` (862), `crates/rez-next-rex/src/executor_tests.rs` (856), `crates/rez-next-python/src/diff_bindings.rs` (854), `crates/rez-next-repository/src/filesystem_tests.rs` (850), `crates/rez-next-python/src/depends_bindings.rs` (840), `crates/rez-next-repository/src/scanner.rs` (834), `tests/rez_compat_tests.rs` (807), `tests/rez_solver_advanced_tests.rs` (806).
- Follow-up: prioritize splitting the mixed integration suites first (`cli_e2e_tests.rs`, `rez_compat_*`, `rez_solver_advanced_tests.rs`) before more iteration cycles add overlap.

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
- **Status**: COMPLETE ✓ (cycle 84)
- `src/cli/commands/pkg_cache.rs` (793 lines) split into `pkg_cache/` directory:
  - `types.rs` — `PkgCacheArgs`, `PkgCacheMode`, `CacheEntry`, `CacheStatus`
  - `ops.rs` — `add_variants`, `remove_variants`, `clean_cache`, `run_daemon`, `view_logs`, `determine_cache_directory`, `initialize_cache_manager`
  - `display.rs` — `show_cache_status`, `show_cache_entries_table`, `scan_cache_directory`, table helpers
  - `mod.rs` — entry point `execute()`, integration tests
- `src/cli/commands/search_v2.rs` (718 lines) split into `search_v2/` directory:
  - `types.rs` — `SearchArgs`, `SearchResult`
  - `matcher.rs` — `evaluate_package_match`, `get_package_timestamp`
  - `filter.rs` — `perform_search`, `sort_results`, `filter_latest_versions`
  - `display.rs` — `display_search_results`, table/JSON/detailed format renderers
  - `mod.rs` — entry point `execute()`, `search_async()`, `add_default_repositories()`
- All files now ≤240 lines; 0 clippy warnings; all tests pass
- Follow-up: keep `bind.rs` (~500 lines) and `systems/mod.rs` (~424 lines) from regrowing mixed responsibilities


### 24. CLI helper logic is still duplicated across commands
- **Status**: COMPLETE ✓ (cycle 34)
- `build.rs` and `search_v2/mod.rs` now both use `src/cli/utils.rs::expand_home_path` for `~` expansion instead of carrying command-local logic
- Time parsing remains centralized in `src/cli/utils.rs`; future CLI commands that discover repositories or package paths should route through the same shared helper layer


### 25. Public compatibility stubs still need explicit product decisions
- **Status**: COMPLETE ✓ (cycle 43 for build-system tests; stubs fixed in cycle 39)
- `get_pip_dependencies()` — **FIXED**: now raises `NotImplementedError` instead of returning empty list silently (cycle 39)
- `pip_install()` — **FIXED**: now raises `NotImplementedError` instead of fake-installing packages (cycle 39)
- `optimized_solver.rs` — **DELETED**: dead file not in module tree, `detect_conflicts_optimized()` was only reachable via this dead code (cycle 39)
- `crates/rez-next-build/src/systems/` — **TESTED** (cycle 43): added mock tests for `PythonBuildSystem`, `NodeJsBuildSystem`, `CargoBuildSystem`, and `BuildSystem::detect`/`detect_with_package` using `tempdir`
  - 10 new `detect*` tests in `systems/mod.rs`; 4 tests in `python.rs`; 3 tests in `nodejs.rs`; 3 tests in `cargo_build.rs`
  - `BuildStep` received `#[derive(PartialEq)]` to support `assert_eq!`
  - Cycle 46 removed redundant unit-struct smoke tests and cleaned follow-up clippy regressions in nearby bindings/tests
  - All 70 rez-next-build tests pass; 0 clippy warnings

### 26. Build-system command execution still depends on shell-specific strings
- **Status**: COMPLETE ✓ (cycle 80)
- Extracted `crates/rez-next-build/src/systems/cmd_builder.rs` with two shared helpers:
  - `run_cmd(executor, step, cmd, optional, fallback_msg)` — runs a command; when `optional=true` swallows non-zero exits and `Err` variants, returning `success: true` + `fallback_msg`; no `2>&1` in command strings
  - `make_install_cmd(destdir)` — formats `make install DESTDIR="..."` with proper quoting
- `nodejs.rs`: `compile()` + `test()` rewritten with `run_cmd(…, optional=true, …)`, removing `"2>&1 || echo '...'"` literals
- `cargo_build.rs`: `configure()`, `test()`, `package()` rewritten with `run_cmd`; `"2>&1"` removed from all three command strings
- `python.rs`: `test()` rewritten to run pytest first (optional) then unittest discover (optional); no `"2>&1 || python -m unittest"` inline
- `make.rs`: `install()` now uses `make_install_cmd()` instead of inline `format!("make install DESTDIR={}", …)`
- `mod.rs`: registers `pub(crate) mod cmd_builder`
- `cmd_builder.rs` has 2 unit tests for `make_install_cmd`
- All 70 rez-next-build tests pass; Clippy: 0 warnings

### 27. Python context/source bindings still expose placeholder compatibility behavior
- **Status**: COMPLETE ✓ (cycles 82-83)
- `context_bindings.rs`: introduced module-level `TOKIO_RT: OnceLock<Runtime>` (shared via `crate::runtime::get_runtime()`); `get_tools()` now uses `pkg.base` as primary path, falls back to `{name}-{version}/bin/{tool}` estimate
- `source_bindings.rs`: `REZ_CONTEXT_FILE` now uses cross-platform `std::env::temp_dir().join("rez_context.rxt")` (Windows-safe)
- Cycle 83b: extracted `crates/rez-next-python/src/runtime.rs` — all 14 per-call `Runtime::new()` occurrences across 8 binding files migrated to shared `get_runtime()`

### 28. `rez-next-context` test mega-file should be split by concern
- **Status**: COMPLETE ✓ (cycle 56)
- `crates/rez-next-context/src/tests.rs` has already been replaced by `crates/rez-next-context/src/tests/` with focused modules for context loading, shell generation, RXT/RXTB IO, execution, env diff, and resolved-context behavior
- The previous TODO note became stale after iteration commit `4aa3b1d`, which completed the split into concern-specific test modules
- Follow-up: keep future context tests in the focused modules instead of regrowing a single mega-file

### 29. `RexExecutor` still applies actions after `stop()`
- **Status**: COMPLETE ✓ (cycle 28)
- This TODO became stale after iteration commit `c4ba991`, which changed `RexEnvironment::apply()` to stop processing after `RexActionType::Stop`
- Current behavior is now locked by focused tests in `crates/rez-next-rex/src/lib.rs` and `crates/rez-next-rex/src/executor_tests.rs`
- Follow-up: keep documenting rez-compatible `stop()` semantics in user-facing Rex docs if new command examples are added

### 30. Repository format support has diverged between `FileSystemRepository` and `SimpleRepository`
- **Status**: COMPLETE ✓ (cycle 84, refreshed cycle 34)
- Iteration commit `53abfa1` updated `SimpleRepository` to scan `package.py`, `package.yaml`, `package.yml`, and `package.json`
- Cycle 34 removed the last local duplicate by switching `simple_repository.rs` from its private `PACKAGE_FILENAMES` array to the shared `scanner_types::REZ_PACKAGE_FILENAMES` constant
- `simple_repository_tests.rs` locks the behavior with yaml/json/yml discovery coverage plus an explicit `package.py`-beats-`package.yaml` priority assertion



### 31. `PackageBinder::list_bound_packages()` still lacks a real unit-test seam
- **Status**: COMPLETE ✓ (cycle 79)
- Extracted `list_bound_packages_in(install_root: &Path)` as a public free function in `binder.rs`
- `PackageBinder::list_bound_packages()` now delegates to it
- Exported via `lib.rs` as `rez_next_bind::list_bound_packages_in`
- Added 7 contract tests: empty dir, nonexistent dir, single package, multiple families, multiple versions sorted, ignores dirs without package.py, ignores non-dir root entries, alphabetical sort

### 32. `PrefetchPredictor` tests still encode placeholder semantics instead of behavior contracts
- **Status**: COMPLETE ✓ (cycle 80)
- `PrefetchPredictor` struct now has a full doc-comment block documenting that all three methods are placeholders returning constant / empty values
- All three `impl` methods have inline `/// **Placeholder**: ...` doc lines
- Test module renamed from `test_prefetch_predictor` → `test_prefetch_predictor_smoke`
- All 5 test function names updated with explicit `_smoke` suffix
- Each test now carries a `// Placeholder:` comment explaining what the placeholder currently does
- Follow-up: when real ML prediction is implemented, replace the smoke tests with contract tests that verify actual behavior against known inputs

### 33. `cli_e2e_tests.rs` still allows implicit skips and weak exit-code assertions
- **Status**: PARTIAL — reopened in cycle 91
- Cycle 78 added `rez_output()` and removed most exit-code-only assertions, but later test drift left a few misleading cases behind.
- Cycle 91 fixed 3 concrete issues:
  - `test_view_package_in_repo`: now views a real package directory instead of creating an unused temp repo and accepting any non-empty output
  - `test_full_workflow_search_and_view`: now views a real package directory and no longer relies on the stale `view --path ...` invocation that current `ViewArgs` does not support
  - `test_build_extra_args_separator_accepted`: now runs inside the temp package root so the fixture is actually consumed, and it guards against the false-negative `No package.py ... found` path
- Remaining follow-up:
  - `skip_no_bin!()` still returns early when the built binary is missing; decide whether CI should always prebuild the binary and turn that case into an explicit precondition failure instead of an implicit skip

### 34. `real_repo_*` split test files still duplicate local repository helpers

- **Status**: COMPLETE ✓ (cycle 32)
- Extracted shared helpers into `tests/real_repo_test_helpers.rs` (`create_package`) and `tests/real_repo_manager_helpers.rs` (`make_repo`)
- `tests/real_repo_integration.rs`, `tests/real_repo_resolve_tests.rs`, and `tests/real_repo_context_tests.rs` now reuse the shared helpers instead of keeping near-identical local fixture builders
- Follow-up: keep future real-repo fixture behavior centralized in these helper modules so the split integration suites do not drift again


### 35. Split-test migration notice shells still build as empty integration targets
- **Status**: COMPLETE ✓ (cycle 77)
- Deleted `tests/rez_solver_graph_tests.rs`, `tests/rez_solver_platform_tests.rs`, and `tests/rez_compat_late_tests.rs` — all were 7-11 line comment-only files with no tests; git history in the split-file commit messages is sufficient
- All tests continued to pass (0 failed); test-target noise reduced by 3 empty crates

### 36. Compat cycle tests now overlap with dedicated solver-graph topology coverage
- **Status**: COMPLETE ✓ (cycle 77)
- Removed 4 duplicate cycle tests from `tests/rez_compat_context_tests.rs`: `test_circular_dependency_direct`, `test_circular_dependency_three_way`, `test_no_circular_dependency_linear`, `test_self_referencing_package_is_cycle`
- Kept `test_diamond_dependency_not_cycle` which has no equivalent in the topology suite
- File reduced from 713 → ~550 lines; all tests pass

### 37. Python shell detection logic is duplicated across bindings
- **Status**: OPEN (cycle 33)
- `shell_bindings.rs`, `completion_bindings.rs`, `status_bindings.rs`, `context_bindings.rs`, and `source_bindings.rs` each implement their own environment-based shell detection
- The fallbacks have already drifted: `source_bindings.rs` says "Windows CMD fallback" but returns `powershell`, while `completion_bindings.rs` defaults any Windows environment to `powershell` and others still distinguish `cmd`
- Follow-up: extract a shared helper in `rez-next-python` and align all call sites/tests to one shell-detection contract

### 38. Python compatibility tests still duplicate helpers and overfit placeholder APIs
- **Status**: OPEN (cycle 33)
- `write_package_py` is duplicated in `test_e2e_real_world.py` and `test_context_repository_api.py`; shell/bundle assertions are also duplicated across `test_compat_io_modules.py` and `test_e2e_real_world.py`
- Several tests in `test_compat_advanced.py` only assert list-ness / empty results against nonexistent paths, and `test_compat_io_modules.py` currently locks in the `cli_functions.rs` known-command stub instead of an observable CLI contract
- Follow-up: centralize shared Python test fixtures/helpers and replace placeholder-smoke cases with temp-repo behavior tests before broadening compatibility claims

### 39. `move_package()` may delete the wrong source version when `version=None`
- **Status**: OPEN (cycle 34)
- `copy_package()` selects the latest available package version when `version` is omitted, but `move_package()` later removes `pkg_name/unknown` because it falls back to `version.unwrap_or("unknown")` instead of the version that was actually copied.
- This looks like a correctness bug rather than low-risk cleanup, so it should be fixed in a dedicated change after locking the intended contract with tests.
- Follow-up: return or share the selected version from `copy_package()` so `move_package()` can delete the exact source directory it copied.

### 40. `selftest_functions.rs` still mixes library-side reporting with panic-prone checks
- **Status**: COMPLETE ✓ (cycle 132)
- Refactored `selftest()` around a shared `collect_selftest_results()` helper so the runtime path and unit tests now exercise the same contract surface.
- Removed library-side `eprintln!` reporting; the Python API now returns counts only, and tests assert against structured check results instead of regrowing cycle-tagged smoke cases.
- Replaced panic-prone internal `unwrap()` usage in the self-test checks with fallible guards, so malformed internal fixtures become failed checks instead of crashing the self-test entry point.

### 41. `bundle_functions.rs` still over-tests placeholder `dest_packages_path`

- **Status**: COMPLETE ✓ (cycle 35)
- Removed the placeholder-only `test_unbundle_with_dest_path_is_ignored_but_ok` smoke test from `bundle_functions.rs`.
- Coverage now stays focused on observable manifest parsing / roundtrip behavior instead of locking in the reserved `dest_packages_path` argument's current no-op semantics.
- Follow-up: once package extraction is implemented, add filesystem-observable tests for the real extraction contract rather than reintroducing placeholder acceptance checks.

### 42. `cli_functions.rs` still documents a real CLI surface over a stubbed command table
- **Status**: PARTIAL ✓ (cycle 132)
- Doc comments now explicitly describe `cli_run()` / `cli_main()` as compatibility stubs that validate against `KNOWN_COMMANDS` and ignore `args`.
- Removed the regrown per-command `Ok(0)` smoke tests; coverage now stays at the table level (known commands, malformed commands, `cli_main` dispatch) instead of growing sideways with each iteration cycle.
- Remaining follow-up:
  - `cli_run()` still discards `args` and does not dispatch to the real rez CLI.
  - Decide whether this module should remain an explicit stub or begin routing into real command execution before widening the documented contract.

### 43. `config_bindings.rs` still grows through non-observable config smoke tests
- **Status**: OPEN (cycle 128)
- Recent iteration cycles added multiple tests that only assert non-empty strings, compile-time field access, or `get_field()` no-panic behavior for config values such as `local_packages_path`, `release_packages_path`, `use_rust_solver`, and `version_check_behavior`.
- The file already has stronger nearby contracts for default values, getter/inner parity, and JSON field typing, so the remaining smoke cases mostly add count without adding behavioral signal.
- Follow-up: when revisiting `config_bindings.rs`, consolidate around exact default-value contracts and selected typed `get_field()` assertions, and remove compile-only / no-panic checks instead of letting the file keep growing sideways.

### 44. Python repository compatibility tests still describe real contract drift as “not implemented”
- **Status**: OPEN (cycle 39)
- `test_context_repository_api.py` still carried stale `xfail` reasons even though the APIs exist and fail for narrower reasons:
  - `RepositoryManager.get_latest_package()` and top-level `get_latest_package()` currently return `3.9.0` ahead of `3.11.0` because the binding sorts version strings lexicographically instead of using semantic version ordering.
  - top-level `get_package_family_names()` / `walk_packages()` and `RepositoryManager.get_package_family_names()` still return `[]` on temp repos because the current implementation delegates to `find_packages("")`, which does not enumerate package families for an empty-name scan.
- Follow-up: fix the binding helpers in a dedicated correctness change, then remove the now-accurate xfails instead of widening placeholder compatibility claims.



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
