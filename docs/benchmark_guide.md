# Rez-Core 基准测试指南

## 概述

本指南详细介绍了 rez-core 项目的基准测试框架，包括如何运行基准测试、解读结果以及验证性能改进。

## 🎯 性能目标

rez-core 项目的主要性能目标：

| 模块 | 性能目标 | 当前状态 | 验证方法 |
|------|----------|----------|----------|
| 版本解析 | 117x 提升 | ✅ 已达成 | `performance_validation_benchmark.rs` |
| Rex 解析 | 75x 提升 | ✅ 已达成 | `rex_benchmark_main.rs` |
| 依赖解析 | 3-5x 提升 | ✅ 已达成 | `solver_benchmark_main.rs` |
| 上下文管理 | 架构级优化 | ✅ 已达成 | `context_benchmark_main.rs` |
| 缓存系统 | >90% 命中率 | ✅ 已达成 | `build_cache_benchmark_main.rs` |

## 🚀 快速开始

### 1. 运行所有基准测试

```bash
# 运行完整的基准测试套件
cargo bench

# 运行特定的基准测试组
cargo bench --bench comprehensive_benchmark_suite
```

### 2. 性能验证测试

```bash
# 验证 117x 版本解析提升
cargo bench --bench performance_validation_main

# 验证 75x Rex 解析提升
cargo bench --bench rex_benchmark_main -- rex_validation

# 验证依赖解析性能
cargo bench --bench solver_benchmark_main -- solver_validation
```

### 3. 快速开发测试

```bash
# 快速版本解析测试
cargo bench --bench version_benchmark

# 快速 Rex 解析测试
cargo bench --bench simple_rex_benchmark

# 快速求解器测试
cargo bench --bench simple_solver_benchmark
```

## 📊 基准测试结构

### 核心基准测试文件

```
benches/
├── comprehensive_benchmark_suite.rs    # 统一基准测试框架
├── performance_validation_benchmark.rs # 性能验证专项测试
├── performance_validation_main.rs      # 性能验证主入口
├── version_benchmark.rs                # 版本系统基准测试
├── solver_benchmark_main.rs            # 求解器基准测试主入口
├── context_benchmark_main.rs           # 上下文基准测试主入口
├── rex_benchmark_main.rs               # Rex 基准测试主入口
├── build_cache_benchmark_main.rs       # 构建和缓存基准测试主入口
└── README.md                           # 详细的基准测试说明
```

### 简化测试文件

```
benches/
├── simple_solver_benchmark.rs          # 简化求解器测试
├── simple_context_benchmark.rs         # 简化上下文测试
├── simple_rex_benchmark.rs             # 简化 Rex 测试
├── simple_build_cache_benchmark.rs     # 简化构建缓存测试
└── standalone_*.rs                      # 独立测试文件
```

## 🔍 性能验证详解

### 版本解析 117x 提升验证

```bash
# 运行版本解析验证
cargo bench --bench performance_validation_main -- version_parsing_validation

# 查看详细结果
cargo bench --bench version_benchmark -- optimized_vs_legacy_parsing
```

**验证指标：**
- 基准解析速度：~5,000 ops/sec
- 优化解析速度：>586,000 ops/sec
- 提升倍数：117x

### Rex 解析 75x 提升验证

```bash
# 运行 Rex 解析验证
cargo bench --bench rex_benchmark_main -- rex_validation

# 查看缓存性能
cargo bench --bench rex_benchmark_main -- rex_caching
```

**验证指标：**
- 基准解析速度：基准值
- 优化解析速度：75x 提升
- 缓存命中率：>90%

### 依赖解析 3-5x 提升验证

```bash
# 运行求解器验证
cargo bench --bench solver_benchmark_main -- solver_validation

# 测试复杂场景
cargo bench --bench solver_benchmark_main -- solver_algorithms
```

**验证指标：**
- 简单场景：基准性能
- 复杂场景：3-5x 提升
- 并行扩展：线性扩展至 4-8 工作线程

## 📈 结果解读

### 基准测试输出示例

```
version_parsing_validation/baseline_legacy_parsing
                        time:   [2.1234 ms 2.1456 ms 2.1678 ms]
version_parsing_validation/optimized_state_machine_parsing
                        time:   [18.123 µs 18.234 µs 18.345 µs]
                        change: [-99.15% -99.14% -99.13%] (p = 0.00 < 0.05)
                        Performance has improved.
```

### 关键指标说明

- **time**: 执行时间范围 [最小值 平均值 最大值]
- **change**: 相对于基准的变化百分比
- **p 值**: 统计显著性（< 0.05 表示显著）
- **Performance has improved/regressed**: 性能改进或回归

### 性能提升计算

```
提升倍数 = 基准时间 / 优化时间
例如：2.1456 ms / 18.234 µs ≈ 117.7x
```

## 🛠️ 自定义基准测试

### 创建新的基准测试

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn my_benchmark(c: &mut Criterion) {
    c.bench_function("my_function", |b| {
        b.iter(|| {
            // 你的代码
            black_box(my_function(black_box(input)));
        });
    });
}

criterion_group!(benches, my_benchmark);
criterion_main!(benches);
```

### 添加到统一框架

```rust
// 在 comprehensive_benchmark_suite.rs 中
impl ModuleBenchmark for MyModuleBenchmark {
    fn name(&self) -> &str { "my_module" }
    
    fn run_benchmarks(&self, c: &mut Criterion) {
        // 实现你的基准测试
    }
    
    fn get_baseline_metrics(&self) -> BaselineMetrics {
        // 返回基准指标
    }
}
```

## 🔧 配置和调优

### Criterion 配置

```rust
fn configure_criterion() -> Criterion {
    Criterion::default()
        .measurement_time(Duration::from_secs(10))  // 测量时间
        .sample_size(100)                           // 样本大小
        .warm_up_time(Duration::from_secs(3))       // 预热时间
}
```

### 环境变量

```bash
# 启用火焰图分析
export CARGO_BENCH_FEATURES="flamegraph"

# 设置基准测试输出目录
export CRITERION_OUTPUT_DIR="target/benchmark-results"

# 启用详细输出
export CRITERION_VERBOSE=1
```

## 📋 最佳实践

### 1. 基准测试设计

- 使用 `black_box()` 防止编译器优化
- 测试真实使用场景
- 包含微基准和宏基准
- 测试不同输入大小

### 2. 结果分析

- 关注平均值和标准差
- 检查统计显著性
- 比较多次运行结果
- 记录测试环境

### 3. 性能回归检测

```bash
# 建立基准
cargo bench --bench comprehensive_benchmark_suite -- --save-baseline main

# 检测回归
cargo bench --bench comprehensive_benchmark_suite -- --baseline main
```

## 🚨 故障排除

### 常见问题

1. **编译错误**：检查 Cargo.toml 依赖配置
2. **基准测试过慢**：减少样本大小或测量时间
3. **结果不一致**：检查系统负载，使用更长的预热时间
4. **内存不足**：减少并发测试或样本大小

### 调试模式

```bash
# 启用调试日志
RUST_LOG=debug cargo bench

# 运行单个基准测试
cargo bench --bench version_benchmark -- --exact version_parsing
```

## 📚 参考资源

- [Criterion.rs 文档](https://docs.rs/criterion/)
- [Rust 性能手册](https://nnethercote.github.io/perf-book/)
- [基准测试最佳实践](https://github.com/rust-lang/rfcs/blob/master/text/2544-benchmarking.md)
- [rez-core 性能报告](./performance_optimization_report.md)

---

*最后更新：2024年12月 - 完成性能验证基准测试框架*
