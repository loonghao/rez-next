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

### Phase 1: Version System (🚧 In Progress)
- High-performance version parsing and comparison
- Version range calculations and intersections
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
- Python 3.7+ (for PyO3 bindings)
- Git

### Quick Start

```bash
# Clone the repository
git clone https://github.com/loonghao/rez-core.git
cd rez-core

# Check that everything compiles
cargo check

# Run tests
cargo test

# Run benchmarks
cargo bench
```

### Building with Python Support

```bash
# Install maturin for Python extension building
pip install maturin

# Build in development mode
maturin develop

# Build release wheels
maturin build --release
```

## 📋 Implementation Status & TODO

### ✅ Completed
- [x] Basic project structure and Cargo configuration
- [x] Core module architecture (common, version, solver, repository)
- [x] Error handling and configuration management
- [x] Basic version token system
- [x] Placeholder implementations for all major components

### 🚧 Version System (Phase 1 - Current Focus)
- [ ] **High-priority**: Implement state-machine based version parsing
- [ ] **High-priority**: Optimize version comparison algorithms
- [ ] **Medium-priority**: Version range intersection and union operations
- [ ] **Medium-priority**: Support for custom version token types
- [ ] **Low-priority**: PyO3 Python bindings for version system
- [ ] **Low-priority**: Comprehensive test suite with edge cases

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

**Current Status**: 🚧 Work in Progress - Version System Implementation
**Next Milestone**: Complete basic version parsing and comparison (Phase 1)
