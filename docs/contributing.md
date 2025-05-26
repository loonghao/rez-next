# Contributing to rez-core

Thank you for your interest in contributing to rez-core! This guide will help you understand our development workflow and CI/CD processes.

## 🔄 CI/CD Overview

We use a simplified CI/CD pipeline inspired by [pydantic-core](https://github.com/pydantic/pydantic-core) best practices. Our configuration consists of two main workflows:

### Main CI Pipeline (`ci.yml`)

Our continuous integration runs automatically on every push and pull request:

#### Jobs Overview
- **Coverage** - Code coverage analysis using `cargo-llvm-cov` and `pytest`
- **Test Python** - Multi-version Python testing (3.8-3.13, including freethreaded 3.13t)
- **Test OS** - Cross-platform testing (Ubuntu, macOS, Windows)
- **Test Rust** - Rust testing, formatting, linting, and benchmarks
- **Lint** - Code quality checks using project's linting tools
- **Audit** - Security auditing with `cargo-audit` and `cargo-deny`
- **Build** - Wheel building and installation testing

#### What Gets Tested
- ✅ **Python Compatibility**: Python 3.8, 3.9, 3.10, 3.11, 3.12, 3.13, 3.13t (freethreaded)
- ✅ **Operating Systems**: Ubuntu, macOS, Windows
- ✅ **Rust Code Quality**: `cargo fmt --check`, `cargo clippy`, `cargo test`
- ✅ **Python Code Quality**: `ruff`, `mypy`, `pytest`
- ✅ **Security**: `cargo audit`, `cargo deny`
- ✅ **Performance**: Benchmark execution (no regression testing yet)

### Release Pipeline (`release.yml`)

Automated release process triggered by git tags:

- **Multi-platform builds**: Linux, macOS, Windows
- **Multi-architecture**: x86_64 and aarch64 (where supported)
- **Source distribution**: Automated sdist creation
- **GitHub releases**: Automatic release creation with generated notes
- **PyPI publishing**: Automated publishing with trusted publishing

## 🛠️ Development Workflow

### 1. Local Development Setup

```bash
# Clone and setup
git clone https://github.com/loonghao/rez-core.git
cd rez-core
uv sync --all-extras

# Build development version
make build-dev                    # Unix/Linux/macOS
.\scripts\build.ps1 build-dev     # Windows
```

### 2. Making Changes

```bash
# Create feature branch
git checkout -b feature/your-feature-name

# Make your changes
# ... edit code ...

# Run local tests
make test                    # Unix/Linux/macOS
.\scripts\build.ps1 test     # Windows
```

### 3. Pre-commit Checks

Before submitting a pull request, ensure all local checks pass:

```bash
# Format code
make format                    # Unix/Linux/macOS
.\scripts\build.ps1 format     # Windows

# Run linting
make lint                      # Unix/Linux/macOS
.\scripts\build.ps1 lint       # Windows

# Run all tests
make test                      # Unix/Linux/macOS
.\scripts\build.ps1 test       # Windows

# Check Rust code
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

### 4. Submitting Pull Requests

1. **Push your branch** to your fork
2. **Create a pull request** with a clear description
3. **Wait for CI** - All CI checks must pass
4. **Address feedback** if any issues are found
5. **Merge** once approved and CI passes

## 🚨 CI/CD Requirements

### All PRs Must Pass
- ✅ **All tests** across Python 3.8-3.13 and all operating systems
- ✅ **Code formatting** (`cargo fmt`, `ruff format`)
- ✅ **Linting** (`cargo clippy`, `ruff check`, `mypy`)
- ✅ **Security audits** (`cargo audit`, `cargo deny`)
- ✅ **Build tests** (wheel creation and installation)

### Performance Considerations
- Benchmarks run in CI but don't fail builds yet
- Performance regression testing is planned for future implementation
- Use `make benchmark` locally to check performance impact

## 🔧 Troubleshooting CI Issues

### Common CI Failures

#### Python Test Failures
```bash
# Run specific Python version locally
uv run --python 3.8 pytest tests/python/
uv run --python 3.13 pytest tests/python/
```

#### Rust Formatting Issues
```bash
# Fix formatting
cargo fmt

# Check what would be changed
cargo fmt -- --check
```

#### Clippy Warnings
```bash
# Fix clippy issues
cargo clippy --fix --all-targets --all-features

# Check clippy without fixing
cargo clippy --all-targets --all-features -- -D warnings
```

#### Security Audit Failures
```bash
# Update advisory database
cargo audit --update-db

# Check for vulnerabilities
cargo audit

# Check licensing and other policies
cargo deny check
```

### Platform-Specific Issues

#### Windows
- Use PowerShell scripts: `.\scripts\build.ps1 <command>`
- Ensure Rust toolchain is properly installed
- Some tools may have different behavior on Windows

#### macOS
- Ensure Xcode command line tools are installed
- Some dependencies may require additional setup

#### Linux
- Most reliable platform for development
- All tools should work out of the box

## 📈 Performance Testing

### Local Benchmarking
```bash
# Python benchmarks
make benchmark                    # Unix/Linux/macOS
.\scripts\build.ps1 benchmark     # Windows

# Rust benchmarks
cargo bench

# Profiling (Linux/macOS recommended)
make flamegraph
```

### CI Performance Testing
- Benchmarks run automatically in CI
- Results are not currently used for pass/fail decisions
- Performance regression testing is planned

## 🎯 Best Practices

### Code Quality
- **Write tests** for all new functionality
- **Follow Rust conventions** for Rust code
- **Follow Python conventions** for Python code
- **Add documentation** for public APIs
- **Keep PRs focused** on single features/fixes

### CI/CD Efficiency
- **Run tests locally** before pushing
- **Fix formatting issues** before submitting PRs
- **Address security warnings** promptly
- **Keep builds fast** by avoiding unnecessary dependencies

### Communication
- **Clear PR descriptions** explaining what and why
- **Reference issues** when applicable
- **Respond to feedback** promptly
- **Ask questions** if anything is unclear

## 🔗 Useful Links

- [Main Repository](https://github.com/loonghao/rez-core)
- [Issues](https://github.com/loonghao/rez-core/issues)
- [pydantic-core CI Reference](https://github.com/pydantic/pydantic-core/blob/main/.github/workflows/ci.yml)
- [Performance Analysis Guide](./performance.md)

## 📞 Getting Help

If you encounter issues with the CI/CD pipeline or development setup:

1. **Check this documentation** first
2. **Search existing issues** for similar problems
3. **Create a new issue** with detailed information about your problem
4. **Include relevant logs** and error messages

We're here to help and appreciate your contributions! 🚀
