# Python Integration for rez-next

## Status

Python bindings are **implemented** in `crates/rez-next-python` and exposed through the `rez_next` Python package.

## Current state

- Native bindings are built with PyO3 and `abi3-py38`
- The extension module is exposed as `rez_next._native`
- Python shim modules mirror the Rez-style import surface, including modules such as `rez_next.version`, `rez_next.packages_`, and `rez_next.resolved_context`
- The broader module matrix and compatibility notes are maintained in the root `README.md`

## Local development

```bash
just py-build
just py-test
```

For direct development commands, use:

```bash
cd crates/rez-next-python
vx maturin develop --features pyo3/extension-module
vx pytest tests/ -v --tb=short
```

## Example

```python
import rez_next as rez
from rez_next.packages_ import get_latest_package

pkg = get_latest_package("python")
ctx = rez.resolve_packages(["python-3.9"])
print(pkg.name, ctx.status)
```


## License

[Apache License 2.0](../LICENSE)
