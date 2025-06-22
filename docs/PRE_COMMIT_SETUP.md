# Universal Pre-commit Configuration for Rust Projects

This document describes how to set up and use the universal pre-commit configuration for Rust projects.

## 🎯 Overview

Our pre-commit configuration is designed to be **universal** and **reusable** across any Rust project. It provides:

- ✅ **Fast execution** - Optimized for development workflow
- ✅ **Essential checks** - Core quality gates without bloat
- ✅ **Multi-language support** - Rust + Python scripts
- ✅ **Smart exclusions** - Automatically skips build artifacts

## 🚀 Quick Setup

### 1. Copy Configuration

Copy the `.pre-commit-config.yaml` file to your Rust project root:

```yaml
# Universal Pre-commit Configuration for Rust Projects
# Fast and essential checks for modern Rust development
# Suitable for any Rust project with optional Python tooling support

exclude: |
  (?x)^(
    target/.*|
    standalone_benchmark/target/.*|
    .*\.lock$|
    .*\.log$
  )$

repos:
  # Essential file checks
  - repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v5.0.0
    hooks:
      - id: trailing-whitespace
        exclude: '\.md$'
      - id: end-of-file-fixer
        exclude: '\.md$'
      - id: check-yaml
      - id: check-toml
      - id: check-merge-conflict
      - id: check-added-large-files
        args: ['--maxkb=1024']

  # Rust formatting and linting - core development tools
  - repo: https://github.com/doublify/pre-commit-rust
    rev: v1.0
    hooks:
      - id: fmt
        args: ['--manifest-path', 'Cargo.toml', '--']
      - id: clippy
        args: ['--manifest-path', 'Cargo.toml', '--all-targets', '--', '-D', 'warnings']

  # Python support for scripts (lightweight)
  - repo: https://github.com/astral-sh/ruff-pre-commit
    rev: v0.8.4
    hooks:
      - id: ruff
        args: [--fix]
        files: ^(python/|scripts/|tests/python/).*\.py$
      - id: ruff-format
        files: ^(python/|scripts/|tests/python/).*\.py$
```

### 2. Install Pre-commit

```bash
# Using uv (recommended)
uv tool install pre-commit

# Using pip
pip install pre-commit

# Using conda
conda install -c conda-forge pre-commit
```

### 3. Install Hooks

```bash
pre-commit install
```

### 4. Run Initial Check

```bash
pre-commit run --all-files
```

## 🔧 Configuration Details

### File Exclusions

The configuration automatically excludes:
- `target/` directories (Rust build artifacts)
- `*.lock` files (dependency locks)
- `*.log` files (log files)
- Custom benchmark targets

### Included Checks

#### Essential File Checks
- **Trailing whitespace** - Removes unnecessary whitespace
- **End of file fixer** - Ensures files end with newline
- **YAML/TOML validation** - Syntax checking for config files
- **Merge conflict detection** - Prevents accidental commits
- **Large file detection** - Warns about files >1MB

#### Rust Development Tools
- **cargo fmt** - Code formatting
- **cargo clippy** - Linting with warnings as errors

#### Python Script Support
- **ruff** - Fast Python linting and formatting
- **Scope**: Only affects `python/`, `scripts/`, `tests/python/` directories

## 🎨 Customization

### For Workspace Projects

If your project uses Cargo workspaces, the configuration works out of the box.

### For Single Crate Projects

No changes needed - the configuration detects project structure automatically.

### Adding Security Scanning

To add security scanning, uncomment and add:

```yaml
  # Security scanning (optional)
  - repo: https://github.com/gitguardian/ggshield
    rev: v1.32.1
    hooks:
      - id: ggshield
        language: python
        stages: [commit]
```

### Adding Commit Message Validation

To enforce conventional commits:

```yaml
  # Conventional commits (optional)
  - repo: https://github.com/compilerla/conventional-pre-commit
    rev: v3.6.0
    hooks:
      - id: conventional-pre-commit
        stages: [commit-msg]
```

## 🚨 Troubleshooting

### Pre-commit is Slow

1. **Check exclusions** - Ensure `target/` is excluded
2. **Update hooks** - Run `pre-commit autoupdate`
3. **Clean cache** - Run `pre-commit clean`

### Clippy Failures

1. **Fix warnings** - Address the specific warnings
2. **Temporary bypass** - Use `git commit --no-verify` (not recommended)
3. **Adjust rules** - Modify clippy args in configuration

### Python Errors (if no Python code)

Remove the Python section entirely:

```yaml
# Remove this entire section if no Python code
# - repo: https://github.com/astral-sh/ruff-pre-commit
#   ...
```

## 📋 Best Practices

### Development Workflow

1. **Make changes** to your Rust code
2. **Run tests** - `cargo test`
3. **Commit** - Pre-commit runs automatically
4. **Push** - All checks passed

### Team Setup

1. **Document requirements** - Add setup instructions to project README
2. **CI integration** - Run `pre-commit run --all-files` in CI
3. **Version pinning** - Pin pre-commit hook versions for consistency

### Performance Tips

- **Incremental runs** - Pre-commit only checks changed files
- **Parallel execution** - Hooks run in parallel when possible
- **Smart caching** - Results are cached between runs

## 🔄 Maintenance

### Updating Hooks

```bash
# Update all hooks to latest versions
pre-commit autoupdate

# Update specific hook
pre-commit autoupdate --repo https://github.com/doublify/pre-commit-rust
```

### Checking Configuration

```bash
# Validate configuration
pre-commit validate-config

# Show hook information
pre-commit run --help
```

## 📚 Additional Resources

- [Pre-commit Documentation](https://pre-commit.com/)
- [Rust Pre-commit Hooks](https://github.com/doublify/pre-commit-rust)
- [Ruff Documentation](https://docs.astral.sh/ruff/)
- [Conventional Commits](https://www.conventionalcommits.org/)

---

This configuration is battle-tested and ready for production use in any Rust project! 🦀
