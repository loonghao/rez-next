# rez-next development commands

set windows-shell := ["pwsh.exe", "-NoLogo", "-NoProfile", "-Command"]

# Default recipe - show available commands
default:
    @just --list

# Build the project
build:
    vx cargo build

# Build in release mode
build-release:
    vx cargo build --release

# Run all tests
test:
    vx cargo test --workspace -- --test-threads=1

# Run tests with output
test-verbose:
    vx cargo test --workspace -- --test-threads=1 --nocapture

# Run clippy lints (local dev: all features, all targets)
lint:
    vx cargo clippy --workspace --all-targets --all-features -- -D warnings

# Run the same complete Clippy gate in CI
lint-ci:
    vx cargo clippy --workspace --all-targets --all-features -- -D warnings

# Check GitHub Actions workflows
actionlint:
    vx actionlint

# Format code
fmt:
    vx cargo fmt --all

# Check formatting
fmt-check:
    vx cargo fmt --all -- --check

# Run the CLI
run *ARGS:
    vx cargo run --bin rez-next -- {{ARGS}}

# Check everything (format, lint, test)
check: fmt-check lint test

# Run all CI checks locally (mirrors GitHub Actions)
ci: version-check actionlint fmt-check lint-ci doc-check test

# Check that all release-managed package versions match
version-check:
    vx python scripts/check_release_versions.py

# Check documentation builds without warnings
doc:
    vx cargo doc --workspace --all-features --no-deps

# Check documentation with warnings as errors
doc-check:
    vx cargo --config 'build.rustdocflags=["-D", "warnings"]' doc --workspace --all-features --no-deps --document-private-items

# Run benchmarks
bench:
    vx cargo bench --bench version_benchmark --bench package_benchmark --bench simple_package_benchmark

# Clean build artifacts
clean:
    vx cargo clean

# Install locally
install:
    vx cargo install --path .

# ── pre-commit ─────────────────────────────────────────────────────────────

# Install pre-commit hooks
pre-commit-install:
    vx pre-commit install

# Run pre-commit on all files (same as CI)
pre-commit:
    vx pre-commit run --all-files

# Run pre-commit on staged files only
pre-commit-staged:
    vx pre-commit run

# Update pre-commit hook versions
pre-commit-update:
    vx pre-commit autoupdate

# ── Python ─────────────────────────────────────────────────────────────────

# Build Python wheel with maturin develop (for local testing)
py-build:
    cd crates/rez-next-python && vx uv run --locked --extra test python -m maturin develop --features pyo3/extension-module

# Run Python compatibility tests
py-test:
    cd crates/rez-next-python && vx uv run --locked --extra test pytest tests/ -v --tb=short

# Run Python compatibility tests (fast, stop on first failure)
py-test-fast:
    cd crates/rez-next-python && vx uv run --locked --extra test pytest tests/ -v --tb=short -x

# Run Python e2e tests only
py-test-e2e:
    cd crates/rez-next-python && vx uv run --locked --extra test pytest tests/ -v --tb=short -k "e2e or E2E or end_to_end"

# Run Python tests by module
py-test-module MODULE:
    cd crates/rez-next-python && vx uv run --locked --extra test pytest tests/ -v --tb=short -k "{{MODULE}}"

# Format Python test files with ruff
py-fmt:
    vx ruff format crates/rez-next-python/

# Lint Python test files with ruff
py-lint:
    vx ruff check crates/rez-next-python/

# Build wheel + run lint and all Python tests (full Python CI flow)
py-ci: py-lint py-build py-test

# ── CLI E2E ────────────────────────────────────────────────────────────────

# Build the rez-next binary
build-bin:
    vx cargo build --bin rez-next

# Run CLI end-to-end tests (requires binary to be built first)
# Use CARGO_MANIFEST_DIR-based absolute path to avoid cwd issues on Linux
cli-e2e:
    vx cargo build --bin rez-next
    vx cargo test --test cli_e2e_tests -- --nocapture

# Run CLI e2e tests with release binary (faster)
cli-e2e-release:
    vx cargo build --release --bin rez-next
    vx cargo --config 'env.REZ_NEXT_E2E_BINARY="target/release/rez-next"' test --test cli_e2e_tests -- --nocapture

# Run a single CLI e2e test by name
cli-e2e-one TEST: build-bin
    vx cargo --config 'env.REZ_NEXT_E2E_BINARY="target/debug/rez-next"' test --test cli_e2e_tests {{TEST}} -- --nocapture

