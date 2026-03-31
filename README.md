# rez-next

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![CI](https://img.shields.io/github/actions/workflow/status/loonghao/rez-next/ci.yml?branch=main)](https://github.com/loonghao/rez-next/actions)

A complete Rust rewrite of the [Rez](https://github.com/AcademySoftwareFoundation/rez) package manager with full Python bindings — just replace `import rez` with `import rez_next` for a seamless drop-in switch.

[English](README.md) | [中文](README_zh.md)

---

## Quick Start

```python
# Before
import rez
from rez.packages_ import iter_packages, get_latest_package
from rez.resolved_context import ResolvedContext

# After (drop-in replacement)
import rez_next as rez
from rez_next.packages_ import iter_packages, get_latest_package
from rez_next.resolved_context import ResolvedContext

# API is fully compatible
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
| `rez_next.resolved_context` | `rez.resolved_context` | Dependency resolution, context management |
| `rez_next.suite` | `rez.suite` | Suite creation and tool-chain management |
| `rez_next.config` | `rez.config` | Configuration reading |
| `rez_next.system` | `rez.system` | System info (platform, Python version, etc.) |
| `rez_next.shell` | `rez.shells` | Shell script generation (bash/zsh/fish/PowerShell/cmd) |
| `rez_next.rex` | `rez.rex` | Rex command-language interpreter |
| `rez_next.build_` | `rez.build_` | Package build system integration |
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
| `rez_next.data` | `rez.data` | Built-in data resources (completion scripts, examples) |
| `rez_next.cli` | `rez.cli` | CLI entry-points (programmatic invocation) |
| `rez_next.exceptions` | `rez.exceptions` | Exception hierarchy |
| `rez_next.utils.resources` | `rez.utils.resources` | Resource loading utilities |

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

A Cargo workspace with 14 crates + Python bindings:

```
rez-next-common       Shared error types, config, utilities
rez-next-version      Version parsing, comparison, ranges (state-machine parser)
rez-next-package      Package definition, package.py parsing (RustPython AST)
rez-next-solver       Dependency resolution (A* algorithm + backtracking + cycle detection)
rez-next-repository   Repository scanning and caching
rez-next-context      Resolved context, Rex integration, serialization
rez-next-build        Build system integration (cmake/make/python/cargo/nodejs)
rez-next-cache        Multi-level caching (memory + disk)
rez-next-rex          Rex command language (full DSL + 5 shell activation scripts)
rez-next-suites       Suite management (collections of resolved contexts)
rez-next-bind         Bind system tools (python/cmake/pip/git, etc.)
rez-next-search       Package search (exact / contains / regex FilterMode)
rez-next-python       Python bindings via PyO3 (18 submodules)
```

### Component status

| Crate | Status | Tests |
|-------|--------|-------|
| `rez-next-version` | Complete | ~30 |
| `rez-next-package` | Complete | ~25 |
| `rez-next-common` | Complete | ~10 |
| `rez-next-rex` | Complete | ~20 |
| `rez-next-solver` | Complete (A* enabled) | ~15 |
| `rez-next-context` | Complete | ~12 |
| `rez-next-repository` | Complete | ~8 |
| `rez-next-build` | Complete | ~6 |
| `rez-next-cache` | Complete | ~5 |
| `rez-next-suites` | Complete | ~10 |
| `rez-next-bind` | Complete | ~37 |
| `rez-next-search` | Complete | ~16 |
| `rez-next-python` | Complete (18 submodules) | ~125 |
| Compat integration tests | — | ~210 |

---

## Building from source

```bash
git clone https://github.com/loonghao/rez-next
cd rez-next
cargo build --release
```

### Prerequisites

- Rust 1.70+
- [just](https://github.com/casey/just) (optional, convenience commands)

### Common commands

```bash
just build           # Dev build
just build-release   # Release build
just test            # Run all tests
just lint            # Clippy
just fmt             # Format
just ci              # Full CI check
just bench           # Benchmarks
```

---

## Tests

Run the full test suite:

```bash
cargo test --workspace
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

---

## Documentation

- [Contributing](docs/contributing.md) — development workflow and CI
- [Benchmark Guide](docs/benchmark_guide.md) — running and interpreting benchmarks
- [Performance Guide](docs/performance.md) — profiling tools
- [Python Integration](docs/python-integration.md) — Python bindings usage
- [Pre-commit Setup](docs/PRE_COMMIT_SETUP.md) — code quality hooks

---

## License

[Apache License 2.0](LICENSE)

## Acknowledgments

- [Rez](https://github.com/AcademySoftwareFoundation/rez) — the package manager this project implements
- [Rust](https://www.rust-lang.org/) — language and ecosystem
- [PyO3](https://pyo3.rs/) — Rust/Python bindings framework
