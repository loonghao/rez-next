# rez-next-common

[![Crates.io](https://img.shields.io/crates/v/rez-next-common.svg)](https://crates.io/crates/rez-next-common)
[![Documentation](https://docs.rs/rez-next-common/badge.svg)](https://docs.rs/rez-next-common)
[![License](https://img.shields.io/crates/l/rez-next-common.svg)](LICENSE)
[![Build Status](https://github.com/loonghao/rez-next/workflows/CI/badge.svg)](https://github.com/loonghao/rez-next/actions)

[中文文档](README_zh.md) | [English](README.md)

**Rez Next 通用工具和类型** - 高性能包管理的基础构建块。

## 🚀 特性

- **错误处理**: 带有详细上下文的综合错误类型
- **配置管理**: 带有验证的灵活配置系统
- **工具函数**: 常用辅助函数和宏
- **类型安全**: 强类型系统，支持 serde 序列化
- **性能优化**: 零成本抽象和优化的数据结构

## 📦 安装

在你的 `Cargo.toml` 中添加：

```toml
[dependencies]
rez-next-common = "0.1.0"
```

## 🔧 使用方法

### 错误处理

```rust
use rez_next_common::{RezCoreError, RezCoreResult};

fn example_function() -> RezCoreResult<String> {
    // 你的代码
    Ok("Success".to_string())
}

// 优雅地处理错误
match example_function() {
    Ok(result) => println!("成功: {}", result),
    Err(e) => eprintln!("错误: {}", e),
}
```

### 配置

```rust
use rez_next_common::Config;

let config = Config::default();
println!("配置已加载: {:?}", config);
```

## 🏗️ 架构

这个 crate 提供了整个 Rez Next 生态系统中使用的基础类型和工具：

- **错误类型**: 跨所有 crate 的标准化错误处理
- **配置**: 集中式配置管理
- **工具函数**: 常用辅助函数和类型定义

## 🤝 贡献

我们欢迎贡献！请查看我们的[贡献指南](../../CONTRIBUTING.md)了解详情。

## 📄 许可证

本项目采用 Apache License 2.0 许可证 - 详情请查看 [LICENSE](../../LICENSE) 文件。

## 🔗 相关 Crate

- [`rez-next-version`](../rez-next-version) - 版本管理和解析
- [`rez-next-package`](../rez-next-package) - 包定义和操作
- [`rez-next-repository`](../rez-next-repository) - 仓库管理和扫描

## 📊 性能

以性能为核心设计：
- 零成本抽象
- 最小内存分配
- 针对高吞吐量操作优化

---

[Rez Next](https://github.com/loonghao/rez-next) 项目的一部分 - Rez 包管理器的高性能 Rust 实现。
