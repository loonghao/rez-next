# rez-next - AI Agent Guide

> **Progressive disclosure map** — start here, then follow links to details.
>
> **Disclosure chain**: this file → [llms.txt](./llms.txt) (concise AI index) → [llms-full.txt](./llms-full.txt) (full API reference) → [docs/](./docs/) (human guides)

## Project Overview

rez-next is a **high-performance Rust rewrite** of the [Rez](https://github.com/AcademySoftwareFoundation/rez) package manager with Python bindings. It provides drop-in compatibility for most common Rez workflows while delivering significantly better performance.

**Key facts for agents:**
- Language: Rust 2024 edition (MSRV 1.95) + Python 3.8+ bindings (PyO3 abi3)
- Build system: Cargo workspace (20 crates) + Maturin for Python
- Current version: 0.3.4 (see [CHANGELOG.md](./CHANGELOG.md))
- License: Apache 2.0
- Python modules: **41 modules** (43 .py files) + **35 native** PyO3 submodules covering most Rez APIs
- Native: **35 registered** PyO3 submodules, 28 native classes, ~50 top-level functions
- CLI: **28 subcommands** (config, context, view, env, release, test, build, search, bind, depends, solve, cp, mv, rm, status, diff, pkg-help, plugins, pkg-cache, suites, bundle, pip, complete, forward, gui, parse-version, self-test, self-update)
- Tests: **82 test files** (27 Python + 46 Rust + 9 fixtures)
- Status: Many workflows work with `import rez_next as rez`, but **not yet a seamless drop-in** for every API surface. The [`auto-improve`](https://github.com/loonghao/rez-next/tree/auto-improve) branch has 25+ additional modules in active development (62 total submodules, 56 Python tests).

## Quick Start

```python
# Drop-in replacement for most use cases
import rez_next as rez
from rez_next.packages_ import iter_packages, get_latest_package
from rez_next.resolved_context import ResolvedContext

# API is fully compatible
ctx = rez.resolve_packages(["python-3.9", "maya-2024"])
pkg = rez.get_latest_package("python")
```

## Documentation Map

### For AI Agents (this file is the primary entry point)
- **CLAUDE.md** → Anthropic Claude-specific guidance (references this file)
- **GEMINI.md** → Google Gemini-specific guidance (references this file)
- **llms.txt** → AI-friendly concise usage index (Python submodules, crate map, quick commands)
- **llms-full.txt** → Complete API reference (Rust + Python + CLI + benchmarks)

### For Humans
- **README.md** → Project overview, installation, quick start
- **README_zh.md** → Chinese version
- **docs/** → Detailed guides:
  - `contributing.md` → Development workflow, CI
  - `benchmark_guide.md` → Running benchmarks
  - `performance.md` → Profiling tools
  - `python-integration.md` → Python bindings usage
  - `python-integration_zh.md` → Chinese version
  - `PRE_COMMIT_SETUP.md` → Code quality hooks

### Reference
- **CHANGELOG.md** → Release history
- **SECURITY.md** → Security policy

## Architecture (Simplified)

```
rez-next/                          # Monorepo root
├── crates/                        # Rust crates (20 total)
│   ├── rez-next-common/           # Shared types, errors, config
│   ├── rez-next-config/           # Config loading & validation
│   ├── rez-next-version/          # Version parsing (state machine)
│   ├── rez-next-package/          # Package definition, package.py parser
│   ├── rez-next-package-cache/    # Package payload caching
│   ├── rez-next-package-filter/   # Package filter (glob, regex, range rules)
│   ├── rez-next-solver/           # Dependency solver (A* + backtracking)
│   ├── rez-next-repository/       # Repository scanning, caching
│   ├── rez-next-context/         # Resolved contexts, Rex integration
│   ├── rez-next-build/           # Build system integration
│   ├── rez-next-cache/           # Multi-level caching
│   ├── rez-next-rex/             # Rex DSL interpreter
│   ├── rez-next-suites/          # Suite management
│   ├── rez-next-bind/            # Bind system tools
│   ├── rez-next-search/          # Package search
│   ├── rez-next-explicit/        # Explicit package lists
│   ├── rez-next-serialise/       # Serialization support
│   ├── rez-next-release-hook/    # Release hooks
│   ├── rez-next-util/            # Utility functions
│   └── rez-next-python/          # Python bindings (PyO3)
├── src/                           # Rust CLI binary (28 subcommands, entry: src/bin/rez-next.rs)
├── tests/                         # Integration tests
├── benches/                       # Criterion benchmarks
├── docs/                          # Documentation (see above)
└── metrics/                       # Performance tracking
```

## Key Concepts for Agents

### 1. Drop-in Replacement Strategy
- Users can `import rez_next as rez` for most workflows
- Python module structure mirrors Rez: `rez_next.packages_`, `rez_next.version`, etc.
- Not all Rez APIs are implemented yet (check `python-integration.md` for coverage)

### 2. Development Workflow
```bash
# Build
vx just build

# Test
vx just test

# Lint
vx just lint

# Full CI check
vx just ci

# Benchmarks
vx just bench
```

### 3. Python Integration Testing
```bash
# Build wheel + run tests (recommended)
vx just py-build
vx just py-test

# Full Python CI
vx just py-ci
```

### 4. Release Process
- Automated via [release-please](https://github.com/googleapis/release-please)
- Multi-platform builds (Linux, macOS, Windows)
- Publishes to crates.io and PyPI

## Common Agent Tasks

### Task 1: Understanding Module Coverage
→ Read `docs/python-integration.md` for implemented Python submodules

### Task 2: Adding a New Feature
1. Implement in appropriate `crates/` module
2. Expose via `rez-next-python` if needed
3. Add tests (Rust: `#[test]`, Python: `pytest`)
4. Update documentation
5. Run `vx just ci` before submitting PR

### Task 3: Debugging Solver Issues
→ Check `crates/rez-next-solver/` (A* algorithm implementation)
→ Enable debug logging: `RUST_LOG=debug`

### Task 4: Performance Optimization
→ Run benchmarks: `vx just bench`
→ Profile with `docs/performance.md` guide
→ Check `crates/rez-next-cache/` for caching opportunities

## API Quick Reference

### Version Operations
```python
import rez_next as rez
v1 = rez.PyVersion("1.2.3")
v2 = rez.PyVersionRange(">=3.9,<4.0")
```

### Package Queries
```python
from rez_next.packages_ import get_latest_package, iter_packages
pkg = get_latest_package("python")
for p in iter_packages("maya"):
    print(p.version)
```

### Dependency Resolution
```python
from rez_next.resolved_context import ResolvedContext
ctx = ResolvedContext.resolve_packages(["python-3.9", "maya-2024"])
```

### Context Diff (rez.diff)
```python
from rez_next.diff import diff_contexts
result = diff_contexts(["python-3.9"], ["python-3.11", "maya-2024"])
```

## Configuration Files for Agents

- `vx.toml` → Tool version management (use `vx` command)
- `justfile` → Common development tasks
- `.github/workflows/ci.yml` → CI pipeline definition
- `release-please-config.json` → Release automation
- `renovate.json` → Dependency updates
- `deny.toml` → Supply chain security (cargo-deny)

## Important Notes

⚠️ **Not all Rez features are implemented** — check coverage before suggesting code changes

⚠️ **Python bindings are partial** — `rez_next` has 41 Python modules + 35 native submodules, covering most Rez APIs but not every edge case

⚠️ **Breaking changes possible** — pre-1.0 project, API may change

## Getting Help

- **Issues**: https://github.com/loonghao/rez-next/issues
- **Discussions**: https://github.com/loonghao/rez-next/discussions
- **CI Status**: Check PR for green checkmarks before merging

---

**For AI agents:** This file is your progressive disclosure map. For concise API reference, see [llms.txt](./llms.txt). For complete API details, see [llms-full.txt](./llms-full.txt). For human-oriented guides, see [docs/](./docs/). When in doubt, check `docs/python-integration.md` for feature coverage before suggesting implementations.
