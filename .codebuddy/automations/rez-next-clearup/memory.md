# rez-next cleanup 执行记录

## 最新执行 (2026-03-31 23:28, 第四轮)

### 执行摘要
本轮清理聚焦于阶段 1（过期代码清理）和阶段 6（结构性重构评估记录）。

#### 阶段 1：过期代码清理
- 删除 `solver/lib.rs` 中注释掉的 `// mod cache` / `// mod optimized_solver` 和 `// pub use cache::*` / `// pub use optimized_solver::*`（4 行）
- 删除 `optimized_solver.rs` 中注释掉的 `// use rez_next_repository::...` 导入（1 行）
- 注：其他文件（src/lib.rs, context.rs, shell.rs, 多个 Cargo.toml）的清理已由迭代 Agent 在 squash commit 中一并完成

#### 阶段 6：结构性重构评估
- 创建 `CLEANUP_TODO.md`，记录 3 个高优先级结构性问题：
  1. **python-bindings feature 清理**（119+ 处）：永远不会被启用的 `#[cfg(feature = "python-bindings")]` 代码块
  2. **Workspace lint 配置收紧**：`dead_code = "allow"` 等全局抑制项隐藏了大量潜在问题
  3. **重复 ResolutionResult 类型**：3 个同名 struct 导致 glob re-export 歧义

### 已推送 Commits
- `56a52d5` chore(cleanup): dead-code: remove commented-out cache/optimized_solver mod declarations and stale imports
- `8b80813` chore(cleanup): docs: add CLEANUP_TODO.md with structural refactoring items and python-bindings feature audit

### 基线状态
- **分支**: `auto-improve`（已推送）
- **测试**: 全部通过（0 failures），总计约 1200+ tests
- **Clippy**: 0 warnings（workspace lint 全局 allow 下）
- **删除行数**: 约 5 行代码（本轮在此分支上的直接清理）

### 下一轮重点
1. **python-bindings 大清理**：移除 10+ crates 中 119+ 处无效 cfg 门控代码
2. **lint 配置收紧**：将 `dead_code` 和 `unused_imports` 从 `allow` 改为 `warn`，修复暴露的问题
3. **ResolutionResult 合并**：将 3 个同名类型合并为一个权威定义

### 注意事项
- 迭代 Agent 同时在 `auto-improve` 分支活跃，需注意合并冲突
- 存在 `auto-improve-squashed` 分支（squash 后的版本），清理工作应在 `auto-improve` 上进行
