# rez-next cleanup 执行记录

## 最新执行 (2026-04-01 09:31, 第七轮)

### 执行摘要
本轮完成了 **python-bindings 大清理的最终阶段** — `version.rs` 双门控合并（最复杂的单文件）。同时清理了所有剩余的 cfg_attr/cfg gates、注释残留。

**python-bindings 清理现已 100% 完成**，crates 目录下（排除 rez-next-python）零残留。

#### 阶段 1：过期代码清理

**Commit 1** (`fbea1e2`): 12 files, -1005 lines (+70 refactored)
- `version.rs`: 完整双门控合并 — 移除 ~850 行：
  - 整个 `#[pymethods]` impl 块 (230 行)
  - 双 struct fields (Vec<PyObject> vs Vec<String>)
  - 双 `parse()`, `Clone`, `compare_rez()`, `is_prerelease()`, `compare_token_strings()`, `reconstruct_string()`
  - Python helpers: `create_version_with_python_tokens`, `extract_token_strings_gil_free`, `parse_optimized`, `parse_legacy_simulation`, `parse_with_gil_release`, `cmp_with_gil_release`
  - `OPTIMIZED_PARSER` static, imports: pyo3, PyTuple, AlphanumericVersionToken, once_cell, StateMachineParser
- `parser.rs`: 移除 `use VersionToken` 和 `parse_tokens()` 死方法
- `environment.rs`: 移除注释掉的 `#[pyclass]` 和 `/* #[pymethods] ... */` 块
- `shell.rs`: 移除 `// use pyo3::prelude::*;` 注释
- `context/lib.rs`: 移除注释掉的 pyo3 import 和 pymodule 块
- `batch.rs`: 移除 `use pyo3` 和 12 个 `cfg_attr` 注解
- `cache.rs`: 移除 `use pyo3` 和 6 个 `cfg_attr` 注解
- `dependency.rs`: 移除 3 个 `cfg_attr` 注解
- `version/lib.rs`: 移除过时文档行

**Commit 2** (`e52a09c`): 更新 CLEANUP_TODO.md
**Commit 3** (`52534b4`): 移除 Cargo.toml 中过时的 pyo3 注释

### 基线状态
- **分支**: `auto-improve`（已推送）
- **测试**: 1290 passed, 0 failed（与清理前基线一致）
- **删除行数**: ~935 lines（本轮 version.rs 合并 + 残留清理）
- **python-bindings 总计**: ~2400 lines removed across 7 cycles

### 下一轮重点
1. **#2 Lint 配置收紧**: 将 `unexpected_cfgs = "allow"` 改为 `warn`（python-bindings 已清理完毕）
2. **#5 孤立 pyo3 文件**: `validation.rs`/`management.rs` 无条件使用 pyo3，`version_token.rs`/`token.rs` 整文件 pyo3
3. **#3 重复 ResolutionResult**: 合并三个同名结构体

### 注意事项
- `version_token.rs` 和 `token.rs` 包含 pyo3 pyclass 定义，但不从 lib.rs 导出 — 可能是 rez-next-python 通过 `use rez_next_version::*` 间接使用
- `validation.rs` 和 `management.rs` 的 pyo3 清理风险高，需要 rez-next-python 同步修改
