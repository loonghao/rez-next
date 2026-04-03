# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
