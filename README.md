# rez-next

[![Rust](https://img.shields.io/badge/rust-1.95+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![CI](https://img.shields.io/github/actions/workflow/status/loonghao/rez-next/ci.yml?branch=main)](https://github.com/loonghao/rez-next/actions)
[![GitHub Release](https://img.shields.io/github/v/release/loonghao/rez-next)](https://github.com/loonghao/rez-next/releases)
[![Crates.io](https://img.shields.io/crates/v/rez-next)](https://crates.io/crates/rez-next)
[![Crates.io Downloads](https://img.shields.io/crates/d/rez-next)](https://crates.io/crates/rez-next)
[![PyPI - Version](https://img.shields.io/pypi/v/rez-next)](https://pypi.org/project/rez-next/)
[![PyPI - Downloads](https://img.shields.io/pypi/dm/rez-next)](https://pypi.org/project/rez-next/)
[![PyPI - Python Version](https://img.shields.io/pypi/pyversions/rez-next)](https://pypi.org/project/rez-next/)
[![Coverage](https://img.shields.io/codecov/c/gh/loonghao/rez-next/main)](https://codecov.io/gh/loonghao/rez-next)

> **Production-ready scope.** Documented common workflows are production-ready when a released version is pinned and validated against your package corpus. rez-next is pre-1.0, uses curated compatibility, is **not** a seamless replacement for every Rez API, and is not an official AcademySoftwareFoundation project.

A high-performance Rust implementation of common [Rez](https://github.com/AcademySoftwareFoundation/rez) package-management workflows with Python bindings. Unsupported compatibility surfaces fail explicitly instead of reporting false success. See [Production readiness](docs/production-readiness.md) for the supported surface, exclusions, release gates, and adoption checklist.

[English](README.md) | [中文](README_zh.md)

---

## Installation

### Linux / macOS

```bash
curl -fsSL https://raw.githubusercontent.com/loonghao/rez-next/main/install.sh | sh
```

Or with a specific version:

```bash
REZ_NEXT_VERSION=0.3.5 curl -fsSL https://raw.githubusercontent.com/loonghao/rez-next/main/install.sh | sh # x-release-please-version
```

Environment variables:

| Variable | Description | Default |
|---|---|---|
| `REZ_NEXT_VERSION` | Version to install (e.g. `0.3.5`) | latest | <!-- x-release-please-version -->
| `REZ_NEXT_INSTALL` | Installation directory | `$HOME/.rez-next/bin` |
| `REZ_NEXT_MUSL` | Force musl build on Linux | auto-detect |

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/loonghao/rez-next/main/install.ps1 | iex
```

### Python (PyPI)

```bash
pip install rez-next
```

### Build from Source

```bash
git clone https://github.com/loonghao/rez-next
cd rez-next
cargo build --release
```

---

## Self-Update

Keep rez-next up to date with the built-in `self-update` command:

```bash
# Update to the latest release
rez-next self-update

# Check for updates without installing
rez-next self-update --check

# Update to a specific version
rez-next self-update --version 0.3.5 # x-release-please-version

# Force reinstall of the current version
rez-next self-update --force
```

---

## Quick Start

```python
# Before
import rez
from rez.packages_ import iter_packages, get_latest_package
from rez.resolved_context import ResolvedContext

# After (supported interface)
import rez_next as rez
from rez_next.packages_ import iter_packages, get_latest_package
from rez_next.resolved_context import ResolvedContext

# Supported top-level API
ctx = rez.resolve_packages(["python-3.9", "maya-2024"])
pkg = rez.get_latest_package("python")
for p in rez.iter_packages("maya"):
    print(p.name, p.version)
```

---

## Feature Overview

### Implemented Python Submodules

| Submodule | Equivalent rez module | Functionality |
|-----------|----------------------|---------------|
| `rez_next.version` | `rez.vendor.version` | Version parsing, comparison, ranges |
| `rez_next.packages_` | `rez.packages_` | Package iteration, queries, copy/move/remove |
| `rez_next.packages` | `rez.packages` | Package object model |
| `rez_next.resolved_context` | `rez.resolved_context` | Dependency resolution, context management |
| `rez_next.suite` | `rez.suite` | Suite creation and tool-chain management |
| `rez_next.config` | `rez.config` | Configuration reading (100+ fields) |
| `rez_next.system` | `rez.system` | System info (platform, Python version, etc.) |
| `rez_next.shell` | `rez.shells` | Shell script generation (bash/zsh/fish/PowerShell/cmd) |
| `rez_next.rex` | `rez.rex` | Rex command-language interpreter |
| `rez_next.build_` | `rez.build_` | Package build system integration |
| `rez_next.build_plugins` | `rez.build_plugins` | Build plugins |
| `rez_next.release` | `rez.release` | Package release workflow |
| `rez_next.bind` | `rez.bind` | Bind system tools as rez packages |
| `rez_next.pip` | `rez.pip` | Convert pip packages to rez packages |
| `rez_next.plugins` | `rez.plugins` | Plugin management |
| `rez_next.env` | `rez.env` | Environment creation and activation |
| `rez_next.source` | `rez.source` | Context activation script generation |
| `rez_next.bundles` | `rez.bundles` | Context bundling (offline use) |
| `rez_next.forward` | `rez.forward` | Shell forward-compatibility scripts |
| `rez_next.search` | `rez.cli.search` | Package search (exact / contains / regex) |
| `rez_next.complete` | `rez.cli.complete` | Shell tab-completion script generation |
| `rez_next.diff` | `rez.cli.diff` | Diff two resolved contexts |
| `rez_next.status` | `rez.cli.status` | Query the currently active context |
| `rez_next.depends` | `rez.cli.depends` | Reverse-dependency queries |
| `rez_next.data` | `rez.data` | Built-in data resources |
| `rez_next.cli` | `rez.cli` | CLI entry-points (programmatic invocation) |
| `rez_next.exceptions` | `rez.exceptions` | Exception hierarchy |
| `rez_next.deprecations` | — | Deprecation warnings |
| `rez_next.package_cache` | `rez.package_cache` | Package payload cache |
| `rez_next.package_help` | `rez.package_help` | Package help |
| `rez_next.package_search` | `rez.package_search` | Package search API |
| `rez_next.package_remove` | `rez.package_remove` | Package removal |
| `rez_next.solver_` | `rez.solver` | Dependency solver (partial) |
| `rez_next.solver` | `rez.solver` | Advanced solver API |
| `rez_next.serialise_` | `rez.serialise` | Serialization support |
| `rez_next.test` | `rez.test` | Package testing |
| `rez_next.util` | — | Curated native utility functions |
| `rez_next.vendor.version` | `rez.vendor.version` | Vendored version module |

### API Examples

#### Version operations

```python
import rez_next as rez

# Parse and compare versions
v1 = rez.PyVersion("1.2.3")
v2 = rez.PyVersion("2.0.0")
print(v1 < v2)  # True

# Version ranges
r = rez.PyVersionRange(">=3.9,<4.0")
print(r.contains(v1))  # False
```

#### Package queries

```python
import rez_next as rez

# Get the latest version
pkg = rez.get_latest_package("python")
print(pkg.name, pkg.version)

# Iterate all versions
for p in rez.iter_packages("maya", range_=">=2023"):
    print(p.version)
```

#### Dependency resolution

```python
import rez_next as rez

ctx = rez.resolve_packages(["python-3.9", "maya-2024", "numpy-1.24"])
print(ctx.status)            # "solved"
print(ctx.resolved_packages)
```

#### Context diff (`rez.diff`)

```python
from rez_next.diff import diff_contexts, format_diff

result = diff_contexts(
    ["python-3.9", "maya-2023"],
    ["python-3.11", "maya-2024", "houdini-20"]
)
print(f"Added: {result.num_added}, Upgraded: {result.num_upgraded}")
print(format_diff(result))
# Output:
#   + houdini 20
#   ^ python 3.9 -> 3.11
#   ^ maya 2023 -> 2024
```

#### Reverse-dependency queries (`rez.depends`)

```python
from rez_next.depends import get_reverse_dependencies, print_depends

# Find all packages that depend on python
result = get_reverse_dependencies("python", transitive=True)
print(result.format())
# Output:
#   Reverse dependencies for 'python':
#     Direct:
#       maya-2024.1  (requires 'python-3.9')
#       houdini-20.0  (requires 'python-3.10')
#     Transitive:
#       nuke-14.0  (requires 'maya-2024')
```

#### Active context status (`rez.status`)

```python
from rez_next.status import get_current_status, is_in_rez_context

if is_in_rez_context():
    status = get_current_status()
    print(f"Active packages: {status.resolved_packages}")
    print(f"Shell: {status.current_shell}")
```

#### Package search (`rez.search`)

```python
from rez_next.search import search_packages, search_package_names

# Search for all maya-related packages
results = search_packages("maya")
for r in results:
    print(r.name, r.version)

# Return name list only
names = search_package_names("^py")  # supports regex
```

#### Shell completion (`rez.complete`)

```python
from rez_next.complete import get_completion_script

# Generate bash completion script
script = get_completion_script("bash")
print(script)
```

---

## Architecture

A Cargo workspace with 20 crates, including `rez-next-python` for Python bindings:

```
rez-next-common        Shared error types, config, utilities
rez-next-config        Config loading & validation
rez-next-version       Version parsing, comparison, ranges (state-machine parser)
rez-next-package       Package definition, package.py parsing (RustPython AST)
rez-next-package-cache Package payload caching
rez-next-package-filter Package filter (glob, regex, range rules)
rez-next-solver        Dependency resolution (A* algorithm + backtracking + cycle detection)
rez-next-repository    Repository scanning and caching
rez-next-context       Resolved context, Rex integration, serialization
rez-next-build         Build system integration (cmake/make/python/cargo/nodejs)
rez-next-cache         Multi-level caching (memory + disk)
rez-next-rex           Rex command language (full DSL + 5 shell activation scripts)
rez-next-suites        Suite management (collections of resolved contexts)
rez-next-bind          Bind system tools (python/cmake/pip/git, etc.)
rez-next-search        Package search (exact / contains / regex FilterMode)
rez-next-explicit      Explicit package lists
rez-next-serialise     Package serialization
rez-next-release-hook  Release hooks
rez-next-util          Utility functions (command runner, etc.)
rez-next-python        Python bindings via PyO3 (40 submodules)
```

### Component status

| Crate | Status | Tests |
|-------|--------|-------|
| `rez-next-version` | Mature core | ~30 |
| `rez-next-package` | Mature core | ~25 |
| `rez-next-common` | Mature core | ~10 |
| `rez-next-config` | Stable | ~8 |
| `rez-next-rex` | Mature core | ~20 |
| `rez-next-solver` | Active development (A* enabled) | ~15 |
| `rez-next-context` | Active development | ~12 |
| `rez-next-repository` | Mature core | ~8 |
| `rez-next-build` | Supported build/release workflows | ~6 |
| `rez-next-cache` | Active development | ~5 |
| `rez-next-suites` | Active development | ~10 |
| `rez-next-bind` | Active development | ~37 |
| `rez-next-search` | Active development | ~16 |
| `rez-next-package-cache` | Stable | ~8 |
| `rez-next-package-filter` | Stable | ~12 |
| `rez-next-release-hook` | Stable | ~6 |
| `rez-next-serialise` | Stable | ~5 |
| `rez-next-explicit` | Stable | ~5 |
| `rez-next-util` | Stable | ~5 |
| `rez-next-python` | Curated compatibility (40 submodules) | ~125 |
| Compat integration tests | Growing coverage | ~210 |

---

## Building from source

```bash
git clone https://github.com/loonghao/rez-next
cd rez-next
cargo build --release
```

### Prerequisites

- Rust 1.95+
- [just](https://github.com/casey/just) (optional, convenience commands)

### Common commands

```bash
vx just build           # Dev build
vx just build-release   # Release build
vx just test            # Run all tests
vx just lint            # Clippy
vx just fmt             # Format
vx just ci              # Full CI check
vx just bench           # Benchmarks
```

---

## Tests

Run the full test suite:

```bash
# For Rust tests only
cargo test --workspace

# For Rust + Python binding tests (requires maturin develop --release first)
maturin develop --release
pytest
```

Test coverage includes:
- Version semantics (rez-compatible: `1.0 > 1.0.0`)
- Package serialization (package.py parsing, YAML/JSON serialization)
- Rex DSL (setenv / prepend_path / alias / info / stop)
- Shell script generation (bash / zsh / fish / PowerShell / cmd)
- Suite management (create / save / load / tool-conflict detection)
- Dependency solving (A* algorithm, backtracking, cycle detection)
- Context serialization (.rxt file read/write)
- `rez.diff` (resolved-context diff)
- `rez.status` (env-var reading, shell detection)
- `rez.search` (exact / contains / regex filtering)
- `rez.depends` (reverse dependencies, transitive deps)
- Custom exception hierarchy (15 typed exceptions)

---

## Benchmark results

Measured with [Criterion.rs](https://github.com/bheisler/criterion.rs) in release mode (opt-level=3, LTO).

### Version operations

| Operation | Time |
|-----------|------|
| Parse single version (`1.2.3-alpha.1`) | ~9.1 us |
| Compare two versions | ~6.8 ns |
| Sort 100 versions | ~19 us |
| Sort 1000 versions | ~176 us |
| Batch parse 1000 versions | ~9.0 ms |

### Package operations

| Operation | Time |
|-----------|------|
| Create empty package | ~35 ns |
| Create package with version | ~8.4 us |
| Serialize to YAML | ~7.0 us |
| Serialize to JSON | ~3.4 us |

<details>
<summary>Reproduce</summary>

```bash
cargo bench --bench version_benchmark
cargo bench --bench simple_package_benchmark
```

</details>

### Python API performance

Measured with `pytest-benchmark` (Python layer over Rust core).

| Operation | Time (avg) | Throughput |
|-----------|-------------|------------|
| `pip_install()` | ~420 ns | 2.38M ops/sec |
| `walk_packages()` | ~42 μs | 23.9K ops/sec |
| `get_pip_dependencies()` | ~293 μs | 3.41K ops/sec |

> Benchmark results from Cycle 188 (pytest-benchmark, Python 3.12).

---

## Documentation

- [Contributing](docs/contributing.md) — development workflow and CI
- [Python Integration](docs/python-integration.md) — Python bindings usage and module coverage
- [Benchmark Guide](docs/benchmark_guide.md) — running and interpreting benchmarks
- [Performance Guide](docs/performance.md) — profiling tools
- [Pre-commit Setup](docs/PRE_COMMIT_SETUP.md) — code quality hooks

### For AI Agents
- [AGENTS.md](AGENTS.md) — progressive disclosure map (start here)
- [llms.txt](llms.txt) — AI-friendly concise usage index
- [llms-full.txt](llms-full.txt) — complete API reference
- [CLAUDE.md](CLAUDE.md) — Claude-specific guidance
- [GEMINI.md](GEMINI.md) — Gemini-specific guidance

---

## License

[Apache License 2.0](LICENSE)

## Acknowledgments

- [Rez](https://github.com/AcademySoftwareFoundation/rez) — the package manager this project implements
- [Rust](https://www.rust-lang.org/) — language and ecosystem
- [PyO3](https://pyo3.rs/) — Rust/Python bindings framework
