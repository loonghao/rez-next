# Contributing to rez-next

Thank you for your interest in contributing.

## CI/CD

### CI Pipeline (`ci.yml`)

Runs on pushes to `main` / `develop` and pull requests targeting `main`:

- Toolchain pin consistency via `scripts/check_toolchain_pins.py`
- Formatting check via `vx just fmt-check`
- CI lint via `vx just lint-ci` (`clippy --workspace --all-targets --all-features -- -D warnings`)
- Docs check via `vx just doc-check`
- Workspace tests via `cargo test --workspace --exclude rez-next-python` on Linux/macOS/Windows
- CLI E2E via `vx just cli-e2e`
- API stability via `vx just semver-check`
- Packagability via `vx just package-check` (`cargo package --workspace`, tarball
  plus build verification, covering the top-level `rez-next` crate). This requires
  a clean working tree: Cargo exits 101 with `N files in the working directory
  contain changes` if anything is uncommitted, so commit or stash first.
- Security auditing via `rustsec/audit-check`
- Coverage via `cargo llvm-cov`
- Python binding tests via `maturin develop --release` + `pytest`

### Rust toolchain

`rust-toolchain.toml` is the single source of truth for the Rust toolchain. The
same version must appear in `Cargo.toml` (`rust-version`), `clippy.toml`
(`msrv`), and every `dtolnay/rust-toolchain@<version>` / `RUSTUP_TOOLCHAIN`
entry under `.github/workflows/`; `scripts/check_toolchain_pins.py` fails CI
when they disagree, and `just toolchain-check` runs the same check locally.

The toolchain always comes from rustup, never from `vx`: `rust` is deliberately
not listed in `vx.toml`, and the `justfile` calls `cargo` directly. Routing
cargo through `vx` made it install the `stable` channel at runtime and switch
the rustup default to it, so CI silently ran on a newer toolchain than the one
it pinned - any new upstream lint then turned unrelated pull requests red.

### Release

Automated via [release-please](https://github.com/googleapis/release-please). Multi-platform builds for Linux, macOS, and Windows.

## Development

### Setup

```bash
git clone https://github.com/loonghao/rez-next.git
cd rez-next
vx just build
vx just test
```

### Making changes

```bash
git checkout -b feature/your-feature
# edit code...
vx just ci    # run all checks
```

### Before submitting a PR

```bash
vx just fmt
vx just lint
vx just test
```

### PR requirements

- All tests pass
- Code formatted
- Clippy clean
- Security audits pass

## Troubleshooting

```bash
# Fix formatting
cargo fmt

# Fix clippy
cargo clippy --fix --workspace --all-targets --all-features

# Update audit DB
cargo audit --update-db
cargo deny check
```

## Links

- [Repository](https://github.com/loonghao/rez-next)
- [Issues](https://github.com/loonghao/rez-next/issues)
- [Benchmark Guide](./benchmark_guide.md)
- [Performance Guide](./performance.md)
