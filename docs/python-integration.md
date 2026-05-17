# Python Integration Guide for rez-next

> **Overview**: This guide covers Python bindings, module coverage, and development workflow.

## Status

Python bindings are **partially implemented** in `crates/rez-next-python` and exposed through the `rez_next` Python package. Many common Rez workflows already work with `import rez_next`, but it is **not yet a seamless drop-in** for every API surface.

## Architecture

```
rez_next/
├── _native.*.pyd        # PyO3 native extension (abi3-py38)
├── __init__.py           # Exports, version, drop-in shims
├── version.py            # rez_next.version
├── packages_.py          # rez_next.packages_
├── resolved_context.py    # rez_next.resolved_context
├── solver_.py            # rez_next.solver (partial)
├── suite.py              # rez_next.suite
├── config.py             # rez_next.config
├── system.py             # rez_next.system
├── shell.py              # rez_next.shell
├── rex.py                # rez_next.rex
├── build_.py             # rez_next.build_ (partial)
├── release.py            # rez_next.release (partial)
├── bind.py               # rez_next.bind
├── search.py             # rez_next.search
├── diff.py               # rez_next.diff
├── depends.py            # rez_next.depends
├── status.py             # rez_next.status
├── complete.py           # rez_next.complete
└── ...                   # (18 submodules total)
```

## Implemented Python Submodules

| Submodule | Rez Equivalent | Functionality | Status |
|-----------|----------------|---------------|--------|
| `rez_next.version` | `rez.vendor.version` | Version parsing, comparison, ranges | ✅ Stable |
| `rez_next.packages_` | `rez.packages_` | Package iteration, queries, copy/move/remove | ✅ Stable |
| `rez_next.resolved_context` | `rez.resolved_context` | Dependency resolution, context management | ✅ Stable |
| `rez_next.suite` | `rez.suite` | Suite creation and tool-chain management | ✅ Stable |
| `rez_next.config` | `rez.config` | Configuration reading | ✅ Stable |
| `rez_next.system` | `rez.system` | System info (platform, Python version, etc.) | ✅ Stable |
| `rez_next.shell` | `rez.shells` | Shell script generation (bash/zsh/fish/PowerShell/cmd) | ✅ Stable |
| `rez_next.rex` | `rez.rex` | Rex command-language interpreter | ✅ Stable |
| `rez_next.build_` | `rez.build_` | Package build system integration | ✅ Stable |
| `rez_next.release` | `rez.release` | Package release workflow | ✅ Stable |
| `rez_next.bind` | `rez.bind` | Bind system tools as rez packages | ✅ Stable |
| `rez_next.pip` | `rez.pip` | Convert pip packages to rez packages | ✅ Stable |
| `rez_next.plugins` | `rez.plugins` | Plugin management | ✅ Stable |
| `rez_next.env` | `rez.env` | Environment creation and activation | ✅ Stable |
| `rez_next.source` | `rez.source` | Context activation script generation | ✅ Stable |
| `rez_next.bundles` | `rez.bundles` | Context bundling (offline use) | ✅ Stable |
| `rez_next.forward` | `rez.forward` | Shell forward-compatibility scripts | ✅ Stable |
| `rez_next.search` | `rez.cli.search` | Package search (exact / contains / regex) | ✅ Stable |
| `rez_next.complete` | `rez.cli.complete` | Shell tab-completion script generation | ✅ Stable |
| `rez_next.diff` | `rez.cli.diff` | Diff two resolved contexts | ✅ Stable |
| `rez_next.status` | `rez.cli.status` | Query the currently active context | ✅ Stable |
| `rez_next.depends` | `rez.cli.depends` | Reverse-dependency queries | ✅ Stable |
| `rez_next.data` | `rez.data` | Built-in data resources | ✅ Stable |
| `rez_next.cli` | `rez.cli` | CLI entry-points (programmatic invocation) | ✅ Stable |
| `rez_next.exceptions` | `rez.exceptions` | Exception hierarchy | ✅ Stable |
| `rez_next.utils.resources` | `rez.utils.resources` | Resource loading utilities | ✅ Stable |
| `rez_next.package_cache` | `rez.package_cache` | Package payload caching | ✅ Stable |
| `rez_next.solver_` | `rez.solver` | Dependency solver (partial) | ✅ Stable |
| `rez_next.package_filter` | `rez.package_filter` | Package filtering | ✅ Stable |
| `rez_next.package_test` | `rez.package_test` | Package testing | ✅ Stable |
| `rez_next.command` | `rez.command` | Command utilities | ✅ Stable |
| `rez_next.env` | `rez.env` | Environment creation and activation | ✅ Stable |
| `rez_next.source` | `rez.source` | Context activation script generation | ✅ Stable |
| `rez_next.bundles` | `rez.bundles` | Context bundling (offline use) | ✅ Stable |
| `rez_next.forward` | `rez.forward` | Shell forward-compatibility scripts | ✅ Stable |
| `rez_next.search` | `rez.cli.search` | Package search (exact / contains / regex) | ✅ Stable |
| `rez_next.complete` | `rez.cli.complete` | Shell tab-completion script generation | ✅ Stable |
| `rez_next.diff` | `rez.cli.diff` | Diff two resolved contexts | ✅ Stable |
| `rez_next.status` | `rez.cli.status` | Query the currently active context | ✅ Stable |
| `rez_next.depends` | `rez.cli.depends` | Reverse-dependency queries | ✅ Stable |
| `rez_next.data` | `rez.data` | Built-in data resources | ✅ Stable |
| `rez_next.cli` | `rez.cli` | CLI entry-points (programmatic invocation) | ✅ Stable |
| `rez_next.exceptions` | `rez.exceptions` | Exception hierarchy | ✅ Stable |
| `rez_next.utils.resources` | `rez.utils.resources` | Resource loading utilities | ✅ Stable |

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

### Basic Usage (Drop-in Replacement)

```python
# Before (Rez)
import rez
from rez.packages_ import iter_packages, get_latest_package

# After (rez-next)
import rez_next as rez
from rez_next.packages_ import iter_packages, get_latest_package

# API is fully compatible for implemented modules
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

- Rust 1.70+
- Python 3.8+
- Maturin (`pip install maturin`)
- (Optional) `vx` for tool management

### Build Commands

```bash
# Using just (recommended)
just py-build
just py-test

# Using maturin directly
cd crates/rez-next-python
vx maturin develop
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
2. **Some advanced features missing** - Build system, release workflow partial
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
- **API reference**: See [llms-full.txt](../llms-full.txt) for complete API
- **Contributing**: See [contributing.md](./contributing.md)
- **Benchmarks**: See [benchmark_guide.md](./benchmark_guide.md)
- **Repository**: https://github.com/loonghao/rez-next

## License

[Apache License 2.0](../LICENSE)
