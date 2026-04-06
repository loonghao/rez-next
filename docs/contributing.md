# Contributing to rez-next

Thank you for your interest in contributing.

## CI/CD

### CI Pipeline (`ci.yml`)

Runs on pushes to `main` / `develop` and pull requests targeting `main`:

- Formatting check via `vx just fmt-check`
- CI lint via `vx just lint-ci` (`clippy --exclude rez-next-python -- -A warnings -D clippy::correctness`)
- Docs check via `vx just doc-check`
- Workspace tests via `vx cargo test --workspace --exclude rez-next-python` on Linux/macOS/Windows
- CLI E2E via `vx just cli-e2e`
- Security auditing via `rustsec/audit-check`
- Coverage via `cargo llvm-cov`
- Python binding tests via `maturin develop --release` + `pytest`

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
vx cargo fmt

# Fix clippy
vx cargo clippy --fix --workspace --all-targets --all-features

# Update audit DB
cargo audit --update-db
cargo deny check
```

## Links

- [Repository](https://github.com/loonghao/rez-next)
- [Issues](https://github.com/loonghao/rez-next/issues)
- [Benchmark Guide](./benchmark_guide.md)
- [Performance Guide](./performance.md)
