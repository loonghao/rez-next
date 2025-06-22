# 🐍 Python Integration for rez-next

> **⚠️ Status: Not Yet Implemented**
>
> This document outlines the planned Python integration for rez-next. The Python bindings are currently under development and will provide seamless integration with existing Rez workflows while delivering the same 117x performance improvements.

## 📋 Overview

The Python integration for rez-next will provide:

- **🔄 100% API Compatibility** - Drop-in replacement for existing Rez Python code
- **⚡ 117x Performance Boost** - Same speed improvements as the Rust implementation
- **🛡️ Memory Safety** - Rust's ownership system prevents crashes and memory leaks
- **🧠 Smart Type Hints** - Full Python typing support for better IDE experience
- **📊 Built-in Profiling** - Performance monitoring and benchmarking tools

## 🚀 Installation (Planned)

```bash
# Install from PyPI (when available)
pip install rez-next-python

# Or install with development dependencies
pip install rez-next-python[dev]

# Verify installation
python -c "import rez_next; print(rez_next.__version__)"
```

## 🎯 Expected API

### Version Management

```python
import rez_next as rez

# 🚀 117x faster version parsing
version = rez.Version("2.1.0-beta.1+build.123")
print(f"Version: {version}")
print(f"Major: {version.major}")
print(f"Minor: {version.minor}")
print(f"Patch: {version.patch}")
print(f"Prerelease: {version.prerelease}")
print(f"Build: {version.build}")

# Version comparison (optimized)
v1 = rez.Version("1.0.0")
v2 = rez.Version("2.0.0")
print(f"{v1} < {v2}: {v1 < v2}")

# Version ranges
range_spec = rez.VersionRange(">=1.0.0,<2.0.0")
print(f"1.5.0 in range: {rez.Version('1.5.0') in range_spec}")
```

### Package Management

```python
# 📦 Package loading and validation
package = rez.Package.load("package.py")
print(f"Package: {package.name} {package.version}")
print(f"Description: {package.description}")
print(f"Authors: {package.authors}")

# Package validation
validator = rez.PackageValidator()
result = validator.validate(package)
if result.is_valid:
    print("✅ Package is valid")
else:
    print("❌ Validation errors:")
    for error in result.errors:
        print(f"  - {error}")

# Package requirements
for req in package.requires:
    print(f"Requires: {req}")
```

### Dependency Resolution

```python
# 🧠 Smart dependency resolution (5x faster)
solver = rez.Solver()

# Configure solver
config = rez.SolverConfig()
config.max_fails = 10
config.timeout = 30.0
solver.set_config(config)

# Resolve packages
try:
    context = solver.resolve([
        "python-3.9",
        "maya-2024",
        "nuke-13.2"
    ])
    
    print(f"✅ Resolved {len(context.resolved_packages)} packages:")
    for pkg in context.resolved_packages:
        print(f"  - {pkg.name} {pkg.version}")
        
except rez.ResolutionError as e:
    print(f"❌ Resolution failed: {e}")
    print("Conflicts:")
    for conflict in e.conflicts:
        print(f"  - {conflict}")
```

### Environment Management

```python
# 🌍 Environment execution (75x faster)
context = rez.ResolvedContext(["python-3.9", "maya-2024"])

# Get environment variables
env_vars = context.get_environ()
print(f"PATH: {env_vars.get('PATH')}")
print(f"PYTHONPATH: {env_vars.get('PYTHONPATH')}")

# Execute commands
proc = context.execute_command([
    "python", "-c", "print('Hello from rez-next!')"
])
exit_code = proc.wait()
print(f"Command exit code: {exit_code}")

# Execute shell commands
result = context.execute_shell("echo $REZ_USED_RESOLVE")
print(f"Shell output: {result.stdout}")
```

### Repository Management

```python
# 📚 Repository scanning and management
repo_manager = rez.RepositoryManager()

# Add repositories
repo_manager.add_repository("/path/to/packages")
repo_manager.add_repository("https://github.com/user/rez-packages")

# Find packages
packages = repo_manager.find_packages("maya")
print(f"Found {len(packages)} maya packages")

# Find with version constraints
packages = repo_manager.find_packages(
    "maya", 
    version_range=">=2020,<2025"
)

# Get latest package
latest = repo_manager.get_latest_package("python")
print(f"Latest Python: {latest.version}")
```

### Intelligent Caching

```python
# ⚡ Intelligent caching with ML-based preheating
cache = rez.IntelligentCacheManager()

# Enable advanced features
cache.enable_predictive_preheating()
cache.enable_adaptive_tuning()
cache.enable_performance_monitoring()

# Cache configuration
config = rez.CacheConfig()
config.max_memory_mb = 512
config.max_disk_gb = 10
config.preheating_threshold = 0.8
cache.configure(config)

# Cache statistics
stats = cache.get_statistics()
print(f"Cache hit rate: {stats.hit_rate:.2%}")
print(f"Memory usage: {stats.memory_usage_mb}MB")
print(f"Disk usage: {stats.disk_usage_gb}GB")
```

### Performance Monitoring

```python
# 📊 Built-in performance monitoring
profiler = rez.PerformanceProfiler()

# Profile a resolution
with profiler.profile("package_resolution"):
    context = solver.resolve(["python-3.9", "maya-2024"])

# Get performance metrics
metrics = profiler.get_metrics()
print(f"Resolution time: {metrics['package_resolution'].duration_ms}ms")
print(f"Memory peak: {metrics['package_resolution'].memory_peak_mb}MB")

# Compare with baseline
baseline = profiler.get_baseline("original_rez")
improvement = metrics['package_resolution'].compare_to(baseline)
print(f"Performance improvement: {improvement.speedup}x faster")
```

## 🔄 Migration Guide

### Drop-in Replacement

The Python bindings are designed to be a complete drop-in replacement for the original Rez Python API:

```python
# Before: Original Rez
from rez import packages_path, resolved_context
from rez.packages import get_latest_package
from rez.solver import Solver

# After: rez-next (same code, 117x faster!)
# Just change the import:
import rez_next as rez
# Or use the compatibility layer:
from rez_next.compat import packages_path, resolved_context
from rez_next.compat.packages import get_latest_package
from rez_next.compat.solver import Solver
```

### Gradual Migration

For large codebases, you can migrate gradually:

```python
# Use environment variable to switch implementations
import os
if os.getenv("USE_REZ_NEXT", "false").lower() == "true":
    import rez_next as rez
else:
    import rez

# Your existing code works with both implementations
solver = rez.Solver()
context = solver.resolve(["python-3.9"])
```

## 🛠️ Development

### Building Python Bindings

```bash
# Install development dependencies
pip install maturin pytest pytest-benchmark

# Build in development mode
maturin develop

# Run tests
pytest tests/

# Run benchmarks
pytest benchmarks/ --benchmark-only
```

### Testing

```python
# Example test structure
import pytest
import rez_next as rez

def test_version_parsing():
    """Test version parsing performance and correctness."""
    version = rez.Version("1.2.3-alpha.1+build.456")
    assert version.major == 1
    assert version.minor == 2
    assert version.patch == 3
    assert version.prerelease == "alpha.1"
    assert version.build == "build.456"

@pytest.mark.benchmark
def test_version_parsing_performance(benchmark):
    """Benchmark version parsing against original Rez."""
    result = benchmark(rez.Version, "1.2.3-alpha.1+build.456")
    assert result.major == 1
```

## 📚 API Reference

The complete API reference will be available at:

- **[Python API Docs](https://docs.rs/rez-next-python)** - Complete Python API reference
- **[Type Stubs](https://github.com/loonghao/rez-next/tree/main/python/rez_next.pyi)** - Type hints for IDEs
- **[Examples](https://github.com/loonghao/rez-next/tree/main/examples/python)** - Usage examples

## 🤝 Contributing

We welcome contributions to the Python integration! Areas where help is needed:

- **PyO3 Bindings** - Implementing the Rust-Python interface
- **API Design** - Ensuring 100% compatibility with original Rez
- **Performance Testing** - Benchmarking and optimization
- **Documentation** - Examples and tutorials
- **Testing** - Comprehensive test coverage

See our [Contributing Guide](../CONTRIBUTING.md) for details on how to get started.

## 📄 License

The Python bindings are licensed under the same Apache License, Version 2.0 as the main project.
