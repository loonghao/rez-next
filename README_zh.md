# rez-next

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![CI](https://img.shields.io/github/actions/workflow/status/loonghao/rez-next/ci.yml?branch=main)](https://github.com/loonghao/rez-next/actions)

用 Rust 重写的 [Rez](https://github.com/AcademySoftwareFoundation/rez) 包管理器，提供完整的 Python 绑定，只需将 `import rez` 替换为 `import rez_next` 即可无缝切换。

[English](README.md) | [中文](README_zh.md)

---

## 快速开始

```python
# 之前
import rez
from rez.packages_ import iter_packages, get_latest_package
from rez.resolved_context import ResolvedContext

# 之后（无缝替换）
import rez_next as rez
from rez_next.packages_ import iter_packages, get_latest_package
from rez_next.resolved_context import ResolvedContext

# API 完全一致
ctx = rez.resolve_packages(["python-3.9", "maya-2024"])
pkg = rez.get_latest_package("python")
for p in rez.iter_packages("maya"):
    print(p.name, p.version)
```

---

## 功能概览

### 已实现的 Python 子模块

| 子模块 | 等价 rez 模块 | 功能 |
|--------|--------------|------|
| `rez_next.version` | `rez.vendor.version` | 版本解析、比较、范围 |
| `rez_next.packages_` | `rez.packages_` | 包迭代、查询、复制/移动/删除 |
| `rez_next.resolved_context` | `rez.resolved_context` | 依赖解析、上下文管理 |
| `rez_next.suite` | `rez.suite` | Suite 创建、工具链管理 |
| `rez_next.config` | `rez.config` | 配置读取 |
| `rez_next.system` | `rez.system` | 系统信息（平台、Python版本等） |
| `rez_next.shell` | `rez.shells` | Shell 脚本生成（bash/zsh/fish/PowerShell/cmd）|
| `rez_next.rex` | `rez.rex` | Rex 命令语言解释器 |
| `rez_next.build_` | `rez.build_` | 包构建系统集成 |
| `rez_next.release` | `rez.release` | 包发布流程 |
| `rez_next.bind` | `rez.bind` | 系统工具绑定为 rez 包 |
| `rez_next.pip` | `rez.pip` | pip 包转换为 rez 包 |
| `rez_next.plugins` | `rez.plugins` | 插件管理 |
| `rez_next.env` | `rez.env` | 环境创建与激活 |
| `rez_next.source` | `rez.source` | 上下文激活脚本生成 |
| `rez_next.bundles` | `rez.bundles` | 上下文打包（离线使用）|
| `rez_next.forward` | `rez.forward` | Shell 前向兼容脚本 |
| `rez_next.search` | `rez.cli.search` | 包搜索（精确/包含/正则）|
| `rez_next.complete` | `rez.cli.complete` | Shell tab 补全脚本生成 |
| `rez_next.diff` | `rez.cli.diff` | 两个已解析上下文的差异比对 |
| `rez_next.status` | `rez.cli.status` | 当前激活上下文状态查询 |
| `rez_next.depends` | `rez.cli.depends` | 反向依赖查询 |
| `rez_next.data` | `rez.data` | 内置数据资源（补全脚本、示例）|
| `rez_next.cli` | `rez.cli` | CLI 入口（程序化调用）|
| `rez_next.exceptions` | `rez.exceptions` | 异常类 |
| `rez_next.utils.resources` | `rez.utils.resources` | 资源加载工具 |

### API 示例

#### 版本操作

```python
import rez_next as rez

# 版本解析与比较
v1 = rez.PyVersion("1.2.3")
v2 = rez.PyVersion("2.0.0")
print(v1 < v2)  # True

# 版本范围
r = rez.PyVersionRange(">=3.9,<4.0")
print(r.contains(v1))  # False
```

#### 包查询

```python
import rez_next as rez

# 获取最新版本
pkg = rez.get_latest_package("python")
print(pkg.name, pkg.version)

# 迭代所有版本
for p in rez.iter_packages("maya", range_=">=2023"):
    print(p.version)
```

#### 依赖解析

```python
import rez_next as rez

ctx = rez.resolve_packages(["python-3.9", "maya-2024", "numpy-1.24"])
print(ctx.status)          # "solved"
print(ctx.resolved_packages)
```

#### 上下文差异比对（rez.diff）

```python
from rez_next.diff import diff_contexts, format_diff

result = diff_contexts(
    ["python-3.9", "maya-2023"],
    ["python-3.11", "maya-2024", "houdini-20"]
)
print(f"新增: {result.num_added}, 升级: {result.num_upgraded}")
print(format_diff(result))
# 输出：
#   + houdini 20
#   ^ python 3.9 -> 3.11
#   ^ maya 2023 -> 2024
```

#### 反向依赖查询（rez.depends）

```python
from rez_next.depends import get_reverse_dependencies, print_depends

# 查找所有依赖 python 的包
result = get_reverse_dependencies("python", transitive=True)
print(result.format())
# 输出：
#   Reverse dependencies for 'python':
#     Direct:
#       maya-2024.1  (requires 'python-3.9')
#       houdini-20.0  (requires 'python-3.10')
#     Transitive:
#       nuke-14.0  (requires 'maya-2024')
```

#### 当前上下文状态（rez.status）

```python
from rez_next.status import get_current_status, is_in_rez_context

if is_in_rez_context():
    status = get_current_status()
    print(f"Active packages: {status.resolved_packages}")
    print(f"Shell: {status.current_shell}")
```

#### 包搜索（rez.search）

```python
from rez_next.search import search_packages, search_package_names

# 搜索所有 maya 相关包
results = search_packages("maya")
for r in results:
    print(r.name, r.version)

# 仅返回名称列表
names = search_package_names("^py")  # 支持正则
```

#### Shell 补全（rez.complete）

```python
from rez_next.complete import get_completion_script

# 生成 bash 补全脚本
script = get_completion_script("bash")
print(script)
```

---

## 架构

Cargo workspace 包含 14 个 crate + Python bindings：

```
rez-next-common       共享错误类型、配置、工具函数
rez-next-version      版本解析、比较、范围（状态机解析器）
rez-next-package      包定义、package.py 解析（RustPython AST）
rez-next-solver       依赖求解（A* 算法 + 回溯 + 环检测）
rez-next-repository   仓库扫描和缓存
rez-next-context      已解析上下文、Rex 集成、序列化
rez-next-build        构建系统集成（cmake/make/python/cargo/nodejs）
rez-next-cache        多级缓存（内存 + 磁盘）
rez-next-rex          Rex 命令语言（完整 DSL + 5 种 shell 激活脚本）
rez-next-suites       Suite 管理（已解析上下文集合）
rez-next-bind         系统工具绑定（python/cmake/pip/git 等）
rez-next-search       包搜索（精确/包含/正则 FilterMode）
rez-next-python       Python 绑定 via PyO3（18 个子模块）
```

### 各组件状态

| Crate | 状态 | 测试数 |
|-------|------|--------|
| `rez-next-version` | 完成 | ~30 |
| `rez-next-package` | 完成 | ~25 |
| `rez-next-common` | 完成 | ~10 |
| `rez-next-rex` | 完成 | ~20 |
| `rez-next-solver` | 完成（A* 启用）| ~15 |
| `rez-next-context` | 完成 | ~12 |
| `rez-next-repository` | 完成 | ~8 |
| `rez-next-build` | 完成 | ~6 |
| `rez-next-cache` | 完成 | ~5 |
| `rez-next-suites` | 完成 | ~10 |
| `rez-next-bind` | 完成 | ~37 |
| `rez-next-search` | 完成 | ~16 |
| `rez-next-python` | 完成（18 子模块）| ~101 |
| Compat integration | — | ~210 |

---

## 从源码构建

```bash
git clone https://github.com/loonghao/rez-next
cd rez-next
cargo build --release
```

### 前置条件

- Rust 1.70+
- [just](https://github.com/casey/just)（可选，便捷命令）

### 常用命令

```bash
just build           # 开发构建
just build-release   # 发布构建
just test            # 运行所有测试
just lint            # Clippy
just fmt             # 格式化
just ci              # 完整 CI 检查
just bench           # 基准测试
```

---

## 测试

运行全量测试：

```bash
cargo test --exclude rez-next-python --workspace
```

各类测试覆盖：
- 版本语义（rez 兼容语义：`1.0 > 1.0.0`）
- 包序列化（package.py 解析、YAML/JSON 序列化）
- Rex DSL（setenv/prepend_path/alias/info/stop）
- Shell 脚本生成（bash/zsh/fish/PowerShell/cmd）
- Suite 管理（创建/保存/加载/工具冲突检测）
- 依赖求解（A* 算法、回溯、环检测）
- 上下文序列化（.rxt 文件读写）
- rez.diff（上下文差异比对）
- rez.status（环境变量读取、shell 检测）
- rez.search（精确/包含/正则过滤）
- rez.depends（反向依赖查询、传递依赖）

---

## 基准测试结果

使用 [Criterion.rs](https://github.com/bheisler/criterion.rs) 在 release 模式下测量。

### 版本操作

| 操作 | 耗时 |
|------|------|
| 解析单个版本 (`1.2.3-alpha.1`) | ~9.1 us |
| 比较两个版本 | ~6.8 ns |
| 排序 100 个版本 | ~19 us |
| 排序 1000 个版本 | ~176 us |
| 批量解析 1000 个版本 | ~9.0 ms |

### 包操作

| 操作 | 耗时 |
|------|------|
| 创建空包 | ~35 ns |
| 创建带版本号的包 | ~8.4 us |
| 序列化为 YAML | ~7.0 us |
| 序列化为 JSON | ~3.4 us |

<details>
<summary>复现方法</summary>

```bash
cargo bench --bench version_benchmark
cargo bench --bench simple_package_benchmark
```

</details>

---

## 文档

- [贡献指南](docs/contributing.md) — 开发工作流和 CI
- [基准测试指南](docs/benchmark_guide.md) — 运行和解读基准测试
- [性能指南](docs/performance.md) — 性能分析工具
- [Python 集成](docs/python-integration_zh.md) — Python 绑定使用说明
- [Pre-commit 配置](docs/PRE_COMMIT_SETUP.md) — 代码质量钩子

---

## 许可证

[Apache License 2.0](LICENSE)

## 致谢

- [Rez](https://github.com/AcademySoftwareFoundation/rez) — 本项目所实现的包管理器
- [Rust](https://www.rust-lang.org/) — 语言和生态
- [PyO3](https://pyo3.rs/) — Rust/Python 绑定框架
