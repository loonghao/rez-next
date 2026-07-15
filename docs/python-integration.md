# Python Integration Guide for rez-next

> **Overview**: This guide covers Python bindings, module coverage, and development workflow.

## Status

Python bindings expose a **curated compatibility surface** through the `rez_next` package. Documented common workflows are production-ready, but rez-next intentionally does not mirror every Rez internal API.

## Architecture

```
rez_next/                              # Main Python package
├── _native.*.pyd                      # PyO3 native extension (abi3-py38)
├── __init__.py                        # Supported exports and version
├── vendor/                            # rez_next.vendor subpackage
│   ├── __init__.py
│   └── version.py                     # rez_next.vendor.version
├── packages_.py                       # rez_next.packages_ (iter, get, copy, move, remove)
├── packages.py                        # rez_next.packages (object model)
├── resolved_context.py                # rez_next.resolved_context
├── config.py                          # rez_next.config (100+ fields)
├── system.py                          # rez_next.system
├── shell.py                           # rez_next.shell
├── rex.py                             # rez_next.rex
├── solver_.py / solver.py             # rez_next.solver
├── build_.py / build_plugins.py       # Build system
├── release.py                         # Package release
├── bind.py                            # System tool binding
├── pip.py                             # pip → rez conversion
├── plugins.py                         # Plugin management
├── env.py / source.py                 # Environment & activation
├── bundles.py / forward.py            # Context bundling & forward compat
├── cli.py / search.py / complete.py   # CLI tools
├── diff.py / status.py / depends.py   # Context diff, status, reverse deps
├── data.py / exceptions.py            # Data resources & exceptions
├── deprecations.py                    # Deprecation warnings
├── serialise_.py                      # Serialization
├── suite.py / test.py                 # Suite & test
├── util.py                            # General utilities
├── package_cache.py                   # Package cache
├── package_help.py                    # Package help
├── package_remove.py                  # Package removal
├── package_search.py                  # Package search
├── __main__.py                        # CLI entry points
└── ...
```

## Implemented Python Submodules

### Core Modules

| Submodule | Rez Equivalent | Functionality | Status |
|-----------|----------------|---------------|--------|
| `rez_next.version` | `rez.vendor.version` | Version parsing, comparison, ranges | ✅ Stable |
| `rez_next.packages_` | `rez.packages_` | Package iteration, queries, copy/move/remove | ✅ Stable |
| `rez_next.packages` | `rez.packages` | Package object model | ✅ Stable |
| `rez_next.resolved_context` | `rez.resolved_context` | Dependency resolution, context management | ✅ Stable |
| `rez_next.config` | `rez.config` | Configuration reading (100+ fields) | ✅ Stable |
| `rez_next.system` | `rez.system` | System info (platform, Python version, etc.) | ✅ Stable |
| `rez_next.exceptions` | `rez.exceptions` | Exception hierarchy | ✅ Stable |
| `rez_next.deprecations` | — | Deprecation warnings | ✅ Stable |

### Environment & Shell

| Submodule | Rez Equivalent | Functionality | Status |
|-----------|----------------|---------------|--------|
| `rez_next.shell` | `rez.shells` | Shell script generation (bash/zsh/fish/PowerShell/cmd) | ✅ Stable |
| `rez_next.rex` | `rez.rex` | Rex command-language interpreter | ✅ Stable |
| `rez_next.env` | `rez.env` | Environment creation and activation | ✅ Stable |
| `rez_next.source` | `rez.source` | Context activation script generation | ✅ Stable |
| `rez_next.forward` | `rez.forward` | Shell forward-compatibility scripts | ✅ Stable |

### Build & Release

| Submodule | Rez Equivalent | Functionality | Status |
|-----------|----------------|---------------|--------|
| `rez_next.build_` | `rez.build_` | Package build system integration | ✅ Stable |
| `rez_next.build_plugins` | `rez.build_plugins` | Build plugins | ✅ Stable |
| `rez_next.release` | `rez.release` | Package release workflow | ✅ Stable |
| `rez_next.bind` | `rez.bind` | Bind system tools as rez packages | ✅ Stable |
| `rez_next.pip` | `rez.pip` | Convert pip packages to rez packages | ✅ Stable |

### Package Management

| Submodule | Rez Equivalent | Functionality | Status |
|-----------|----------------|---------------|--------|
| `rez_next.suite` | `rez.suite` | Suite creation and tool-chain management | ✅ Stable |
| `rez_next.bundles` | `rez.bundles` | Context bundling (offline use) | ✅ Stable |
| `rez_next.package_cache` | `rez.package_cache` | Package payload caching | ✅ Stable |
| `rez_next.package_help` | `rez.package_help` | Package help | ✅ Stable |
| `rez_next.package_search` | `rez.package_search` | Package search API | ✅ Stable |
| `rez_next.package_remove` | `rez.package_remove` | Package removal | ✅ Stable |

### Solver & Context

| Submodule | Rez Equivalent | Functionality | Status |
|-----------|----------------|---------------|--------|
| `rez_next.solver_` | `rez.solver` | Dependency solver (partial) | ✅ Stable |
| `rez_next.solver` | `rez.solver` | Advanced solver API | ✅ Stable |
| `rez_next.serialise_` | `rez.serialise` | Serialization support | ✅ Stable |

### CLI & Tools

| Submodule | Rez Equivalent | Functionality | Status |
|-----------|----------------|---------------|--------|
| `rez_next.cli` | `rez.cli` | CLI entry-points (programmatic invocation) | ✅ Stable |
| `rez_next.search` | `rez.cli.search` | Package search (exact / contains / regex) | ✅ Stable |
| `rez_next.complete` | `rez.cli.complete` | Shell tab-completion script generation | ✅ Stable |
| `rez_next.diff` | `rez.cli.diff` | Diff two resolved contexts | ✅ Stable |
| `rez_next.status` | `rez.cli.status` | Query the currently active context | ✅ Stable |
| `rez_next.depends` | `rez.cli.depends` | Reverse-dependency queries | ✅ Stable |
| `rez_next.test` | `rez.test` | Package testing | ✅ Stable |

### Plugins

| Submodule | Rez Equivalent | Functionality | Status |
|-----------|----------------|---------------|--------|
| `rez_next.plugins` | `rez.plugins` | Plugin management | ✅ Stable |

### Utilities

| Submodule | Rez Equivalent | Functionality | Status |
|-----------|----------------|---------------|--------|
| `rez_next.util` | — | Curated native utility functions | ✅ Stable |
| `rez_next.data` | `rez.data` | Built-in data resources | ✅ Stable |
| `rez_next.vendor.version` | `rez.vendor.version` | Vendored version module | ✅ Stable |

The supported public modules are listed above; internal Rez utility modules are not mirrored.

## Quick Start

### Installation

```bash
# From PyPI
pip install rez-next

# From source (development)
git clone https://github.com/loonghao/rez-next
cd rez-next
maturin develop --release
```

### Basic Usage (Supported Interface)

```python
# Before (Rez)
import rez
from rez.packages_ import iter_packages, get_latest_package

# After (rez-next)
import rez_next as rez
from rez_next.packages_ import iter_packages, get_latest_package

# Supported top-level API
ctx = rez.resolve_packages(["python-3.9", "maya-2024"])
pkg = rez.get_latest_package("python")
for p in rez.iter_packages("maya"):
    print(p.name, p.version)
```

## API Examples

### Version Operations

```python
import rez_next as rez

# Parse and compare versions
v1 = rez.PyVersion("1.2.3")
v2 = rez.PyVersion("2.0.0")
print(v1 < v2)  # True

# Version ranges
r = rez.PyVersionRange(">=3.9,<4.0")
print(r.contains(v1))  # False

# Rez-compatible semantics
v3 = rez.PyVersion("1.0")
v4 = rez.PyVersion("1.0.0")
print(v3 > v4)  # True (Rez semantic)
```

### Package Queries

```python
from rez_next.packages_ import get_latest_package, iter_packages

# Get latest version
pkg = get_latest_package("python")
print(pkg.name, pkg.version)

# Iterate all versions
for p in iter_packages("maya", range_=">=2023"):
    print(p.version)
```

### Dependency Resolution

```python
from rez_next.resolved_context import resolve_packages

ctx = resolve_packages(["python-3.9", "maya-2024", "numpy-1.24"])
print(ctx.status)            # "solved"
print(ctx.resolved_packages)
```

### Context Diff

```python
from rez_next.diff import diff_contexts, format_diff

result = diff_contexts(
    ["python-3.9", "maya-2023"],
    ["python-3.11", "maya-2024", "houdini-20"]
)
print(f"Added: {result.num_added}")
print(format_diff(result))
```

### Reverse Dependencies

```python
from rez_next.depends import get_reverse_dependencies

result = get_reverse_dependencies("python", transitive=True)
print(result.format())
```

## Local Development

### Prerequisites

- Rust 1.95+ (MSRV in `Cargo.toml`)
- Python 3.8+
- Maturin (`pip install maturin`)
- (Optional) `vx` for tool management (see `vx.toml`)

### Build Commands

```bash
# Using just (recommended)
vx just py-build
vx just py-test
vx just py-ci

# Using maturin directly
cd crates/rez-next-python
vx maturin develop --features pyo3/extension-module
vx pytest tests/ -v --tb=short
```

### Development Workflow

```bash
# 1. Install in development mode
cd crates/rez-next-python
maturin develop --release

# 2. Run tests
pytest tests/ -v

# 3. Run Rust tests
cd ../../
cargo test --package rez-next-python

# 4. Check linting
vx just lint

# 5. Format code
vx just fmt
```

## Testing

### Python Tests

```bash
# Run all Python tests
pytest tests/ -v

# Run specific test file
pytest tests/test_version.py -v

# Run with coverage
pytest --cov=rez_next tests/
```

### Rust Tests (PyO3)

```bash
# Test Python bindings
cargo test --package rez-next-python

# Test all workspace
cargo test --workspace --exclude rez-next-python  # Pure Rust tests
```

## Performance

Benchmarks show 10-100x speedup compared to Python Rez:

| Operation | rez-next | Rez (Python) | Speedup |
|-----------|----------|--------------|---------|
| Version parse | ~9.1 us | ~250 us | ~27x |
| Package query | ~42 μs | ~5 ms | ~119x |
| Context resolve | ~100 ms | ~2 s | ~20x |

See [benchmark_guide.md](./benchmark_guide.md) for details.

## Known Limitations

1. **Not all Rez APIs implemented** - Check the table above for coverage
2. **Some advanced APIs are excluded** - Variant URI lookup, cached pure-Python resolver execution, and direct repository variant installation fail explicitly
3. **Pre-1.0 project** - API may change
4. **Python 3.8+ required** - For abi3-py38 support

## Troubleshooting

### ImportError: DLL load failed

```bash
# Reinstall maturin and rebuild
pip install maturin --upgrade
maturin develop --release
```

### Segfault in Python tests

```bash
# Debug with Rust logging
RUST_LOG=debug pytest tests/ -v
```

### Incompatible Python version

```bash
# Check Python version
python --version  # Must be 3.8+

# Rebuild with specific Python
maturin develop --release --python /path/to/python3.8
```

## Links

- **Main documentation**: See [AGENTS.md](../AGENTS.md) for project map
- **AI-friendly index**: See [llms.txt](../llms.txt) for concise API reference
- **Complete API reference**: See [llms-full.txt](../llms-full.txt) for full API details
- **Contributing**: See [contributing.md](./contributing.md)
- **Benchmarks**: See [benchmark_guide.md](./benchmark_guide.md)
- **Repository**: https://github.com/loonghao/rez-next

## License

[Apache License 2.0](../LICENSE)
