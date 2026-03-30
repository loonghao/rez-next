# rez-next

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![CI](https://img.shields.io/github/actions/workflow/status/loonghao/rez-next/ci.yml?branch=main)](https://github.com/loonghao/rez-next/actions)

一个实验性项目，尝试用 Rust 重写 [Rez](https://github.com/AcademySoftwareFoundation/rez) 包管理器的核心组件。

[English](README.md) | [中文](README_zh.md)

---

## 警告

**这是一个个人实验项目。它没有达到生产可用的状态，也不打算供他人使用。**

大多数功能不完整，API 不稳定，不保证正确性或与官方 Rez 的兼容性。如果你需要包管理器，请使用 [Rez](https://github.com/AcademySoftwareFoundation/rez)。

---

## 这是什么

一个学习项目，探索用 Rust 重写 Rez 的性能关键子系统——版本解析、包表示、依赖求解、上下文管理以及 Rex 命令语言。

目标不是替代 Rez，而是了解这些子系统的原生实现是什么样的，以及对特定热路径是否能获得有意义的性能提升。

---

## 基准测试结果

使用 [Criterion.rs](https://github.com/bheisler/criterion.rs) 在 release 模式下测量（opt-level=3, LTO）。这些仅是 Rust 内部的微基准测试——不代表与 Python Rez 的对比。

### 版本操作

| 操作 | 耗时 |
|------|------|
| 解析单个版本 (`1.2.3-alpha.1`) | ~9.1 us |
| 状态机分词器（5 个版本） | ~535 ns |
| 比较两个版本 | ~6.8 ns |
| 排序 100 个版本 | ~19 us |
| 排序 1000 个版本 | ~176 us |
| 批量解析 1000 个版本 | ~9.0 ms |

### 包操作

| 操作 | 耗时 |
|------|------|
| 创建空包 | ~35 ns |
| 创建带版本号的包 | ~8.4 us |
| 创建复杂包（依赖 + 工具 + 变体） | ~8.9 us |
| 序列化为 YAML | ~7.0 us |
| 序列化为 JSON | ~3.4 us |

<details>
<summary>复现方法</summary>

```bash
vx cargo bench --bench version_benchmark
vx cargo bench --bench simple_package_benchmark
```

</details>

---

## 架构

Cargo workspace 包含 11 个 crate：

```
rez-next-common       共享错误类型、配置、工具
rez-next-version      版本解析、比较、范围
rez-next-package      包定义、package.py 解析（通过 RustPython AST）
rez-next-solver       依赖求解
rez-next-repository   仓库扫描和缓存
rez-next-context      已解析上下文和环境管理
rez-next-build        构建系统集成
rez-next-cache        多级缓存
rez-next-rex          Rex 命令语言
rez-next-suites       Suite 管理（已解析上下文集合）
rez-next-python       Python 绑定 via PyO3（仅有脚手架）
```

### 各组件状态

| Crate | 状态 | 说明 |
|-------|------|------|
| `rez-next-version` | 可用 | 解析、比较、范围、状态机解析器 |
| `rez-next-package` | 可用 | package.py 解析，序列化（YAML/JSON/Python） |
| `rez-next-common` | 可用 | 错误类型、配置 |
| `rez-next-rex` | 部分完成 | 命令结构、shell 生成器、执行器 |
| `rez-next-solver` | 部分完成 | 基础求解、回溯、环检测 |
| `rez-next-context` | 部分完成 | 上下文创建、环境生成、激活脚本 |
| `rez-next-repository` | 部分完成 | 扫描已搭建 |
| `rez-next-build` | 部分完成 | 构建系统检测 |
| `rez-next-cache` | 部分完成 | 缓存框架 |
| `rez-next-suites` | 部分完成 | Suite 管理基础 |
| `rez-next-python` | 脚手架 | PyO3 绑定存在但不可用 |

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
- [vx](https://github.com/loonghao/vx)（可选，环境管理器）

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

所有测试通过：

```bash
vx just test
```

---

## 文档

- [贡献指南](docs/contributing.md) — 开发工作流和 CI
- [基准测试指南](docs/benchmark_guide.md) — 运行和解读基准测试
- [性能指南](docs/performance.md) — 性能分析工具
- [Python 集成](docs/python-integration_zh.md) — 计划中的 Python 绑定（未实现）
- [Pre-commit 配置](docs/PRE_COMMIT_SETUP.md) — 代码质量钩子

---

## 许可证

[Apache License 2.0](LICENSE)

## 致谢

- [Rez](https://github.com/AcademySoftwareFoundation/rez) — 本项目所研究的包管理器
- [Rust](https://www.rust-lang.org/) — 语言和生态
