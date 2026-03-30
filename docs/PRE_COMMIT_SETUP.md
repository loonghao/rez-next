# Pre-commit Configuration

How to set up pre-commit hooks for this Rust project.

## Overview

The `.pre-commit-config.yaml` provides:

- Fast execution — optimized for development
- Essential checks — formatting, linting, config validation
- Rust + Python support — cargo fmt/clippy + ruff

## Setup

### 1. Install pre-commit

```bash
uv tool install pre-commit
# or: pip install pre-commit
```

### 2. Install hooks

```bash
pre-commit install
```

### 3. Run

```bash
pre-commit run --all-files
```

## What gets checked

### File checks (pre-commit-hooks)

- Trailing whitespace (except .md)
- End-of-file newline
- YAML/TOML syntax
- Merge conflict markers
- Large files (>1MB)

### Rust (pre-commit-rust)

- `cargo fmt` — formatting
- `cargo clippy` — linting with `-D warnings`

### Python (ruff)

- Linting and formatting for `python/`, `scripts/`, `tests/python/`

## Configuration

See `.pre-commit-config.yaml` in the repo root.

For workspace projects, the configuration works out of the box.

## Maintenance

```bash
pre-commit autoupdate
```

## Troubleshooting

- Slow? Check that `target/` is excluded.
- Clippy failures? Fix warnings or run `cargo clippy --fix`.
- No Python code? Remove the ruff section from the config.

## References

- [Pre-commit](https://pre-commit.com/)
- [Rust pre-commit hooks](https://github.com/doublify/pre-commit-rust)
- [Ruff](https://docs.astral.sh/ruff/)
