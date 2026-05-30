# rez-next

[![Rust](https://img.shields.io/badge/rust-1.95+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![CI](https://img.shields.io/github/actions/workflow/status/loonghao/rez-next/ci.yml?branch=main)](https://github.com/loonghao/rez-next/actions)
[![GitHub Release](https://img.shields.io/github/v/release/loonghao/rez-next)](https://github.com/loonghao/rez-next/releases)
[![Crates.io](https://img.shields.io/crates/v/rez-next)](https://crates.io/crates/rez-next)
[![Crates.io Downloads](https://img.shields.io/crates/d/rez-next)](https://crates.io/crates/rez-next)
[![PyPI - Version](https://img.shields.io/pypi/v/rez-next)](https://pypi.org/project/rez-next/)
[![PyPI - Downloads](https://img.shields.io/pypi/dm/rez-next)](https://pypi.org/project/rez-next/)
[![PyPI - Python Version](https://img.shields.io/pypi/pyversions/rez-next)](https://pypi.org/project/rez-next/)
[![Coverage](https://img.shields.io/codecov/c/gh/loonghao/rez-next/main)](https://codecov.io/gh/loonghao/rez-next)

> ⚠️ **实验性项目。** rez-next 是一个 Rust 实验项目，探索完整重写 [Rez](https://github.com/AcademySoftwareFoundation/rez) 包管理器的可能性。它**不是**生产就绪的，**不是** AcademySoftwareFoundation 官方项目，API 可能随时变更。请将其用于评估、基准测试与研究，**不要**用于工作室关键管线。

一个用 Rust 实现的 [Rez](https://github.com/AcademySoftwareFoundation/rez) 包管理器实验项目，提供 Python 绑定且兼容覆盖度持续增长。许多常用工作流已经可以通过 `import rez_next` 使用，但目前尚不能在所有 API 层面实现无缝切换。

[English](README.md) | [中文](README_zh.md)

---

## 安装

### Linux / macOS

```bash
curl -fsSL https://raw.githubusercontent.com/loonghao/rez-next/main/install.sh | sh
```

或指定版本安装：

```bash
REZ_NEXT_VERSION=0.3.1 curl -fsSL https://raw.githubusercontent.com/loonghao/rez-next/main/install.sh | sh
```

环境变量说明：

| 变量 | 说明 | 默认值 |
|---|---|---|
| `REZ_NEXT_VERSION` | 指定安装版本（如 `0.3.0`） | 最新版本 |
| `REZ_NEXT_INSTALL` | 安装目录 | `$HOME/.rez-next/bin` |
| `REZ_NEXT_MUSL` | Linux 强制使用 musl 构建 | 自动检测 |

### Windows（PowerShell）

```powershell
irm https://raw.githubusercontent.com/loonghao/rez-next/main/install.ps1 | iex
```

### Python（PyPI）

```bash
pip install rez-next
```

### 从源码构建

```bash
git clone https://github.com/loonghao/rez-next
cd rez-next
cargo build --release
```

---

## 自动更新

使用内置的 `self-update` 命令保持 rez-next 为最新版本：

```bash
# 更新到最新版本
rez-next self-update

# 仅检查是否有更新（不安装）
rez-next self-update --check

# 更新到指定版本
rez-next self-update --version 0.3.1

# 强制重新安装当前版本
rez-next self-update --force
```

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

### 已实现的 Python 子模块（41 个模块）

| 子模块 | 等价 rez 模块 | 功能 |
|--------|--------------|------|
| `rez_next.version` | `rez.vendor.version` | 版本解析、比较、范围 |
| `rez_next.packages_` | `rez.packages_` | 包迭代、查询、复制/移动/删除 |
| `rez_next.packages` | `rez.packages` | 包对象模型 |
| `rez_next.resolved_context` | `rez.resolved_context` | 依赖解析、上下文管理 |
| `rez_next.suite` | `rez.suite` | Suite 创建、工具链管理 |
| `rez_next.config` | `rez.config` | 配置读取（100+ 字段）|
| `rez_next.system` | `rez.system` | 系统信息（平台、Python版本等） |
| `rez_next.shell` | `rez.shells` | Shell 脚本生成（bash/zsh/fish/PowerShell/cmd）|
| `rez_next.rex` | `rez.rex` | Rex 命令语言解释器 |
| `rez_next.build_` | `rez.build_` | 包构建系统集成 |
| `rez_next.build_plugins` | `rez.build_plugins` | 构建插件 |
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
| `rez_next.data` | `rez.data` | 内置数据资源 |
| `rez_next.cli` | `rez.cli` | CLI 入口（程序化调用）|
| `rez_next.exceptions` | `rez.exceptions` | 异常类 |
| `rez_next.deprecations` | `rez.utils.deprecations` | 弃用警告 |
| `rez_next.package_cache` | `rez.package_cache` | 包负载缓存 |
| `rez_next.package_help` | `rez.package_help` | 包帮助 |
| `rez_next.package_search` | `rez.package_search` | 包搜索 API |
| `rez_next.package_remove` | `rez.package_remove` | 包删除 |
| `rez_next.solver_` | `rez.solver` | 依赖求解器（部分实现）|
| `rez_next.solver` | `rez.solver` | 高级求解器 API |
| `rez_next.serialise_` | `rez.serialise` | 序列化支持 |
| `rez_next.test` | `rez.test` | 包测试 |
| `rez_next.util` | `rez.utils` | 通用工具函数 |
| `rez_next.utils.logging_` | `rez.utils.logging_` | 日志工具 |
| `rez_next.utils.resources` | `rez.utils.resources` | 资源加载工具 |
| `rez_next.vendor.version` | `rez.vendor.version` | 内置版本模块 |

> **25+ 模块正在积极开发中**，位于 [`auto-improve`](https://github.com/loonghao/rez-next/tree/auto-improve) 分支：`bundle_context`, `build_process`, `build_system`, `command`, `developer_package`, `package_bind`, `package_copy`, `package_filter`, `package_maker`, `package_move`, `package_order`, `package_resources`, `package_serialise`, `package_test`, `plugin_managers`, `release_hook`, `release_vcs`, `resolver`, `rex_bindings`, `shells`, `wrapper`, `utils.*` 扩展, `rezconfig`。

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

Cargo workspace 包含 20 个 crate，其中 `rez-next-python` 提供 Python bindings：

```
rez-next-common        共享错误类型、配置、工具函数
rez-next-config        配置加载与校验
rez-next-version       版本解析、比较、范围（状态机解析器）
rez-next-package       包定义、package.py 解析（RustPython AST）
rez-next-package-cache 包负载缓存
rez-next-package-filter 包过滤（glob, regex, range rules）
rez-next-solver        依赖求解（A* 算法 + 回溯 + 环检测）
rez-next-repository    仓库扫描和缓存
rez-next-context       已解析上下文、Rex 集成、序列化
rez-next-build         构建系统集成（cmake/make/python/cargo/nodejs）
rez-next-cache         多级缓存（内存 + 磁盘）
rez-next-rex           Rex 命令语言（完整 DSL + 5 种 shell 激活脚本）
rez-next-suites        Suite 管理（已解析上下文集合）
rez-next-bind          系统工具绑定（python/cmake/pip/git 等）
rez-next-search        包搜索（精确/包含/正则 FilterMode）
rez-next-explicit      显式包列表
rez-next-serialise     包序列化
rez-next-release-hook  发布钩子
rez-next-util          工具函数（命令执行等）
rez-next-python        Python 绑定 via PyO3（41 个子模块）
```

### 各组件状态

| Crate | 状态 | 测试数 |
|-------|------|--------|
| `rez-next-version` | 核心能力较成熟 | ~30 |
| `rez-next-package` | 核心能力较成熟 | ~25 |
| `rez-next-common` | 核心能力较成熟 | ~10 |
| `rez-next-config` | 稳定 | ~8 |
| `rez-next-rex` | 核心能力较成熟 | ~20 |
| `rez-next-solver` | 持续演进中（A* 已启用）| ~15 |
| `rez-next-context` | 持续演进中 | ~12 |
| `rez-next-repository` | 核心能力较成熟 | ~8 |
| `rez-next-build` | 局部实现（基础流程 + placeholders） | ~6 |
| `rez-next-cache` | 持续演进中 | ~5 |
| `rez-next-suites` | 持续演进中 | ~10 |
| `rez-next-bind` | 持续演进中 | ~37 |
| `rez-next-search` | 持续演进中 | ~16 |
| `rez-next-package-cache` | 稳定 | ~8 |
| `rez-next-package-filter` | 稳定 | ~12 |
| `rez-next-release-hook` | 稳定 | ~6 |
| `rez-next-serialise` | 稳定 | ~5 |
| `rez-next-explicit` | 稳定 | ~5 |
| `rez-next-util` | 稳定 | ~5 |
| `rez-next-python` | 部分兼容（41 个子模块） | ~125 |
| Compat integration | 覆盖面持续增长 | ~210 |

---

## 从源码构建

```bash
git clone https://github.com/loonghao/rez-next
cd rez-next
cargo build --release
```

### 前置条件

- Rust 1.95+
- [just](https://github.com/casey/just)（可选，便捷命令）

### 常用命令

```bash
vx just build           # 开发构建
vx just build-release   # 发布构建
vx just test            # 运行所有测试
vx just lint            # Clippy
vx just fmt             # 格式化
vx just ci              # 完整 CI 检查
vx just bench           # 基准测试
```

---

## 测试

运行全量测试：

```bash
# 仅运行 Rust 测试
cargo test --workspace

# 运行 Rust + Python 绑定测试（需要先运行 maturin develop --release）
maturin develop --release
pytest
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

### Python API 性能

使用 `pytest-benchmark` 测量（Python 层调用 Rust 核心）。

| 操作 | 耗时 (平均) | 吞吐量 |
|------|-------------|--------|
| `pip_install()` | ~420 ns | 2.38M ops/sec |
| `walk_packages()` | ~42 μs | 23.9K ops/sec |
| `get_pip_dependencies()` | ~293 μs | 3.41K ops/sec |

> 基准测试结果来自 Cycle 188 (pytest-benchmark, Python 3.12)。

---

## 文档

- [贡献指南](docs/contributing.md) — 开发工作流和 CI
- [Python 集成](docs/python-integration_zh.md) — Python 绑定使用说明与模块覆盖
- [基准测试指南](docs/benchmark_guide.md) — 运行和解读基准测试
- [性能指南](docs/performance.md) — 性能分析工具
- [Pre-commit 配置](docs/PRE_COMMIT_SETUP.md) — 代码质量钩子

### 面向 AI Agent
- [AGENTS.md](AGENTS.md) — 渐进式信息披露地图（从这里开始）
- [llms.txt](llms.txt) — AI 友好的精简用法索引
- [llms-full.txt](llms-full.txt) — 完整 API 参考
- [CLAUDE.md](CLAUDE.md) — Claude 专用指南
- [GEMINI.md](GEMINI.md) — Gemini 专用指南

---

## 许可证

[Apache License 2.0](LICENSE)

## 致谢

- [Rez](https://github.com/AcademySoftwareFoundation/rez) — 本项目所实现的包管理器
- [Rust](https://www.rust-lang.org/) — 语言和生态
- [PyO3](https://pyo3.rs/) — Rust/Python 绑定框架
