# 🐍 rez-next Python集成

> **⚠️ 状态：尚未实现**
>
> 本文档概述了rez-next的计划Python集成。Python绑定目前正在开发中，将提供与现有Rez工作流程的无缝集成，同时提供相同的117倍性能提升。

## 📋 概述

rez-next的Python集成将提供：

- **🔄 100% API兼容性** - 现有Rez Python代码的直接替换
- **⚡ 117倍性能提升** - 与Rust实现相同的速度改进
- **🛡️ 内存安全** - Rust的所有权系统防止崩溃和内存泄漏
- **🧠 智能类型提示** - 完整的Python类型支持，提供更好的IDE体验
- **📊 内置性能分析** - 性能监控和基准测试工具

## 🚀 安装（计划中）

```bash
# 从PyPI安装（可用时）
pip install rez-next-python

# 或安装开发依赖
pip install rez-next-python[dev]

# 验证安装
python -c "import rez_next; print(rez_next.__version__)"
```

## 🎯 预期API

### 版本管理

```python
import rez_next as rez

# 🚀 117倍更快的版本解析
version = rez.Version("2.1.0-beta.1+build.123")
print(f"版本: {version}")
print(f"主版本: {version.major}")
print(f"次版本: {version.minor}")
print(f"补丁版本: {version.patch}")
print(f"预发布: {version.prerelease}")
print(f"构建: {version.build}")

# 版本比较（优化）
v1 = rez.Version("1.0.0")
v2 = rez.Version("2.0.0")
print(f"{v1} < {v2}: {v1 < v2}")

# 版本范围
range_spec = rez.VersionRange(">=1.0.0,<2.0.0")
print(f"1.5.0在范围内: {rez.Version('1.5.0') in range_spec}")
```

### 包管理

```python
# 📦 包加载和验证
package = rez.Package.load("package.py")
print(f"包: {package.name} {package.version}")
print(f"描述: {package.description}")
print(f"作者: {package.authors}")

# 包验证
validator = rez.PackageValidator()
result = validator.validate(package)
if result.is_valid:
    print("✅ 包有效")
else:
    print("❌ 验证错误:")
    for error in result.errors:
        print(f"  - {error}")

# 包依赖
for req in package.requires:
    print(f"需要: {req}")
```

### 依赖解析

```python
# 🧠 智能依赖解析（5倍更快）
solver = rez.Solver()

# 配置解析器
config = rez.SolverConfig()
config.max_fails = 10
config.timeout = 30.0
solver.set_config(config)

# 解析包
try:
    context = solver.resolve([
        "python-3.9",
        "maya-2024",
        "nuke-13.2"
    ])
    
    print(f"✅ 解析了 {len(context.resolved_packages)} 个包:")
    for pkg in context.resolved_packages:
        print(f"  - {pkg.name} {pkg.version}")
        
except rez.ResolutionError as e:
    print(f"❌ 解析失败: {e}")
    print("冲突:")
    for conflict in e.conflicts:
        print(f"  - {conflict}")
```

### 环境管理

```python
# 🌍 环境执行（75倍更快）
context = rez.ResolvedContext(["python-3.9", "maya-2024"])

# 获取环境变量
env_vars = context.get_environ()
print(f"PATH: {env_vars.get('PATH')}")
print(f"PYTHONPATH: {env_vars.get('PYTHONPATH')}")

# 执行命令
proc = context.execute_command([
    "python", "-c", "print('来自rez-next的问候!')"
])
exit_code = proc.wait()
print(f"命令退出代码: {exit_code}")

# 执行shell命令
result = context.execute_shell("echo $REZ_USED_RESOLVE")
print(f"Shell输出: {result.stdout}")
```

### 仓库管理

```python
# 📚 仓库扫描和管理
repo_manager = rez.RepositoryManager()

# 添加仓库
repo_manager.add_repository("/path/to/packages")
repo_manager.add_repository("https://github.com/user/rez-packages")

# 查找包
packages = repo_manager.find_packages("maya")
print(f"找到 {len(packages)} 个maya包")

# 使用版本约束查找
packages = repo_manager.find_packages(
    "maya", 
    version_range=">=2020,<2025"
)

# 获取最新包
latest = repo_manager.get_latest_package("python")
print(f"最新Python: {latest.version}")
```

### 智能缓存

```python
# ⚡ 基于ML的智能缓存预热
cache = rez.IntelligentCacheManager()

# 启用高级功能
cache.enable_predictive_preheating()
cache.enable_adaptive_tuning()
cache.enable_performance_monitoring()

# 缓存配置
config = rez.CacheConfig()
config.max_memory_mb = 512
config.max_disk_gb = 10
config.preheating_threshold = 0.8
cache.configure(config)

# 缓存统计
stats = cache.get_statistics()
print(f"缓存命中率: {stats.hit_rate:.2%}")
print(f"内存使用: {stats.memory_usage_mb}MB")
print(f"磁盘使用: {stats.disk_usage_gb}GB")
```

### 性能监控

```python
# 📊 内置性能监控
profiler = rez.PerformanceProfiler()

# 分析解析过程
with profiler.profile("package_resolution"):
    context = solver.resolve(["python-3.9", "maya-2024"])

# 获取性能指标
metrics = profiler.get_metrics()
print(f"解析时间: {metrics['package_resolution'].duration_ms}ms")
print(f"内存峰值: {metrics['package_resolution'].memory_peak_mb}MB")

# 与基准比较
baseline = profiler.get_baseline("original_rez")
improvement = metrics['package_resolution'].compare_to(baseline)
print(f"性能提升: {improvement.speedup}倍更快")
```

## 🔄 迁移指南

### 直接替换

Python绑定设计为原始Rez Python API的完全直接替换：

```python
# 之前：原始Rez
from rez import packages_path, resolved_context
from rez.packages import get_latest_package
from rez.solver import Solver

# 之后：rez-next（相同代码，117倍更快！）
# 只需更改导入：
import rez_next as rez
# 或使用兼容层：
from rez_next.compat import packages_path, resolved_context
from rez_next.compat.packages import get_latest_package
from rez_next.compat.solver import Solver
```

### 渐进式迁移

对于大型代码库，您可以逐步迁移：

```python
# 使用环境变量切换实现
import os
if os.getenv("USE_REZ_NEXT", "false").lower() == "true":
    import rez_next as rez
else:
    import rez

# 您的现有代码适用于两种实现
solver = rez.Solver()
context = solver.resolve(["python-3.9"])
```

## 🛠️ 开发

### 构建Python绑定

```bash
# 安装开发依赖
pip install maturin pytest pytest-benchmark

# 开发模式构建
maturin develop

# 运行测试
pytest tests/

# 运行基准测试
pytest benchmarks/ --benchmark-only
```

### 测试

```python
# 示例测试结构
import pytest
import rez_next as rez

def test_version_parsing():
    """测试版本解析性能和正确性。"""
    version = rez.Version("1.2.3-alpha.1+build.456")
    assert version.major == 1
    assert version.minor == 2
    assert version.patch == 3
    assert version.prerelease == "alpha.1"
    assert version.build == "build.456"

@pytest.mark.benchmark
def test_version_parsing_performance(benchmark):
    """对比原始Rez的版本解析基准测试。"""
    result = benchmark(rez.Version, "1.2.3-alpha.1+build.456")
    assert result.major == 1
```

## 📚 API参考

完整的API参考将在以下位置提供：

- **[Python API文档](https://docs.rs/rez-next-python)** - 完整Python API参考
- **[类型存根](https://github.com/loonghao/rez-next/tree/main/python/rez_next.pyi)** - IDE类型提示
- **[示例](https://github.com/loonghao/rez-next/tree/main/examples/python)** - 使用示例

## 🤝 贡献

我们欢迎对Python集成的贡献！需要帮助的领域：

- **PyO3绑定** - 实现Rust-Python接口
- **API设计** - 确保与原始Rez 100%兼容
- **性能测试** - 基准测试和优化
- **文档** - 示例和教程
- **测试** - 全面的测试覆盖

有关如何开始的详细信息，请参阅我们的[贡献指南](../CONTRIBUTING.md)。

## 📄 许可证

Python绑定与主项目采用相同的Apache License 2.0许可。
