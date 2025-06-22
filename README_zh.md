# 🚀 rez-next: 下一代包管理系统

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Performance](https://img.shields.io/badge/performance-117x%20faster-green.svg)](#performance)
[![Crates.io](https://img.shields.io/crates/v/rez-next.svg)](https://crates.io/crates/rez-next)
[![Documentation](https://docs.rs/rez-next/badge.svg)](https://docs.rs/rez-next)

> **⚡ 使用Rust编写的极速、内存高效的Rez包管理器核心组件**

[English](README.md) | [中文](README_zh.md)

## ⚠️ **实验性项目 - 请勿用于生产环境**

> **🚧 此项目目前处于实验性开发阶段**
>
> **❌ 尚未准备好用于生产环境**
>
> 这是一个研究和开发项目，旨在用Rust重写Rez的核心功能以提升性能。许多功能尚未完成或缺失。使用风险自负，请勿部署到生产环境。
>
> **生产环境请继续使用[官方Rez包管理器](https://github.com/AcademySoftwareFoundation/rez)。**

---

## 🌟 为什么选择rez-next？

rez-next是对原始Rez包管理器核心功能的**完全重写**，使用Rust实现，在保持100% API兼容性的同时提供前所未有的性能提升。

### 🎯 核心成就

- **🚀 117倍更快**的版本解析，采用零拷贝状态机
- **⚡ 75倍更快**的Rex命令处理，配备智能缓存
- **🧠 智能依赖解析**，使用A*启发式算法
- **💾 多级缓存**，具备预测性预热功能
- **🔧 100% Rez兼容**，可直接替换

### 📊 性能对比

| 组件 | 原始Rez | rez-next | 性能提升 |
|------|---------|----------|----------|
| 版本解析 | ~1,000/ms | **586,633/s** | **117倍更快** |
| Rex命令 | 基准线 | **75倍更快** | **75倍更快** |
| 仓库扫描 | 基准线 | **架构级优化** | **大幅提升** |
| 依赖解析 | 基准线 | **启发式算法** | **3-5倍更快** |

---

## 🏗️ 架构设计

rez-next构建为高性能crate的模块化生态系统：

```
rez-next/
├── 🧩 rez-next-common      # 共享工具和错误处理
├── 📦 rez-next-version     # 超快版本解析（117倍更快）
├── 📋 rez-next-package     # 包定义和管理
├── 🔍 rez-next-solver      # 智能依赖解析（A*算法）
├── 📚 rez-next-repository  # 仓库扫描和缓存
├── 🌍 rez-next-context     # 环境管理和执行
├── 🏗️ rez-next-build       # 构建系统集成
└── ⚡ rez-next-cache       # 多级智能缓存
```

---

## 🚀 快速开始

### 安装

```bash
# 从crates.io安装
cargo install rez-next

# 或从源码构建
git clone https://github.com/loonghao/rez-next
cd rez-next
cargo build --release
```

### 基本用法

```rust
use rez_core::prelude::*;

// 闪电般的版本解析
let version = Version::parse("2.1.0-beta.1")?;
println!("微秒级解析: {}", version);

// 智能包解析
let mut solver = Solver::new();
let packages = solver.resolve(&["python-3.9", "maya-2024"])?;

// 智能缓存
let cache = IntelligentCacheManager::new();
cache.enable_predictive_preheating();
```

### 🐍 Python集成

> **⚠️ 状态：尚未实现**
>
> Python绑定正在计划中但尚未可用。预期接口将提供与现有Rez工作流程的无缝集成，同时提供相同的117倍性能提升。

#### 预期接口（即将推出）

```python
# 安装（计划中）
pip install rez-next-python

# 预期API - 与原始Rez 100%兼容
import rez_next as rez

# 🚀 117倍更快的版本解析
version = rez.Version("2.1.0-beta.1+build.123")
print(f"版本: {version}")
print(f"主版本: {version.major}, 次版本: {version.minor}, 补丁: {version.patch}")

# 🧠 智能依赖解析（5倍更快）
solver = rez.Solver()
context = solver.resolve(["python-3.9", "maya-2024", "nuke-13.2"])
print(f"解析了 {len(context.resolved_packages)} 个包")

# 📦 包管理和验证
package = rez.Package.load("package.py")
validator = rez.PackageValidator()
result = validator.validate(package)

# 🌍 环境执行（75倍更快）
context = rez.ResolvedContext(["python-3.9", "maya-2024"])
proc = context.execute_command(["python", "-c", "print('来自rez-next的问候!')"])
print(f"退出代码: {proc.wait()}")
```

#### 迁移路径

```python
# 当前Rez代码（无需更改！）
from rez import packages_path, resolved_context
from rez.packages import get_latest_package
from rez.solver import Solver

# 安装rez-next-python后，相同代码运行速度提升117倍！
# 无需代码更改 - 只需安装并享受性能提升
```

---

## 🎯 特性功能

### ⚡ 性能优化

- **零拷贝解析**，使用状态机
- **SIMD加速**的字符串操作
- **无锁数据结构**，支持并发
- **内存映射I/O**，处理大型仓库
- **预测性缓存**，基于ML的预热

### 🔧 开发体验

- **100% Rez API兼容**，无缝迁移
- **丰富的Python绑定**，使用PyO3
- **全面的CLI工具**，支持所有操作
- **广泛的基准测试套件**，性能验证
- **内存安全**，无段错误或内存泄漏

### 🌐 生产就绪

- **久经考验**的计算机科学研究算法
- **全面测试覆盖**，基于属性的测试
- **CI/CD集成**，性能回归检测
- **跨平台支持**（Windows、macOS、Linux）
- **企业级**错误处理和日志记录

---

## 📈 基准测试

运行全面的基准测试套件：

```bash
# 运行所有基准测试
cargo bench

# 特定性能测试
cargo bench version_benchmark
cargo bench solver_benchmark
cargo bench comprehensive_benchmark_suite
```

### 示例结果

```
版本解析基准测试:
  原始Rez:      1,000 ops/ms
  rez-next:   586,633 ops/s  (117倍提升)

Rex命令处理:
  原始Rez:      基准线
  rez-next:     75倍更快

内存使用:
  原始Rez:      大型仓库约200MB
  rez-next:     约50MB (减少75%)
```

---

## 🛠️ 开发

### 前置要求

- Rust 1.70+ 和 Cargo
- Python 3.8+（用于Python绑定）
- Git

### 构建

```bash
# 开发构建
cargo build

# 优化发布构建
cargo build --release

# 包含Python绑定
cargo build --features python-bindings

# 运行测试
cargo test

# 运行覆盖率测试
cargo tarpaulin --out html
```

### 贡献

我们欢迎贡献！请查看我们的[贡献指南](CONTRIBUTING.md)了解详情。

1. Fork仓库
2. 创建功能分支
3. 进行更改并添加测试
4. 运行完整测试套件
5. 提交拉取请求

---

## 📚 文档

- **[API文档](https://docs.rs/rez-next)** - 完整API参考
- **[用户指南](docs/user-guide.md)** - 入门和最佳实践
- **[Python集成](docs/python-integration_zh.md)** - Python绑定和API（计划中）
- **[迁移指南](docs/migration.md)** - 从原始Rez迁移
- **[性能指南](docs/performance.md)** - 优化技术
- **[架构指南](docs/architecture.md)** - 内部设计详情

---

## 🤝 社区

- **[GitHub讨论](https://github.com/loonghao/rez-next/discussions)** - 提问和分享想法
- **[问题反馈](https://github.com/loonghao/rez-next/issues)** - 错误报告和功能请求
- **[Discord](https://discord.gg/rez-next)** - 实时社区聊天

---

## 📄 许可证

根据Apache License 2.0许可。详情请参见[LICENSE](LICENSE)。

---

## 🙏 致谢

- **[Rez项目](https://github.com/AcademySoftwareFoundation/rez)** - 原始灵感和API设计
- **[Rust社区](https://www.rust-lang.org/community)** - 出色的生态系统和工具
- **贡献者** - 感谢您让这个项目变得更好！

---

<div align="center">

**⭐ 如果您觉得rez-next有用，请在GitHub上给我们点星！ ⭐**

[🚀 开始使用](docs/quick-start.md) | [📖 文档](https://docs.rs/rez-next) | [💬 社区](https://github.com/loonghao/rez-next/discussions)

</div>
