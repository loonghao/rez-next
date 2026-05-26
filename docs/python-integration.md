# Python Integration Guide for rez-next

> **Overview**: This guide covers Python bindings, module coverage, and development workflow.

## Status

Python bindings are **partially implemented** in `crates/rez-next-python` and exposed through the `rez_next` Python package. Many common Rez workflows already work with `import rez_next`, but it is **not yet a seamless drop-in** for every API surface.

## Architecture

```
rez_next/
├── _native.*.pyd          # PyO3 native extension (abi3-py38)
├── __init__.py             # Exports, version, drop-in shims
├── version.py              # rez_next.version
├── packages_.py            # rez_next.packages_
├── packages.py             # rez_next.packages
├── resolved_context.py      # rez_next.resolved_context
├── solver_.py              # rez_next.solver
├── solver.py               # rez_next.solver (advanced)
├── suite.py                # rez_next.suite
├── config.py               # rez_next.config (100+ fields)
├── system.py               # rez_next.system
├── shell.py                # rez_next.shell
├── rex.py                  # rez_next.rex
├── build_.py               # rez_next.build_
├── build_process.py        # rez_next.build_process
├── build_system.py         # rez_next.build_system
├── build_plugins.py        # rez_next.build_plugins
├── release.py              # rez_next.release
├── release_hook.py         # rez_next.release_hook
├── release_vcs.py          # rez_next.release_vcs
├── bind.py                 # rez_next.bind
├── pip.py                  # rez_next.pip
├── plugins.py              # rez_next.plugins
├── plugin_managers.py      # rez_next.plugin_managers
├── env.py                  # rez_next.env
├── source.py               # rez_next.source
├── bundles.py              # rez_next.bundles
├── bundle_context.py       # rez_next.bundle_context
├── forward.py              # rez_next.forward
├── search.py               # rez_next.search
├── complete.py             # rez_next.complete
├── diff.py                 # rez_next.diff
├── depends.py              # rez_next.depends
├── status.py               # rez_next.status
├── data.py                 # rez_next.data
├── cli.py                  # rez_next.cli
├── exceptions.py           # rez_next.exceptions
├── deprecations.py         # rez_next.deprecations
├── package_cache.py        # rez_next.package_cache
├── package_help.py         # rez_next.package_help
├── package_maker.py        # rez_next.package_maker
├── package_repository.py   # rez_next.package_repository
├── package_search.py       # rez_next.package_search
├── package_remove.py       # rez_next.package_remove
├── package_py_utils.py     # rez_next.package_py_utils
├── serialise_.py           # rez_next.serialise_
├── test.py                 # rez_next.test
├── util.py                 # rez_next.util
├── command.py              # rez_next.command
├── wrapper.py              # rez_next.wrapper
├── resolver.py             # rez_next.resolver
├── utils/                  # rez_next.utils subpackage
│   ├── __init__.py
│   ├── colorize.py
│   ├── data_utils.py
│   ├── filesystem.py
│   ├── formatting.py
│   ├── logging_.py
│   ├── platform_.py
│   ├── resources.py
│   └── yaml.py
├── vendor/                 # rez_next.vendor subpackage
│   ├── __init__.py
│   └── version.py
└── ...                     # (67 .py files, 56+ submodules)
```

## Implemented Python Submodules

| Submodule | Rez Equivalent | Functionality | Status |
|-----------|----------------|---------------|--------|
| `rez_next.version` | `rez.vendor.version` | Version parsing, comparison, ranges | ✅ Stable |
| `rez_next.packages_` | `rez.packages_` | Package iteration, queries, copy/move/remove | ✅ Stable |
| `rez_next.packages` | `rez.packages` | Package object model | ✅ Stable |
| `rez_next.resolved_context` | `rez.resolved_context` | Dependency resolution, context management | ✅ Stable |
| `rez_next.suite` | `rez.suite` | Suite creation and tool-chain management | ✅ Stable |
| `rez_next.config` | `rez.config` | Configuration reading | ✅ Stable |
| `rez_next.system` | `rez.system` | System info (platform, Python version, etc.) | ✅ Stable |
| `rez_next.shell` | `rez.shells` | Shell script generation (bash/zsh/fish/PowerShell/cmd) | ✅ Stable |
| `rez_next.rex` | `rez.rex` | Rex command-language interpreter | ✅ Stable |
| `rez_next.build_` | `rez.build_` | Package build system integration | ✅ Stable |
| `rez_next.build_plugins` | `rez.build_plugins` | Build plugins | ✅ Stable |
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
| `rez_next.deprecations` | `rez.utils.deprecations` | Deprecation warnings | ✅ Stable |
| `rez_next.package_cache` | `rez.package_cache` | Package payload caching | ✅ Stable |
| `rez_next.package_help` | `rez.package_help` | Package help | ✅ Stable |
| `rez_next.package_search` | `rez.package_search` | Package search API | ✅ Stable |
| `rez_next.package_remove` | `rez.package_remove` | Package removal | ✅ Stable |
| `rez_next.solver_` | `rez.solver` | Dependency solver (partial) | ✅ Stable |
| `rez_next.solver` | `rez.solver` | Advanced solver API | ✅ Stable |
| `rez_next.serialise_` | `rez.serialise` | Serialization support | ✅ Stable |
| `rez_next.test` | `rez.test` | Package testing | ✅ Stable |
| `rez_next.util` | `rez.utils` | Utility functions | ✅ Stable |
| `rez_next.package_maker` | `rez.package_maker` | Programmatic package creation | ✅ Stable |
| `rez_next.package_repository` | `rez.package_repository` | Package repository abstraction | ✅ Stable |
| `rez_next.package_py_utils` | `rez.package_py_utils` | package.py utilities | ✅ Stable |
| `rez_next.build_process` | `rez.build_process` | Build process orchestration | ✅ Stable |
| `rez_next.build_system` | `rez.build_system` | Build system abstraction | ✅ Stable |
| `rez_next.release_hook` | `rez.release_hook` | Release hooks | ✅ Stable |
| `rez_next.release_vcs` | `rez.release_vcs` | VCS release integration | ✅ Stable |
| `rez_next.wrapper` | `rez.utils.wrapper` | Tool execution wrappers | ✅ Stable |
| `rez_next.bundle_context` | `rez.bundle_context` | Relocatable context bundles | ✅ Stable |
| `rez_next.command` | `rez.utils.command` | Command execution | ✅ Stable |
| `rez_next.resolver` | `rez.resolver` | Package resolver | ✅ Stable |
| `rez_next.plugin_managers` | `rez.plugin_managers` | Plugin manager implementations | ✅ Stable |
| `rez_next.utils.filesystem` | `rez.utils.filesystem` | Filesystem utilities | ✅ Stable |
| `rez_next.utils.formatting` | `rez.utils.formatting` | Output formatting | ✅ Stable |
| `rez_next.utils.logging_` | `rez.utils.logging_` | Logging utilities | ✅ Stable |
| `rez_next.utils.yaml` | `rez.utils.yaml` | YAML utilities | ✅ Stable |
| `rez_next.utils.resources` | `rez.utils.resources` | Resource loading utilities | ✅ Stable |
| `rez_next.utils.colorize` | `rez.utils.colorize` | Terminal color output | ✅ Stable |
| `rez_next.utils.data_utils` | `rez.utils.data_utils` | Data file helpers | ✅ Stable |
| `rez_next.utils.platform_` | `rez.utils.platform_` | Platform detection utilities | ✅ Stable |
| `rez_next.vendor.version` | `rez.vendor.version` | Vendored version module | ✅ Stable |
| `rez_next.package_copy` | `rez.package_copy` | Package copy operations | ✅ Stable |
| `rez_next.package_move` | `rez.package_move` | Package move operations | ✅ Stable |
| `rez_next.package_order` | `rez.package_order` | Package ordering strategies | ✅ Stable |
| `rez_next.package_bind` | `rez.package_bind` | Package bind utilities | ✅ Stable |
| `rez_next.package_resources` | `rez.package_resources` | Package resource management | ✅ Stable |
| `rez_next.package_serialise` | `rez.package_serialise` | Package serialization | ✅ Stable |
| `rez_next.package_filter` | `rez.package_filter` | Package filter rules | ✅ Stable |
| `rez_next.package_test` | `rez.package_test` | Package test runner | ✅ Stable |
| `rez_next.developer_package` | `rez.developer_package` | Developer package support | ✅ Stable |
| `rez_next.rex_bindings` | `rez.rex_bindings` | Rex low-level bindings | ✅ Stable |
| `rez_next.shells` | `rez.shells` | Shell type registry | ✅ Stable |
| `rez_next.rezconfig` | `rez.rezconfig` | Config defaults module | ✅ Stable |

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
- **AI-friendly index**: See [llms.txt](../llms.txt) for concise API reference
- **Complete API reference**: See [llms-full.txt](../llms-full.txt) for full API details
- **Contributing**: See [contributing.md](./contributing.md)
- **Benchmarks**: See [benchmark_guide.md](./benchmark_guide.md)
- **Repository**: https://github.com/loonghao/rez-next

## License

[Apache License 2.0](../LICENSE)
