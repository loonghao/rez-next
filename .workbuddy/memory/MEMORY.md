# MEMORY.md - rez-core Project Context

## Project Overview
- **Name**: rez-next (binary), rez_core (library)
- **Repository**: https://github.com/loonghao/rez-next
- **Local path**: c:/github/rez-core
- **Author**: LongHao <hal.long@outlook.com>
- **Description**: Next-gen Rez package manager rewrite in Rust (experimental)
- **Version**: 0.1.0

## Architecture
- 8 workspace crates: common, version, package, solver, repository, context, build, cache
- CLI binary: `rez-next` with 18+ subcommands (env, build, solve, search, etc.)
- Uses: clap (CLI), tokio (async), serde (serialization), rayon (parallelism)

## CI/CD Pipeline
- **CI**: Uses `loonghao/rust-actions-toolkit/.github/workflows/reusable-ci.yml@v4`
  - Runs: rustfmt, clippy (-D warnings), docs, test (multi-platform), security audit, coverage
- **Release Please**: `googleapis/release-please-action@v4`, creates release PR on push to main
- **Release Build**: 8 targets (Linux gnu/musl x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64/aarch64)
- **Auto Merge**: Bot PRs (github-actions, release-please, renovate, dependabot) auto-merge
- **Install Scripts**: `install.sh` (Linux/macOS) and `install.ps1` (Windows) with SHA256 verification

## Key Decisions
- Workspace lint configuration via `[workspace.lints]` inherited by all crates (2026-03-26)
- Many style lints allowed since project is experimental
- `Version::compare()` method (renamed from `cmp()` to avoid Ord trait conflict)
- Python bindings currently disabled (PyO3 code commented out)

## Dev Tools
- Uses `vx` (environment manager) and `just` (command runner)
- justfile commands: build, test, lint, fmt, ci, bench, install
- `cargo clippy --workspace --all-targets -- -D warnings` must pass
