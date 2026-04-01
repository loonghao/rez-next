# rez-next cleanup 执行记录

## 最新执行 (2026-04-01 20:35, 第十轮)

### 执行摘要
本轮重点：**dead_code lint 收紧**（#7 继续）和 **unused_variables lint 收紧**。

修复了 1 个编译错误（迭代 Agent 遗留），删除 ~430 行死代码（17 项），收紧 2 个 lint 规则。

#### 阶段 0：编译错误修复
**Commit** (`86484a1`): 修复 `test_framework.rs` 中缺失的 `StatePool` import + 移除 2 个 unused imports

#### 阶段 1：dead_code lint 收紧 + 死代码清理
**Commit** (`b558f21`): `dead_code` 从 `allow` → `warn`，删除 17 个死代码项 (~430 行)，19 files changed
- 删除的函数/方法: `collect_probe_versions`, `negate_bound_set`, `increment_last_token`, `save_cache_index`, `scan_directory_recursive`, `scan_package_file`, `filter_candidates`, `parse_commands_for_env_vars` (+ 3 helper methods), `parse_variants`, `view_preprocessed_package`, `generate_package_content`, `package_exists_at_destination` (x2)
- 删除的字段: `DependencyResolver.stats`, `DependencySolver.stats`, `AStarSearch.state_pool`, `ScanCacheEntry.cached_at`, `PipPackageInfo.location`/`home_page`
- 添加 `#[allow(dead_code)]` 抑制: `RequirementPatterns`, `AdvancedCacheEntry`, `CompositeHeuristic.config`, `AdaptiveHeuristic.base_heuristic`
- 移除 unused imports: `SolverStats`, `StatePool`, `JoinSet`, `Path`, `Package`, `HashMap`

#### 阶段 2：unused_variables lint 收紧
**Commit** (`c3ec157`): `unused_variables` 从 `allow` → `warn`，26 个 warnings 剩余（函数签名中的参数需手动加 `_` 前缀）

#### 阶段 3：文档更新
**Commit** (`1a5b737`): 更新 CLEANUP_TODO.md

### 基线状态
- **分支**: `auto-improve`（已推送）
- **测试**: 1290 passed, 0 failed（与清理前基线一致）
- **删除行数**: ~430 lines（本轮）
- **累计删除**: ~8530 lines across 10 cycles

### 下一轮重点
1. **#7 继续 unused_variables 修复**: 手动修复剩余 26 个 unused_variables warnings（函数签名中的参数加 `_` 前缀）
2. **#7 继续 lint 收紧**: 将 `unused_mut` 从 `allow` 改为 `warn`
3. **#7 继续 lint 收紧**: 将 `ambiguous_glob_reexports` 从 `allow` 改为 `warn`
4. **#4 dead_code helper functions**: 评估 exceptions_bindings.rs 中 5 个函数是否可删除
