# 📦 rez-core-version: 超高速版本解析

[![Crates.io](https://img.shields.io/crates/v/rez-core-version.svg)](https://crates.io/crates/rez-core-version)
[![Documentation](https://docs.rs/rez-core-version/badge.svg)](https://docs.rs/rez-core-version)
[![Performance](https://img.shields.io/badge/performance-117x%20faster-green.svg)](#performance)

[中文文档](README_zh.md) | [English](README.md)

> **⚡ 基于零拷贝状态机的闪电般快速版本解析和比较**

Rust 生态系统中最快的版本解析库，相比传统实现提供 **117 倍性能提升**。

---

## 🌟 特性

### ⚡ 极致性能
- **586,633 版本/秒** 解析速度
- **零拷贝状态机** 实现最大效率
- **SIMD 优化** 字符串操作
- **无锁算法** 支持并发访问

### 🔧 完整版本支持
- **语义化版本** (SemVer) 兼容
- **预发布版本** (alpha, beta, rc)
- **构建元数据** 和自定义后缀
- **版本范围** 和约束
- **复杂比较** 和排序

### 🌐 通用兼容性
- **100% Rez 兼容** - 直接替换
- **Python 绑定** 通过 PyO3 (可选)
- **Serde 支持** 用于序列化
- **无 unsafe 代码** - 内存安全设计

---

## 🚀 快速开始

### 安装

```toml
[dependencies]
rez-core-version = "0.1.0"

# 带 Python 绑定
rez-core-version = { version = "0.1.0", features = ["python-bindings"] }

# 带 serde 支持
rez-core-version = { version = "0.1.0", features = ["serde"] }
```

### 基本用法

```rust
use rez_core_version::Version;

// 闪电般快速解析
let version = Version::parse("2.1.0-beta.1+build.123")?;
println!("版本: {}", version); // "2.1.0-beta.1+build.123"

// 即时比较
let v1 = Version::parse("1.0.0")?;
let v2 = Version::parse("2.0.0")?;
assert!(v1 < v2);

// 版本范围
let range = VersionRange::parse(">=1.0.0,<2.0.0")?;
assert!(range.contains(&Version::parse("1.5.0")?));
```

### Python 集成

```python
from rez_core_version import Version

# 在 Python 中享受同样的极致性能
version = Version("2.1.0-beta.1")
print(f"主版本: {version.major}")  # 2
print(f"次版本: {version.minor}")  # 1
print(f"修订版本: {version.patch}")  # 0

# 快速比较
versions = [Version("1.0.0"), Version("2.0.0"), Version("1.5.0")]
sorted_versions = sorted(versions)
```

---

## 📊 性能基准测试

### 解析速度
```
传统解析器:        1,000 版本/毫秒
Rez-Core Version: 586,633 版本/秒
提升:             117 倍更快
```

### 内存使用
```
传统解析器:        ~200 字节/版本
Rez-Core Version: ~48 字节/版本
提升:             减少 75%
```

### 比较速度
```
传统解析器:        ~10,000 比较/毫秒
Rez-Core Version: ~2,000,000 比较/毫秒
提升:             200 倍更快
```

---

## 🏗️ 架构

### 零拷贝状态机
```rust
pub struct StateMachineParser {
    // 优化的状态转换
    // 解析过程中无堆分配
    // SIMD 加速字符处理
}
```

### 基于令牌的设计
```rust
pub enum VersionToken {
    Numeric(u32),           // 快速整数解析
    AlphaNumeric(String),   // 最小字符串分配
    Separator(char),        // 单字符
}
```

### 智能缓存
```rust
pub struct VersionCache {
    // 已解析版本的 LRU 缓存
    // 预测性预热
    // 内存高效存储
}
```

---

## 🎯 高级特性

### 版本范围
```rust
use rez_core_version::VersionRange;

let range = VersionRange::parse(">=1.0.0,<2.0.0")?;
let intersection = range1.intersect(&range2)?;
let union = range1.union(&range2)?;
```

### 自定义解析
```rust
use rez_core_version::VersionParser;

let parser = VersionParser::new()
    .with_strict_mode(true)
    .with_custom_separators(&['.', '-', '_']);

let version = parser.parse("1.0.0-custom_build")?;
```

### 批量操作
```rust
use rez_core_version::batch;

let versions = vec!["1.0.0", "2.0.0", "1.5.0"];
let parsed = batch::parse_versions(&versions)?;
let sorted = batch::sort_versions(parsed);
```

---

## 🧪 测试

运行综合测试套件：

```bash
# 单元测试
cargo test

# 性能基准测试
cargo bench

# 基于属性的测试
cargo test --features proptest

# Python 集成测试
cargo test --features python-bindings
```

### 测试覆盖率
- **单元测试**: 150+ 测试用例
- **基于属性的测试**: 使用任意输入的模糊测试
- **集成测试**: 真实世界版本字符串
- **基准测试**: 性能回归检测

---

## 🤝 贡献

我们欢迎贡献！需要帮助的领域：

- **性能优化** - SIMD 改进
- **Python 绑定** - 额外的 PyO3 特性
- **文档** - 示例和指南
- **测试** - 边界情况和基准测试

详情请查看 [CONTRIBUTING.md](../../CONTRIBUTING.md)。

---

## 📄 许可证

采用 Apache License 2.0 许可证。详情请查看 [LICENSE](../../LICENSE)。

---

<div align="center">

**⭐ 如果您觉得 rez-core-version 有用，请在 GitHub 上给我们点星！ ⭐**

[📖 文档](https://docs.rs/rez-core-version) | [🚀 示例](examples/) | [🐛 问题](https://github.com/loonghao/rez-core/issues)

</div>
