# rez-next cleanup 执行记录

## 最新执行 (2026-04-01 12:59, 第八轮)

### 执行摘要
本轮完成三项高优先级清理任务：**孤立 pyo3 死文件删除**（#5）、**lint 配置收紧**（#2）、**重复 ResolutionResult 消除**（#3）。

共删除 ~2650 行代码（5 个文件），消除 glob re-export 歧义，收紧 lint 配置。

#### 阶段 1：孤立 pyo3 文件删除

**Commit** (`862609a`): 删除 5 个文件，-2636 行
- `version_token.rs` (371 行): 纯 pyo3 绑定，不在 lib.rs 模块树中
- `token.rs` (123 行): 含 PyVersionToken + VersionToken enum，不在模块树中
- `validation.rs` (1034 行): 全 #[pyclass]，pyo3 已从 Cargo.toml 注释掉
- `management.rs` (1077 行): 全 #[pyclass]，pyo3 已从 Cargo.toml 注释掉
- `version_token_tests.rs` (6 行): 空测试文件
- 同时更新 tests/mod.rs 移除 `pub mod version_token_tests`

#### 阶段 2：Lint 配置收紧

**Commit** (`e951391`): 
- `unexpected_cfgs`: `allow` → `warn`
- 在 `[features]` 中声明 `flamegraph` 和 `quick-benchmarks`（bench 文件使用）
- 更新 `unused_imports` 过时注释

#### 阶段 3：ResolutionResult 去重

**Commit** (`5f75334`):
- 删除 `solver.rs` 中重复的 `ResolutionResult`（12 行），改为 `use crate::resolution::ResolutionResult`
- 重命名 `dependency_resolver::ResolutionResult` → `DetailedResolutionResult`（字段完全不同的类型）
- 更新 CLI `solve.rs` 中 4 处函数签名

### 基线状态
- **分支**: `auto-improve`（已推送）
- **测试**: 540 passed, 0 failed（与清理前基线一致）
- **删除行数**: ~2650 lines（本轮）
- **累计删除**: ~5050 lines across 8 cycles

### 下一轮重点
1. **#6 rez-next-package 死文件评估**: `batch.rs`, `cache.rs`, `dependency.rs`, `variant.rs` 均不在 lib.rs 模块树中（合计 ~115 KB），需评估是否有纯 Rust 价值
2. **#7 进一步 lint 收紧**: 将 `unused_imports` 从 `allow` 改为 `warn`，清理未使用的 imports
3. **#4 dead_code helper functions**: 评估 exceptions_bindings.rs 中 5 个函数是否可删除
