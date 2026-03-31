# rez-next development commands

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
    vx cargo test --workspace

# Run tests with output
test-verbose:
    vx cargo test --workspace -- --nocapture

# Run clippy lints (mirrors CI: --all-features --all-targets -D warnings)
lint:
    vx cargo clippy --workspace --all-targets --all-features -- -D warnings

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

# Check documentation builds without warnings
doc:
    vx cargo doc --workspace --all-features --no-deps

# Check documentation with warnings as errors
doc-check:
    RUSTDOCFLAGS="-D warnings" vx cargo doc --workspace --all-features --no-deps --document-private-items

# Run all CI checks locally (mirrors GitHub Actions)
ci: fmt-check lint doc-check test

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
    cd crates/rez-next-python && vx maturin develop --features pyo3/extension-module

# Run Python compatibility tests
py-test:
    cd crates/rez-next-python && vx pytest tests/ -v --tb=short

# Run Python compatibility tests (fast, stop on first failure)
py-test-fast:
    cd crates/rez-next-python && vx pytest tests/ -v --tb=short -x

# Run Python e2e tests only
py-test-e2e:
    cd crates/rez-next-python && vx pytest tests/ -v --tb=short -k "e2e or E2E or end_to_end"

# Run Python tests by module
py-test-module MODULE:
    cd crates/rez-next-python && vx pytest tests/ -v --tb=short -k "{{MODULE}}"

# Format Python test files with ruff
py-fmt:
    vx ruff format crates/rez-next-python/

# Lint Python test files with ruff
py-lint:
    vx ruff check crates/rez-next-python/

# Build wheel + run all Python tests (full Python CI flow)
py-ci: py-build py-test

# ── CLI E2E ────────────────────────────────────────────────────────────────

# Build the rez-next binary
build-bin:
    vx cargo build --bin rez-next

# Run CLI end-to-end tests (requires binary to be built first)
cli-e2e: build-bin
    REZ_NEXT_E2E_BINARY=target/debug/rez-next vx cargo test --test cli_e2e_tests -- --nocapture

# Run CLI e2e tests with release binary (faster)
cli-e2e-release:
    vx cargo build --release --bin rez-next
    REZ_NEXT_E2E_BINARY=target/release/rez-next vx cargo test --test cli_e2e_tests -- --nocapture

# Run a single CLI e2e test by name
cli-e2e-one TEST: build-bin
    REZ_NEXT_E2E_BINARY=target/debug/rez-next vx cargo test --test cli_e2e_tests {{TEST}} -- --nocapture

