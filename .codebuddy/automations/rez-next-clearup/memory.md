# rez-next cleanup 执行记录

## 最新执行 (2026-04-01 02:54, 第五轮)

### 执行摘要
本轮清理聚焦于 CLEANUP_TODO.md 高优先级 #1：**python-bindings feature 大清理**。完成了 Phase 1（lib.rs 文件和结构性代码），Phase 2（源文件 impl 块）的一半。

#### 阶段 1：过期代码清理 — python-bindings feature gates

**Commit 1** (`d718913`): 9 files, -459 lines
- 6 crate `lib.rs` 文件: 移除 `#[pymodule]`, `use pyo3`, 条件 `pub mod`, 条件 re-exports
- `rez-next-common/error.rs`: 移除 `PyO3` error variant 和 `create_exception!`
- `rez-next-common/config.rs`: 移除 `cfg_attr(pyclass)`, 合并双 impl
- `rez-next-version/tests/version_token_tests.rs`: 清空死测试模块
- `rez-next-package/lib.rs`: 移除 6 条件 mod, 7 re-exports, pymodule 块, 6 死测试

**Commit 2** (`e4d49bb`): 6 files, -221 lines
- `solver.rs`: 移除 `#[pymethods]` impl, `use pyo3`, `cfg_attr pyclass`
- `builder.rs`: 移除 `#[pymethods]` impl (build_package_py 等)
- `process.rs`: 移除 `#[pymethods]` impl (getters)
- `repository.rs`: 移除 `cfg_attr pyclass/pymethods/new/getter`
- `filesystem.rs`: 移除 `cfg_attr pyclass/pymethods/new/getter`
- `context.rs`: 移除 `#[pymethods]` impl, 6 双门控结构体字段

**Commit 3** (`b536915`): 更新 CLEANUP_TODO.md 记录进度

### 基线状态
- **分支**: `auto-improve`（已推送）
- **测试**: 1290 passed, 0 failed（与清理前基线一致）
- **Clippy**: 0 errors
- **删除行数**: ~680 lines（本轮清理）

### 下一轮重点
1. **python-bindings Phase 2**：清理 `version.rs`（19 blocks）, `package.rs`（10）, `variant.rs`（8）中的双门控代码
2. **孤立文件评估**：`batch.rs`, `cache.rs`, `dependency.rs`, `management.rs`, `validation.rs`, `variant.rs` 不再被 lib.rs 引用
3. **cfg_attr 清理**：`dependency.rs`(13), `cache.rs`(8), `batch.rs`(11) 中的 `cfg_attr` 注解
4. **lint 配置收紧**：将 `dead_code` 从 `allow` 改为 `warn`

### 注意事项
- `version.rs`/`package.rs`/`variant.rs` 中有复杂的双门控模式（python 版 vs non-python 版的 fields/Clone/parse/compare），需仔细合并
- 迭代 Agent 同时在 `auto-improve` 分支活跃，需注意合并冲突
