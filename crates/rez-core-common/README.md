# rez-core-common

[![Crates.io](https://img.shields.io/crates/v/rez-core-common.svg)](https://crates.io/crates/rez-core-common)
[![Documentation](https://docs.rs/rez-core-common/badge.svg)](https://docs.rs/rez-core-common)
[![License](https://img.shields.io/crates/l/rez-core-common.svg)](LICENSE)
[![Build Status](https://github.com/loonghao/rez-core/workflows/CI/badge.svg)](https://github.com/loonghao/rez-core/actions)

[中文文档](README_zh.md) | [English](README.md)

**Common utilities and types for Rez Core** - The foundational building blocks for high-performance package management.

## 🚀 Features

- **Error Handling**: Comprehensive error types with detailed context
- **Configuration Management**: Flexible configuration system with validation
- **Utilities**: Common helper functions and macros
- **Type Safety**: Strong typing with serde serialization support
- **Performance**: Zero-cost abstractions and optimized data structures

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rez-core-common = "0.1.0"
```

## 🔧 Usage

### Error Handling

```rust
use rez_core_common::{RezCoreError, RezCoreResult};

fn example_function() -> RezCoreResult<String> {
    // Your code here
    Ok("Success".to_string())
}

// Handle errors gracefully
match example_function() {
    Ok(result) => println!("Success: {}", result),
    Err(e) => eprintln!("Error: {}", e),
}
```

### Configuration

```rust
use rez_core_common::Config;

let config = Config::default();
println!("Config loaded: {:?}", config);
```

## 🏗️ Architecture

This crate provides the foundational types and utilities used across the entire Rez Core ecosystem:

- **Error Types**: Standardized error handling across all crates
- **Configuration**: Centralized configuration management
- **Utilities**: Common helper functions and type definitions

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](../../CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the Apache License 2.0 - see the [LICENSE](../../LICENSE) file for details.

## 🔗 Related Crates

- [`rez-core-version`](../rez-core-version) - Version management and parsing
- [`rez-core-package`](../rez-core-package) - Package definitions and operations
- [`rez-core-repository`](../rez-core-repository) - Repository management and scanning

## 📊 Performance

Built with performance in mind:
- Zero-cost abstractions
- Minimal memory allocations
- Optimized for high-throughput operations

---

Part of the [Rez Core](https://github.com/loonghao/rez-core) project - A high-performance Rust implementation of the Rez package manager.
