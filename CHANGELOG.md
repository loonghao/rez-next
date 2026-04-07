# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.7](https://github.com/loonghao/rez-next/compare/v0.1.6...v0.1.7) (2026-04-07)


### 🐛 Bug Fixes

* **ci:** resolve win-msvc test race, security audit permissions, and yanked dep ([#113](https://github.com/loonghao/rez-next/issues/113)) ([9ba145b](https://github.com/loonghao/rez-next/commit/9ba145b7134a5abdf7bee2a4aa494fd932b8b818))
* resolve issues 108, 109, 110 - test markers and performance monitoring ([#111](https://github.com/loonghao/rez-next/issues/111)) ([7a0bf9c](https://github.com/loonghao/rez-next/commit/7a0bf9c291594c908f0151103a80e9e63b6e98d3))


### ⚡ Performance

* **rex:** cache RexParser to eliminate redundant regex compilation ([#114](https://github.com/loonghao/rez-next/issues/114)) ([1ceb5be](https://github.com/loonghao/rez-next/commit/1ceb5be9179fcbfe63ae495add026824a29b86d3))

## [0.1.6](https://github.com/loonghao/rez-next/compare/v0.1.5...v0.1.6) (2026-04-06)


### 🚀 Features

* **deps:** upgrade dependencies + Python 3.7 CI + E2E tests + auto-improve Cycles 70-90 ([0aa7024](https://github.com/loonghao/rez-next/commit/0aa70242ee844d17791255dd323c8466a9405ba7))
* **logging:** Cycle 35 — replace eprintln! with tracing::warn! in library code [iteration-done] ([5f96bac](https://github.com/loonghao/rez-next/commit/5f96bac806e886a70d7449899877bf77d33a2221))
* **repository:** Cycle 63 — support package.py in FileSystemRepository + variants tests + split filesystem tests [iteration-done] ([a70d978](https://github.com/loonghao/rez-next/commit/a70d97858461d9be10b5b7b45c3e04f3f75162b7))
* squash merge auto-improve (Cycle 64-68) — solver/context/build tests, bind/repo refactors, Python binding improvements, fmt fixes ([fa4cbd3](https://github.com/loonghao/rez-next/commit/fa4cbd3424de013b31568c504a542baadec48c07))
* squash merge auto-improve branch - solver tests, benchmarks, cleanup ([8eb4d28](https://github.com/loonghao/rez-next/commit/8eb4d28b4cd400e252a15abd07507b608655e9c1))
* **version:** improve token comparison + 6 prerelease ordering tests (Cycle 29) [iteration-done] ([d902406](https://github.com/loonghao/rez-next/commit/d90240662cd2097c7daa482ddcf95f920c681497))


### 🐛 Bug Fixes

* **ci:** drop --release from maturin develop to avoid --include-debuginfo/--strip conflict ([da8442e](https://github.com/loonghao/rez-next/commit/da8442e1d4c8844cf1ad3bbd0e419ee7840c80fb))
* **ci:** remove global strip=true from pyproject.toml to fix maturin develop abi3 conflict ([28c2694](https://github.com/loonghao/rez-next/commit/28c269463d50a1547b718a97167f8dc7c30cc151))
* **ci:** rustfmt format all files and regenerate Cargo.lock to fix checksum mismatch ([05f6492](https://github.com/loonghao/rez-next/commit/05f6492dbde81a2cf2a9da520c1c05f1d235b3bf))
* **deps:** Cycle 79 — fix Windows path separator bug, pyo3/bincode/whoami v2 migration, list_bound_packages_in contract tests [iteration-done] ([5304bc9](https://github.com/loonghao/rez-next/commit/5304bc9c77e5f95fe3cf7887f661f41887000332))
* **deps:** update rust crate bincode to v3 ([f360f7d](https://github.com/loonghao/rez-next/commit/f360f7d169c07def4f05626df3121a15c7f4a312))
* **deps:** update rust crate pyo3 to 0.28 ([861bbe3](https://github.com/loonghao/rez-next/commit/861bbe30ec88c77c2cdf102c28f8613155ff5172))
* **deps:** update rust crate whoami to v2 ([2749f2b](https://github.com/loonghao/rez-next/commit/2749f2b286229cbd20d19ad096fe5957ac364429))
* **docs,ci:** remove private-intra-doc-links in python_ast_parser; create venv before maturin develop in CI ([a02ffc7](https://github.com/loonghao/rez-next/commit/a02ffc736f2fba7217f7160bc33083ae2732ce61))
* **docs,ci:** strip=false in pyproject.toml; remove private-intra-doc-links in bind/mod.rs ([cd9e9dc](https://github.com/loonghao/rez-next/commit/cd9e9dcce6b9695be42400fc07df237541aaa719))
* **package:** Cycle 83 — migrate bincode 1.3 -&gt; 2.0 with serde compat API [iteration-done] ([e380396](https://github.com/loonghao/rez-next/commit/e380396ae664da23da23649cdc2a957dd08a3baa))
* **pip:** Cycle 39 — replace silent stubs with NotImplementedError; delete dead optimized_solver [iteration-done] ([e0046ff](https://github.com/loonghao/rez-next/commit/e0046ff02e119a1988ff5ebaf0fe6db015ed3684))
* **python:** ensure _native extension initialized before submodule import in all shim files ([5c702d6](https://github.com/loonghao/rez-next/commit/5c702d6f1aa0c4549cf5a47d54fdf6688d08e146))
* **repository:** Cycle 69 — SIMDPatternMatcher exact filename matching + 12 precision tests [iteration-done] ([97de9e9](https://github.com/loonghao/rez-next/commit/97de9e9b0b9b4a9ae07e94388a0b2408a92b96c6))
* **repository:** Cycle 83c — unify SimpleRepository multi-format scanning via PACKAGE_FILENAMES [iteration-done] ([53abfa1](https://github.com/loonghao/rez-next/commit/53abfa1effa6661607ca19f29a539f704075d8ab))
* **rex:** Cycle 62 — stop() now aborts action processing + split executor tests [iteration-done] ([c4ba991](https://github.com/loonghao/rez-next/commit/c4ba9910f87648a5a473af78e9823cb8e7cfaa0f))
* **solver:** RezCoreError::SolverError -&gt; Solver + test(solver,platform): add 12 new tests [iteration-done] ([2534c3b](https://github.com/loonghao/rez-next/commit/2534c3b447aefd38ac90dc0a8239048ce7f5d25d))
* **tests:** Cycle 38b — eliminate all clippy warnings (0 warnings) ([131a0bb](https://github.com/loonghao/rez-next/commit/131a0bb235504378ea8f028c4e029a00cf45ff58))
* **version:** Cycle 38 — implement rez-compatible alpha&lt;numeric token ordering [iteration-done] ([68cb73d](https://github.com/loonghao/rez-next/commit/68cb73db055e137124a968d204b426edb61fc7e4))


### ⚡ Performance

* **repository:** Cycle 64 — pre-compile exclude regexes in RepositoryScanner + sort list_packages/get_package_names + concurrent FS tests [iteration-done] ([1ef79ab](https://github.com/loonghao/rez-next/commit/1ef79abf189a742a89e85e3915fe1ffdade33d56))


### ♻️ Refactoring

* **bind:** Cycle 66 — split bind.rs (892L) into bind/{mod,detect,package_gen,utils,tests}.rs + 29 new boundary tests [iteration-done] ([f4fc0ca](https://github.com/loonghao/rez-next/commit/f4fc0caa6a4736be695181844c647e9c5fb932f7))
* **build:** Cycle 42 — split systems.rs (1329L) into 7 sub-modules [iteration-done] ([476facc](https://github.com/loonghao/rez-next/commit/476facccd9120921dd056482f1fdf8799b1066db))
* **build:** Cycle 80c — extract cmd_builder, remove shell-specific strings [iteration-done] ([a5b8b90](https://github.com/loonghao/rez-next/commit/a5b8b90b670d3b06230ceff41ec0b884311324b6))
* **cli:** Cycle 40 — extract expand_home/parse_timestamp to cli::utils [iteration-done] ([71ca72b](https://github.com/loonghao/rez-next/commit/71ca72bfe75c72874e6783193363b83c4bc07e2d))
* **cli:** Cycle 84 — split pkg_cache and search_v2 into focused submodules [iteration-done] ([5cebcf7](https://github.com/loonghao/rez-next/commit/5cebcf7a50e2b09b7b38a7441af25f24339ca486))
* **cli:** Cycle 85 — deduplicate path helpers in build.rs; expand filter.rs tests [iteration-done] ([f461318](https://github.com/loonghao/rez-next/commit/f4613187d93edbc020070842d10fd8fda52cc94d))
* **context:** Cycle 50 — split tests.rs (1374L) into 9 focused sub-modules [iteration-done] ([4aa3b1d](https://github.com/loonghao/rez-next/commit/4aa3b1d19155cdcb42e96c9161d8c50b334e71e6))
* **package,repository:** Cycle 65 — split oversized files into submodules + HashSet O(1) include_filenames in scanner [iteration-done] ([a3f103c](https://github.com/loonghao/rez-next/commit/a3f103c1816287eaf9ee519373f86efc343bf9b0))
* **package:** Cycle 41 — split python_ast_parser.rs (1395L) into 5 sub-modules [iteration-done] ([4357c16](https://github.com/loonghao/rez-next/commit/4357c163516f5569807001b99570081078a0d931))
* **package:** Cycle 53 — split serialization.rs (1454L) into 5 sub-modules [iteration-done] ([60613c9](https://github.com/loonghao/rez-next/commit/60613c91985fec3c4d26f8a41811100361e17ff4))
* **python:** Cycle 52 — split lib.rs (1655L→490L) into 6 focused function modules [iteration-done] ([f312c32](https://github.com/loonghao/rez-next/commit/f312c32e64fcfbf6deca471cea277c80a40fd948))
* **python:** Cycle 82 — shared Tokio runtime, fix get_tools() path, cross-platform REZ_CONTEXT_FILE [iteration-done] ([f688da0](https://github.com/loonghao/rez-next/commit/f688da01ac2ec11b97b505c52a9d571bca4c335f))
* **python:** Cycle 83b — extract shared runtime module, fix all per-call Runtime::new() in bindings [iteration-done] ([199dac4](https://github.com/loonghao/rez-next/commit/199dac40e158893bc89d4b2b7f153a5d08603fd0))
* **repository:** Cycle 70 — extract HP scanner tests + REZ_PACKAGE_FILENAMES DRY [iteration-done] ([ac60f2a](https://github.com/loonghao/rez-next/commit/ac60f2ad998dcde3b7fa2b49df24bf5dc09e7f51))
* **repository:** Cycle 80b — rename PrefetchPredictor tests to explicit smoke tests [iteration-done] ([d93c08e](https://github.com/loonghao/rez-next/commit/d93c08e8e3e82d861e7d89e9ca67f0abe628eb0e))
* **solver:** Cycle 51 — extract ResolutionState + split dependency_resolver tests (1260L→308L) [iteration-done] ([c3d50f3](https://github.com/loonghao/rez-next/commit/c3d50f38e1e94a966abea651cbc53df96b5e72c0))
* **tests:** Cycle 31+32 — split oversized test files into &lt;=1000-line modules [iteration-done] ([27dcc4b](https://github.com/loonghao/rez-next/commit/27dcc4b143c0523db5c9f4e88e91b19d61df9816))
* **tests:** Cycle 36 — extract shared build_test_repo into solver_helpers.rs [iteration-done] ([888a467](https://github.com/loonghao/rez-next/commit/888a46763b828dcd2c4dd876b0ed8352c872132b))
* **tests:** Cycle 71 — split 3 oversized integration test files [iteration-done] ([b4de88c](https://github.com/loonghao/rez-next/commit/b4de88c9d88829e3637973b0a3492bb8cae5a250))
* **tests:** Cycle 72 — split rez_compat_context_tests (985L-&gt;674L) + new rez_compat_context_bind_tests (336L, 13 tests) + accept cleanup-agent improvements [iteration-done] ([3ae4673](https://github.com/loonghao/rez-next/commit/3ae467375dc473e36a5975c906c2bd0ff76cb4ba))
* **tests:** Cycle 73 — split rez_compat_solver_tests (943L) into 3 focused files: solver (228L), package_commands (225L), requirement (316L) [iteration-done] ([132ec43](https://github.com/loonghao/rez-next/commit/132ec4381e6dc5166c31fc73daf2f4a4289938bd))
* **tests:** Cycle 74 — split real_repo_integration (1000L) into 3 focused files: scan+parse (363L), resolve (403L), context+e2e (407L) [iteration-done] ([dfa5d7f](https://github.com/loonghao/rez-next/commit/dfa5d7f4fa16ae3d78cd00b2946a68dac8482109))
* **tests:** Cycle 75 — split rez_compat_late_tests (942L) into 3 focused files: activation (451L), config (140L), diff_status (257L) [iteration-done] ([72430ad](https://github.com/loonghao/rez-next/commit/72430adb2aebc403e82cb14d1e6c7ed211dd757b))
* **tests:** Cycle 76 — split rez_solver_graph_tests (941L) + rez_solver_platform_tests (924L) into 4 focused files [iteration-done] ([41b84a0](https://github.com/loonghao/rez-next/commit/41b84a0a7c5e1b5ec12e75473fb4d33ec71d8b9a))
* **tests:** Cycle 77 — delete 3 empty migration shells + remove 4 overlapping cycle tests from compat [iteration-done] ([ac74b64](https://github.com/loonghao/rez-next/commit/ac74b644cfae0236ee6d288ac3875eb153d17efe))
* **tests:** Cycle 80 — extract shared real_repo helpers, expand .gitignore [iteration-done] ([718697b](https://github.com/loonghao/rez-next/commit/718697b0b5a37e9b1a3b2615f2bb3905788cbf30))
* **version:** Cycle 54 — split range.rs (1187L→767L) tests into range_tests.rs [iteration-done] ([9a09103](https://github.com/loonghao/rez-next/commit/9a09103e7c3cc803b967284107af9f21cfa635fb))

## [Unreleased]

### Tests

- **build/systems**: Add `#[derive(PartialEq)]` to `BuildStep` to enable `assert_eq!` in tests
- **build/systems/python**: Add mock tests for `PythonBuildSystem`
  - `test_configure_without_rezbuild_succeeds` — configure path when no rezbuild.py
  - `test_compile_no_build_files_skips_gracefully` — compile skip path
  - `test_package_always_succeeds` — static packaging result
  - `test_install_no_build_files_copies_source` — copy-files fallback in install
- **build/systems/nodejs**: Add mock tests for `NodeJsBuildSystem`
  - `test_package_always_succeeds` — static packaging result
  - `test_install_without_dist_copies_source` — install from source when no dist/
  - `test_install_with_dist_dir_copies_dist` — install from dist/ when present
- **build/systems/cargo_build**: Add tests for `CargoBuildSystem`
  - `test_package_returns_ok_regardless_of_cargo_availability` — package() never propagates Err
  - `test_compile_command_uses_release_flag` — release flag logic
  - `test_install_command_includes_release_flag` — install command flag
- **build/systems/mod**: Add 10 `detect` / `detect_with_package` tests using real `tempdir`
  - cmake, make, python (setup.py + pyproject.toml), nodejs, cargo marker-file detection
  - Custom build script priority over CMakeLists.txt
  - rezbuild.py priority over generic file-based detection
  - Explicit `build_system` and `build_command` override paths (nodejs, python, make, build_command)




## [0.1.5](https://github.com/loonghao/rez-next/compare/v0.1.4...v0.1.5) (2026-04-03)


### 🐛 Bug Fixes

* Fix:  ([49ad6c5](https://github.com/loonghao/rez-next/commit/49ad6c5ae3230d73b26b99d511bb1567185c5fd8))
* **ci:** eliminate duplicate release runs and enable PyPI wheel publishing ([49ad6c5](https://github.com/loonghao/rez-next/commit/49ad6c5ae3230d73b26b99d511bb1567185c5fd8))

## [0.1.4](https://github.com/loonghao/rez-next/compare/v0.1.3...v0.1.4) (2026-04-03)


### 🚀 Features

* **release:** squash merge auto-improve into v0.3.0 ([15df936](https://github.com/loonghao/rez-next/commit/15df9369f084309f1b742d9c5a1219d77d3b939c))


### 🐛 Bug Fixes

* **ci:** publish Python wheels to PyPI from release workflow ([35fafa6](https://github.com/loonghao/rez-next/commit/35fafa6023433e392cff91243dffdcdaa9d06000))
* **deps:** update rust crate sha2 to 0.11 ([215870b](https://github.com/loonghao/rez-next/commit/215870bef3aac47e1a11f9ed5702ecae38a6c76a))
* **deps:** update rust crate windows-sys to 0.61 ([cd1f2c2](https://github.com/loonghao/rez-next/commit/cd1f2c21ae18a2124714fd5e75cde14f0a51fe3c))
* **python:** align pymodule name with maturin module-name config ([f2a8b4c](https://github.com/loonghao/rez-next/commit/f2a8b4cfa0e6b386281fc99e3fce93137db17b05))
* **python:** register pyo3 submodules in sys.modules for dotted-path imports ([87cda78](https://github.com/loonghao/rez-next/commit/87cda787f61049840ff240cf549960ab73094cd2))

## [0.1.3](https://github.com/loonghao/rez-next/compare/v0.1.2...v0.1.3) (2026-04-03)


### 🚀 Features

* **bench:** automated performance comparison vs rez Python ([4e027ef](https://github.com/loonghao/rez-next/commit/4e027efe1ddaa51f0355f520fbeb429f06390890))
* comprehensive Rez core implementation with tests and honest documentation ([8095f04](https://github.com/loonghao/rez-next/commit/8095f0476cd445e7217d30b7297f144c44245510))
* comprehensive Rez Python API with 308 compatibility tests ([95cdae7](https://github.com/loonghao/rez-next/commit/95cdae7e8f1cd1243a9d8b7ff9fa55e818855690))
* **test:** add CLI e2e tests for rez-next binary ([b52b63f](https://github.com/loonghao/rez-next/commit/b52b63fc5838af5873ffcff1f40f46ac745809e6))


### 🐛 Bug Fixes

* **bench:** add CRITERION_QUICK mode to prevent CI timeout ([a20f1c7](https://github.com/loonghao/rez-next/commit/a20f1c7680b99a5b9cb820c7cdc7d4658472481f))
* **deps:** update rust crate base64 to 0.22 ([#74](https://github.com/loonghao/rez-next/issues/74)) ([d5fec05](https://github.com/loonghao/rez-next/commit/d5fec050a79f03ceb275379c90c941a29d8fcb52))
* **deps:** update rust crate toml to v1 ([#83](https://github.com/loonghao/rez-next/issues/83)) ([21815de](https://github.com/loonghao/rez-next/commit/21815dec920a72a2647efe77a20de5724cb1dab5))
* **fmt:** cargo fmt + pre-commit + extended Python tests ([cb21c0d](https://github.com/loonghao/rez-next/commit/cb21c0d7ec5c4eed269d29df033cecd00645a97b))
* **python:** fix config/system submodule overwrite causing ModuleNotFoundError ([a8b67bb](https://github.com/loonghao/rez-next/commit/a8b67bb27643edd4ce668b9f3dbf9352eb798ba9))
* **python:** register all submodules in sys.modules to fix dotted imports ([93b2eea](https://github.com/loonghao/rez-next/commit/93b2eea36825c8719d830f3d10d5dddf2ca5c6f2))

=======
>>>>>>> origin/auto-improve
## [0.2.0] - 2026-03-30

### Added
- **227 compat tests** (213 → 227): added 14 new integration tests covering:
  - Version range union of disjoint ranges (`test_version_range_union_disjoint`)
  - Pre-release version ordering with rez epoch semantics (`test_version_prerelease_ordering`)
  - Version range exclusive upper bound with rez semantics documentation (`test_version_range_exclusive_upper`)
  - Version range inclusive lower edge (`test_version_range_inclusive_lower_edge`)
  - Rex DSL: `unsetenv` removes variables (`test_rex_unsetenv_removes_var`)
  - Rex DSL: multiple `prepend_path` ordering (`test_rex_multiple_prepend_path_order`)
  - Rex DSL: bash script generation contains exports (`test_rex_bash_script_contains_export`)
  - Package name and version field validation (`test_package_name_non_empty`, `test_package_version_optional`)
  - Requirement name-only parsing (`test_requirement_name_only`)
  - Suite two-context tool management (`test_suite_two_contexts_tool_names`)
  - Suite initial empty status (`test_suite_initial_status`)
  - Solver empty requirements returns empty resolved packages (`test_solver_empty_requirements_returns_empty_package_list`)
  - Solver version conflict handling without panic (`test_solver_version_conflict_detected`)
- Version bump: all workspace crates updated from 0.1.0 to 0.2.0

### Fixed
- Documented rez version semantics: `3.0.1 < 3.0` (shorter = higher epoch), exclusive upper bound `<3.0` includes `3.0.1`



## [0.1.2](https://github.com/loonghao/rez-next/compare/v0.1.1...v0.1.2) (2026-03-28)


### 🐛 Bug Fixes

* replace reusable CI with custom workflow and add trigger-release-build ([6700dbc](https://github.com/loonghao/rez-next/commit/6700dbcf1e475089eb1d60d5c57a1b886784b098)), closes [#71](https://github.com/loonghao/rez-next/issues/71)
* resolve security audit vulnerabilities (RUSTSEC-2026-0007, RUSTSEC-2026-0002) ([8c92f1b](https://github.com/loonghao/rez-next/commit/8c92f1bd1002cfa8d390cbad01bbf535396dcdef))

## [0.1.1](https://github.com/loonghao/rez-next/compare/v0.1.0...v0.1.1) (2026-03-27)


### 🚀 Features

* Add comprehensive CI/CD configuration ([9ad9ac8](https://github.com/loonghao/rez-next/commit/9ad9ac81334cd67f4f6b4c7b09684072546057e1))
* add cross-platform release pipeline with install scripts ([#51](https://github.com/loonghao/rez-next/issues/51)) ([d18d81f](https://github.com/loonghao/rez-next/commit/d18d81fded479e00bee42585a00db14d5e8b236e))
* add experimental warning and clean up unnecessary files ([0d081c7](https://github.com/loonghao/rez-next/commit/0d081c76b7f358c0b348b3458be9faca713feb49))
* add flamegraph performance profiling support ([f9ad778](https://github.com/loonghao/rez-next/commit/f9ad778cddf8822927d09387a886f980d1039e7c))
* complete package.py parsing and prepare for crate.io release ([2c7678f](https://github.com/loonghao/rez-next/commit/2c7678f38b6bb243eba3e9c9c7552f25f80043b1))
* enable rez.exe executable build without Python dependencies ([1f539d5](https://github.com/loonghao/rez-next/commit/1f539d54dd372e9f0fc308013c7a7dd6ec70286f))
* implement comprehensive testing framework with ABI3 support ([cc5fa24](https://github.com/loonghao/rez-next/commit/cc5fa24321b4d367d2622a4b0781c43b840a8062))
* Initialize rez-core Rust project with MVP structure ([778ae5d](https://github.com/loonghao/rez-next/commit/778ae5dea4f1b79eab71798eba3f78c17ff1efa2))
* migrate to release-please and justfile (aligned with clawup) ([#62](https://github.com/loonghao/rez-next/issues/62)) ([afec281](https://github.com/loonghao/rez-next/commit/afec28182ba68a3fed5475968467530b54815399))
* rename project from rez-core to rez-next and add Python integration docs ([cd34c15](https://github.com/loonghao/rez-next/commit/cd34c153cc2ee285b6abf2699346f3afd9d7fcf1))
* setup crate publishing with release-plz automation ([d0751a5](https://github.com/loonghao/rez-next/commit/d0751a520097ab37f2077d6371d49b90b407cc59))
* setup Python bindings configuration ([eebe3e6](https://github.com/loonghao/rez-next/commit/eebe3e69a4f623cfe637b79a8385734cb48997f5))
* simplify CI/CD configuration following pydantic-core best practices ([c780b44](https://github.com/loonghao/rez-next/commit/c780b44bdd55e7c368ab28cefc7a02bee21f7bd8))


### 🐛 Bug Fixes

* add version numbers to internal dependencies and resolve package compilation issues ([2581b2d](https://github.com/loonghao/rez-next/commit/2581b2d89198b8b2a188dd4283c6222af7847b55))
* CI compilation errors and release workflow alignment with clawup ([#58](https://github.com/loonghao/rez-next/issues/58)) ([78b578e](https://github.com/loonghao/rez-next/commit/78b578e93bcb78eb044e19e706842b44be58867c))
* Fix Python code style issues ([2b6bccb](https://github.com/loonghao/rez-next/commit/2b6bccbe7eebedc5e400d65f9e3c8e24bd08be2d))
* Implement version parsing validation and resolve CI configuration issues ([bd122f0](https://github.com/loonghao/rez-next/commit/bd122f0e53fe37955887eac6f8124c6e0abfc36b))
* remove python-bindings feature and fix CI --all-features build ([#67](https://github.com/loonghao/rez-next/issues/67)) ([d6329db](https://github.com/loonghao/rez-next/commit/d6329dbb9476c462a7692f72c604734e5ae566aa))
* resolve all clippy warnings and enable workspace lint inheritance ([43f1f0a](https://github.com/loonghao/rez-next/commit/43f1f0a1938bf78ef31bfc8deb1ab0878bc75a29))
* resolve compilation errors and test failures in rez-next-version ([07b8866](https://github.com/loonghao/rez-next/commit/07b88665ee3b3f796cdf446ea4da5ebe9245bac1))
* resolve compilation errors and update CI configuration ([ac55367](https://github.com/loonghao/rez-next/commit/ac55367fabde003e9fd8ccc430ccd24e9d36a2e3))
* Resolve Python binding imports and update project structure ([3d744d0](https://github.com/loonghao/rez-next/commit/3d744d08a39de5bee664d0ca7999e21ec446e8d0))
* resolve release-plz configuration and python-bindings feature warnings ([c61ff56](https://github.com/loonghao/rez-next/commit/c61ff5615958b65caf7173b9022cebb19924c259))
* resolve test failures and warnings in rez-next-cache ([60b8885](https://github.com/loonghao/rez-next/commit/60b88850c008582da495157eabf37c7b92438bd1))
* Restore Python bindings and improve CI configuration ([6d10671](https://github.com/loonghao/rez-next/commit/6d106716a41c03333818b8ea5afb2a754afc5e23))
* shorten keywords to meet crates.io 20-character limit ([52528e4](https://github.com/loonghao/rez-next/commit/52528e437923f0bacb4234f3b19c0bf0b36e50a4))
* Update CI workflows and fix code style issues ([5505e18](https://github.com/loonghao/rez-next/commit/5505e18d4649aa49c705a7be61e19b639ecbe583))

## [0.1.0] - 2024-12-22

### Added
- Core Architecture: Modular crate ecosystem with workspace configuration
- Ultra-fast version parsing (117x improvement over Python rez)
- Advanced package.py parsing with RustPython AST
- Intelligent dependency resolution with A* heuristic algorithms
- Multi-level intelligent caching with predictive preheating
- Rex command language interpreter
- Python bindings via PyO3 with ABI3 compatibility (Python 3.8+)
- Cross-platform support (Windows, macOS, Linux)
