# rez-next Python 集成

**状态：未实现。**

本文档描述计划中的 Python 绑定。`rez-next-python` crate 存在于 workspace 中，但没有可用的绑定。

## 当前状态

- `rez-next-python` 依赖所有其他 crate
- 使用 PyO3 0.25，`abi3-py38`
- 有脚手架代码，但没有任何功能暴露给 Python
- 未发布到 PyPI

## 计划中的 API（愿景）

```python
import rez_next as rez

# 版本
version = rez.Version("2.1.0")
print(version.major, version.minor, version.patch)

# 包
package = rez.Package.load("package.py")
print(package.name, package.version)

# 求解器
solver = rez.Solver()
context = solver.resolve(["python-3.9", "maya-2024"])
```

以上代码目前均不可用。

## 构建（就绪后）

```bash
pip install maturin
maturin develop
```

## 许可证

[Apache License 2.0](../LICENSE)
