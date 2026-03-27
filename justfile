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
