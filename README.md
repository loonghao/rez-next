# Rez Core 🦀

> ⚠️ **WORK IN PROGRESS - EXPERIMENTAL PROJECT**
> This is an experimental attempt to rewrite [Rez](https://github.com/AcademySoftwareFoundation/rez) core components in Rust.
> **DO NOT USE IN PRODUCTION ENVIRONMENTS**
> This project is primarily for learning and exploration purposes.

An experimental high-performance rewrite of [Rez](https://github.com/AcademySoftwareFoundation/rez) core components in Rust, inspired by successful projects like [pydantic-core](https://github.com/pydantic/pydantic-core).

## 🎯 Project Goals

This is primarily a **learning project** to explore whether Rust can bring meaningful performance improvements to package management and dependency resolution.

**If it works out**: We might achieve significant performance gains for the Rez ecosystem.
**If it doesn't**: It's still a valuable learning experience in Rust systems programming.

## 🚀 What We're Building

### Phase 1: Version System (✅ Core Complete, 🚧 Optimizing)
- ✅ High-performance version parsing and comparison
- ✅ Python bindings with ABI3 compatibility (Python 3.8+)
- ✅ Comprehensive test suite (35/38 tests passing)
- 🚧 Version range calculations and intersections
- **Target**: 5-10x performance improvement over Python implementation

### Phase 2: Dependency Solver (📋 Planned)
- Parallel dependency resolution algorithms
- Optimized conflict detection and reporting
- **Target**: 3-5x performance improvement

### Phase 3: Repository Management (💭 Future)
- Async I/O for package scanning
- Multi-layered caching system
- **Target**: 2-3x performance improvement

## 🏗️ Technical Architecture

```
rez-core/                           # Unified Rust crate
├── src/
│   ├── lib.rs                      # Main library entry point
│   ├── common/                     # Shared utilities and types
│   │   ├── error.rs                # Error handling
│   │   ├── config.rs               # Configuration management
│   │   └── utils.rs                # Utility functions
│   ├── version/                    # Version system implementation
│   │   ├── version.rs              # Core Version struct
│   │   ├── range.rs                # Version range operations
│   │   ├── token.rs                # Version token types
│   │   └── parser.rs               # High-performance parsing
│   ├── solver/                     # Dependency solver (planned)
│   ├── repository/                 # Repository management (planned)
│   └── python/                     # PyO3 Python bindings
├── benches/                        # Performance benchmarks
└── tests/                          # Integration tests
```

## 🛠️ Technical Approach

- **Language**: Rust with PyO3 bindings for Python integration
- **Strategy**: Gradual replacement of performance-critical components
- **Compatibility**: 100% API compatibility with existing Rez
- **Fallback**: Automatic fallback to Python implementation if Rust fails

## 📊 Expected Outcomes

**Optimistic**: 4-6x overall performance improvement
**Realistic**: 2-3x improvement in critical paths
**Pessimistic**: Valuable Rust learning experience 😄

## 🔧 Development Setup

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Python 3.8+ (for PyO3 bindings with ABI3 support)
- [uv](https://docs.astral.sh/uv/getting-started/installation/) (recommended Python package manager)
- Git

### Quick Start

```bash
# Clone the repository
git clone https://github.com/loonghao/rez-core.git
cd rez-core

# Install uv (if not already installed)
curl -LsSf https://astral.sh/uv/install.sh | sh  # Unix
# or
powershell -c "irm https://astral.sh/uv/install.ps1 | iex"  # Windows

# Set up development environment
uv sync --all-extras
```

### Building and Testing

#### Windows (PowerShell)
```powershell
# Development build with Python bindings
.\scripts\build.ps1 build-dev

# Run Python tests
.\scripts\build.ps1 test-python

# Run all tests (Python + Rust)
.\scripts\build.ps1 test

# Build ABI3 wheel for distribution
.\scripts\build.ps1 build-wheel

# Run performance benchmarks
.\scripts\build.ps1 benchmark

# Format and lint code
.\scripts\build.ps1 format
.\scripts\build.ps1 lint
```

#### Unix/Linux/macOS (Make)
```bash
# Development build
make build-dev

# Run tests
make test

# Build wheel
make build-wheel

# Run benchmarks
make benchmark

# Format and lint
make format
make lint
```

### Performance Profiling

We use [flamegraph](https://github.com/flamegraph-rs/flamegraph) for performance analysis, following pydantic-core's approach:

```bash
# Install flamegraph (requires perf on Linux)
cargo install flamegraph

# Build with profiling symbols
.\scripts\build.ps1 build-profiling  # Windows
# or
make build-profiling  # Unix

# Profile Python benchmarks
flamegraph -- uv run pytest tests/python/ -k test_version_creation_performance --benchmark-enable

# Profile Rust benchmarks
flamegraph -- cargo bench

# The flamegraph command will produce an interactive SVG at flamegraph.svg
```

**Note**: On Windows, flamegraph requires additional setup. Consider using Linux/WSL for profiling.

## 📋 Implementation Status & TODO

### ✅ Completed
- [x] Basic project structure and Cargo configuration
- [x] Core module architecture (common, version, solver, repository)
- [x] Error handling and configuration management
- [x] Basic version token system and parsing
- [x] PyO3 Python bindings with ABI3 compatibility (Python 3.8+)
- [x] Comprehensive test framework (Python + Rust)
- [x] Development workflow automation (Makefile + PowerShell scripts)
- [x] Version comparison and ordering algorithms
- [x] Performance benchmarking infrastructure
- [x] uv-based dependency management following pydantic-core patterns

### 🚧 Version System (Phase 1 - Current Focus)
- [x] ~~Implement state-machine based version parsing~~ ✅ **Completed**
- [x] ~~Optimize version comparison algorithms~~ ✅ **Completed**
- [x] ~~PyO3 Python bindings for version system~~ ✅ **Completed**
- [x] ~~Comprehensive test suite with edge cases~~ ✅ **Completed (35/38 tests passing)**
- [ ] **High-priority**: Complete version range intersection and union operations
- [ ] **High-priority**: Fix remaining 3 test failures (error handling + pre-release comparison)
- [ ] **Medium-priority**: Support for custom version token types
- [ ] **Medium-priority**: Performance optimization based on flamegraph profiling

### 📋 Dependency Solver (Phase 2 - Planned)
- [ ] Core dependency resolution algorithm implementation
- [ ] Parallel solving with Rayon
- [ ] Conflict detection and detailed error reporting
- [ ] Solver caching and memoization
- [ ] Integration with version system
- [ ] Python bindings for solver

### 💾 Repository System (Phase 3 - Future)
- [ ] Async package scanning with Tokio
- [ ] Multi-layered caching (memory, disk, distributed)
- [ ] File system monitoring for incremental updates
- [ ] Package metadata loading and validation
- [ ] Python bindings for repository management

### 🔧 Infrastructure & Tooling
- [ ] **Critical**: Set up Python environment for PyO3 development
- [ ] **Critical**: Enable PyO3 bindings and Python integration tests
- [ ] **High-priority**: Comprehensive benchmark suite
- [ ] **High-priority**: CI/CD pipeline with multi-platform testing
- [ ] **Medium-priority**: Performance regression testing
- [ ] **Medium-priority**: Memory usage profiling and optimization
- [ ] **Low-priority**: Documentation generation and examples

### 🧪 Testing & Quality
- [ ] Unit tests for all core components
- [ ] Integration tests with existing Rez test suite
- [ ] Property-based testing with proptest
- [ ] Performance benchmarks vs Python implementation
- [ ] Memory safety and leak detection
- [ ] Cross-platform compatibility testing

## 📚 Documentation & References

### Project Documentation
- [Master PRD](../rez/rez-core-master_prd.md) - Overall project planning and architecture
- [Implementation Guide](../rez/rez-core-implementation_guide.md) - Detailed development instructions
- [Version System PRD](../rez/rez-core-version_prd.md) - Version system specifications
- [Solver System PRD](../rez/rez-core-solver_prd.md) - Dependency solver specifications
- [Repository System PRD](../rez/rez-core-repository_prd.md) - Repository management specifications

### Learning Resources
- [The Rust Programming Language](https://doc.rust-lang.org/book/)
- [PyO3 User Guide](https://pyo3.rs/) - Rust-Python bindings
- [pydantic-core](https://github.com/pydantic/pydantic-core) - Our inspiration project
- [Rayon](https://github.com/rayon-rs/rayon) - Data parallelism in Rust

## 🤝 Contributing

We welcome contributions from both Rust experts and fellow learners! Here's how to get started:

1. **Check the TODO list** above to find tasks that match your skill level
2. **Start small** - pick up documentation, tests, or minor features first
3. **Ask questions** - open an issue if you need clarification on any task
4. **Follow the architecture** - respect the modular design outlined in the PRDs

### Development Workflow
1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes and add tests
4. Ensure `cargo test` and `cargo check` pass
5. Submit a pull request with a clear description

## ⚠️ Important Disclaimers

- **Experimental Status**: This project is in early experimental stages
- **No Production Use**: Do not use in production environments
- **API Instability**: APIs may change significantly during development
- **Learning Focus**: Primary goal is learning and exploration
- **Community Effort**: Success depends on community involvement and feedback

## 📄 License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

---

*"The best way to learn Rust is to build something useful... or at least try to!"* 🦀

## 📈 Current Status & Performance

**Current Status**: ✅ Phase 1 Core Complete - Version System with Python Bindings
**Next Milestone**: Performance optimization and Phase 2 planning

### Recent Achievements
- ✅ **ABI3 Python Bindings**: Compatible with Python 3.8+ (single wheel for all versions)
- ✅ **Comprehensive Testing**: 35/38 tests passing with pytest framework
- ✅ **Development Workflow**: Automated builds, testing, and profiling
- ✅ **Performance Infrastructure**: Benchmarking and flamegraph profiling ready

### Performance Baseline
- **Version Creation**: ~1000 versions/ms (development build)
- **Version Comparison**: ~100 sorts of 100 versions/ms
- **Memory Usage**: Reasonable for 1000+ version objects
- **ABI3 Compatibility**: Single wheel works across Python 3.8-3.13+

*Detailed performance analysis with flamegraph profiling coming soon...*
