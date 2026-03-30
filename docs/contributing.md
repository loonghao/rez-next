# Contributing to rez-next

Thank you for your interest in contributing.

## CI/CD

### CI Pipeline (`ci.yml`)

Runs on every push and pull request:

- Formatting check (`cargo fmt --check`)
- Linting (`cargo clippy --workspace --all-targets --all-features -- -D warnings`)
- All workspace tests (`cargo test --workspace`)
- Security auditing (`cargo audit`, `cargo deny`)

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
