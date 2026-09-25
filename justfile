# rez-next development commands

set windows-shell := ["pwsh.exe", "-NoLogo", "-NoProfile", "-Command"]

# Default recipe - show available commands
default:
    @just --list

# Build the project
build:
    cargo build

# Build in release mode
build-release:
    cargo build --release

# Run all tests
test:
    cargo test --workspace -- --test-threads=1

# Run tests with output
test-verbose:
    cargo test --workspace -- --test-threads=1 --nocapture

# Run clippy lints (local dev: all features, all targets)
lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Run the same complete Clippy gate in CI
lint-ci:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Check GitHub Actions workflows
actionlint:
    vx actionlint

# Format code
fmt:
    cargo fmt --all

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Run the CLI
run *ARGS:
    cargo run --bin rez-next -- {{ARGS}}

# Check everything (format, lint, test)
check: fmt-check lint test

# Run all CI checks locally (mirrors GitHub Actions)
ci: toolchain-check version-check benchmark-tools-test actionlint fmt-check lint-ci doc-check test

# Crates covered by the API stability contract (see docs/api-stability.md)
api_crates := "-p rez-next-common -p rez-next-version -p rez-next-package -p rez-next-repository -p rez-next-solver -p rez-next-context"

# Check the public API contract for breaking changes against a baseline revision
# Requires cargo-semver-checks; defaults to comparing against origin/main
semver-check REV="origin/main":
    cargo semver-checks check-release --baseline-rev "{{ REV }}" {{ api_crates }}

# Check that all release-managed package versions match
version-check:
    vx python scripts/check_release_versions.py

# Check that the Rust toolchain pin is identical everywhere (see rust-toolchain.toml)
toolchain-check:
    vx python scripts/check_toolchain_pins.py

# Validate the benchmark output parser and regression-gate contract
benchmark-tools-test:
    vx python -m unittest discover -s metrics/benchmarking/scripts -p "test_*.py" -v

# Check documentation builds without warnings
doc:
    cargo doc --workspace --all-features --no-deps

# Check documentation with warnings as errors
doc-check:
    cargo --config 'build.rustdocflags=["-D", "warnings"]' doc --workspace --all-features --no-deps --document-private-items

# Run benchmarks
bench:
    cargo bench --bench version_benchmark --bench package_benchmark --bench simple_package_benchmark

# Clean build artifacts
clean:
    cargo clean

# Install locally
install:
    cargo install --path .

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

# ── Packaging ──────────────────────────────────────────────────────────────

# Verify every workspace crate can be packaged for crates.io.
#
# `cargo package --workspace` builds a tarball per member and compiles it in
# isolation, which is the same verification `cargo publish` runs. Packaging the
# members in one invocation keeps their inter-crate path dependencies intact,
# so the top-level `rez-next` crate is verified even though its dependencies
# are not on crates.io yet. Never add `--no-verify` or `--allow-dirty`: the
# tarball build is the whole point of the check.
#
# A dirty working tree makes Cargo exit 101 with `error: N files in the working
# directory contain changes`, because the tarball would not match the committed
# source. Commit or stash before running this locally.
#
# Verify every workspace crate can be packaged (clean working tree required).
package-check:
    cargo package --workspace

# ── CLI E2E ────────────────────────────────────────────────────────────────

# Build the rez-next binary
build-bin:
    cargo build --bin rez-next

# Run CLI end-to-end tests (requires binary to be built first)
# Use CARGO_MANIFEST_DIR-based absolute path to avoid cwd issues on Linux
cli-e2e:
    cargo build --bin rez-next
    cargo test --test cli_e2e_tests -- --nocapture

# Run CLI e2e tests with release binary (faster)
cli-e2e-release:
    cargo build --release --bin rez-next
    cargo --config 'env.REZ_NEXT_E2E_BINARY="target/release/rez-next"' test --test cli_e2e_tests -- --nocapture

# Run a single CLI e2e test by name
cli-e2e-one TEST: build-bin
    cargo --config 'env.REZ_NEXT_E2E_BINARY="target/debug/rez-next"' test --test cli_e2e_tests {{TEST}} -- --nocapture

