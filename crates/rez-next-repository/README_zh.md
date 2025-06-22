# rez-next-repository

[![Crates.io](https://img.shields.io/crates/v/rez-next-repository.svg)](https://crates.io/crates/rez-next-repository)
[![Documentation](https://docs.rs/rez-next-repository/badge.svg)](https://docs.rs/rez-next-repository)
[![License](https://img.shields.io/crates/l/rez-next-repository.svg)](LICENSE)
[![Build Status](https://github.com/loonghao/rez-next/workflows/CI/badge.svg)](https://github.com/loonghao/rez-next/actions)

[中文文档](README_zh.md) | [English](README.md)

**Rez Next 高性能仓库管理** - 快速、可靠的包发现、仓库扫描和包操作。

## 🚀 特性

- **快速仓库扫描**: 优化的并行包仓库扫描
- **包发现**: 智能包发现，支持缓存和索引
- **仓库管理**: 全面的仓库操作和管理
- **性能优化**: 高吞吐量扫描，智能缓存
- **兼容性**: 与原始 Rez 仓库语义完全兼容

## 📦 安装

在你的 `Cargo.toml` 中添加：

```toml
[dependencies]
rez-next-repository = "0.1.0"
```

## 🔧 使用方法

### 基本仓库操作

```rust
use rez_next_repository::{Repository, RepositoryManager};

// 创建仓库管理器
let mut manager = RepositoryManager::new();

// 添加仓库
let repo = Repository::new("/path/to/packages".into());
manager.add_repository(repo);

// 扫描包
let packages = manager.scan_all_repositories().await?;
println!("找到 {} 个包", packages.len());
```

### 包发现

```rust
use rez_next_repository::{RepositoryScanner, ScanOptions};

// 创建带选项的扫描器
let scanner = RepositoryScanner::new();
let options = ScanOptions::default()
    .with_parallel_scanning(true)
    .with_caching(true);

// 扫描仓库
let results = scanner.scan_repository("/path/to/packages", options).await?;
for result in results {
    println!("找到包: {} v{}", result.name, result.version);
}
```

### 仓库缓存

```rust
use rez_next_repository::{RepositoryCache, CacheOptions};

// 创建带选项的缓存
let cache_options = CacheOptions::default()
    .with_ttl(3600) // 1 小时 TTL
    .with_max_entries(10000);

let cache = RepositoryCache::new(cache_options);

// 缓存操作在扫描过程中自动进行
```

## 🏗️ 架构

这个 crate 提供全面的仓库管理功能：

- **Repository**: 核心仓库类型，支持扫描和管理
- **RepositoryManager**: 多仓库管理和协调
- **RepositoryScanner**: 高性能并行扫描
- **RepositoryCache**: 智能缓存，支持 TTL 和 LRU 淘汰

## 📊 性能

针对高性能场景优化：
- 并行仓库扫描
- 智能缓存和索引
- 最小内存占用
- 快速包发现操作

## 🤝 贡献

我们欢迎贡献！请查看我们的[贡献指南](../../CONTRIBUTING.md)了解详情。

## 📄 许可证

本项目采用 Apache License 2.0 许可证 - 详情请查看 [LICENSE](../../LICENSE) 文件。

## 🔗 相关 Crate

- [`rez-next-common`](../rez-next-common) - 通用工具和错误处理
- [`rez-next-version`](../rez-next-version) - 版本管理和解析
- [`rez-next-package`](../rez-next-package) - 包定义和操作

---

[Rez Next](https://github.com/loonghao/rez-next) 项目的一部分 - Rez 包管理器的高性能 Rust 实现。
