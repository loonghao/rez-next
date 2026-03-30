# Python Integration for rez-next

**Status: Not implemented.**

This document describes planned Python bindings. The `rez-next-python` crate exists in the workspace but has no functional bindings.

## Current state

- `rez-next-python` depends on all other crates
- Uses PyO3 0.25 with `abi3-py38`
- Has scaffolding code but nothing is exposed to Python yet
- Not published to PyPI

## Planned API (aspirational)

```python
import rez_next as rez

# Version
version = rez.Version("2.1.0")
print(version.major, version.minor, version.patch)

# Package
package = rez.Package.load("package.py")
print(package.name, package.version)

# Solver
solver = rez.Solver()
context = solver.resolve(["python-3.9", "maya-2024"])
```

None of the above works today.

## Building (when ready)

```bash
pip install maturin
maturin develop
```

## License

[Apache License 2.0](../LICENSE)
