# rez-next Python 集成

## 状态

Python 绑定**已经实现**，位于 `crates/rez-next-python`，并通过 `rez_next` Python 包对外暴露。

## 当前状态

- 原生绑定基于 PyO3 构建，并使用 `abi3-py38`
- 扩展模块暴露为 `rez_next._native`
- Python shim 模块保持了 Rez 风格的导入接口，例如 `rez_next.version`、`rez_next.packages_`、`rez_next.resolved_context`
- 更完整的模块矩阵与兼容性说明以根目录 `README.md` 为准

## 本地开发

```bash
just py-build
just py-test
```

如果需要直接执行底层命令，可使用：

```bash
cd crates/rez-next-python
vx maturin develop --features pyo3/extension-module
vx pytest tests/ -v --tb=short
```

## 示例

```python
import rez_next as rez
from rez_next.packages_ import get_latest_package

pkg = get_latest_package("python")
ctx = rez.resolve_packages(["python-3.9"])
print(pkg.name, ctx.status)
```


## 许可证

[Apache License 2.0](../LICENSE)
