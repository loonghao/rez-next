# rez-next cleanup 执行记录

## 最新执行 (2026-04-04 02:28, 第二十轮)

### 执行摘要
- 切换到 `auto-improve`，先做基线校验；发现 `crates/rez-next-python/src/lib.rs` 为 UTF-16 编码，先恢复为 UTF-8 以解除工具链阻塞
- 完成 3 个 cleanup 提交并已推送远端：`42ad977`（lint/编码与重复报错清理）、`0aadd95`（过期文档刷新）、`45ea9e2`（依赖治理记录，含 `chore(cleanup): done` 标记）
- 清理内容以低风险一致性治理为主：移除 `rez view --current` 重复错误输出、删除 2 处过时 `python-bindings` 注释、清理 benchmark 注释残留、刷新 3 份过期文档、在 `CLEANUP_TODO.md` 记录 `pyo3` 版本漂移

### 验证结果
- **分支**: `auto-improve`（已推送至 `45ea9e2`）
- **测试**: `cargo test --workspace` 全量通过；本地可枚举测试用例约 **1432**
- **Lint**: `cargo clippy --workspace --all-targets --all-features --exclude rez-next-python -- -A warnings -D clippy::correctness` 通过
- **Docs**: `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --document-private-items` 通过
- **覆盖率**: 本地未配置覆盖率命令，本轮未采集覆盖率基线

### 下一轮重点
1. 评估 `pyo3` workspace `0.28` 与 `rez-next-python` 直接依赖 `0.25` 的统一策略（需 wheel/build 验证）
2. 继续跟进 3 处库代码 `eprintln!` → `tracing` 的依赖化改造
3. 评估 `fix-ci-security-audit/` 是否应移出仓库源码树



## 最新执行 (2026-04-02 20:11, 第十九轮)

### 执行摘要
本轮重点：**迭代 Agent 新增代码审查** + **bug 修复** + **clippy 清理** + **dead code 审计**。

#### Commit 1 (`293852f`): Bug 修复 — build/env --help exit code
- `rez-next.rs`: `handle_grouped_command` 中 clap `try_parse_from` 返回 `Err` 处理不当
- `--help`/`--version` 属于 clap 的 `DisplayHelp`/`DisplayVersion` 错误类型，不应 exit(1)
- 使用 `e.use_stderr()` 区分：help/version → exit(0)，真正错误 → exit(1)

#### Commit 2 (`c25a295`): Lint 和 dead code 清理
- `pkg_cache.rs`: 将 `#[cfg(test)] mod tests` 从文件中间移到文件末尾，修复 clippy `items-after-test-module` 警告
- `requirement.rs`: 删除 `RequirementPatterns` 中 3 个未使用的 regex 字段（`range`, `platform_condition`, `env_condition`），移除 `#[allow(dead_code)]` 注解
- `python_ast_parser.rs`: 删除调试用 `eprintln!("Unhandled statement type: {:?}", stmt)` — 库代码不应直接打印到 stderr
- `cli_e2e_tests.rs`: 删除从未调用的 `rez_fail` 函数及其 `#[allow(dead_code)]`
- `real_repo_integration.rs`: 移除不必要的 `#[allow(unused_imports)]`（`PackageRepository` trait 实际被使用）

#### Commit 3 (`d3f7c2e`): 文档更新
- 更新 CLEANUP_TODO.md：TODO 审计 2→1（daemon TODO 由迭代 Agent 实现），新增 #12 (exit code bug — COMPLETE), #13 (dead regex fields — COMPLETE)

### 基线状态
- **分支**: `auto-improve`（已推送 827b05d）
- **测试**: 1371 passed, 0 failed（基线 1341 → 1371，增加 30 来自迭代 Agent 新测试）
- **Clippy warnings**: 0 (--all-targets)
- **TODO 数量**: 1（`view.rs` — context package viewing）+ 2 benches（非阻塞性能 validation stubs）
- **`#[allow(dead_code)]`**: 5 处（test_astar_standalone.rs: 2 辅助方法, high_performance_scanner.rs: 3 cache metadata 字段, intelligent_cache_demo.rs: 1 示例函数）
- **累计删除**: ~9520+ lines across 19 cycles

### 下一轮重点
1. **eprintln 改 tracing**: 库代码中 3 处 `eprintln!` 应替换为 `tracing` 日志框架（`intelligent_manager.rs`, `scanner.rs`, `filesystem.rs`）
2. **fix-ci-security-audit/ 目录**: Harbor 任务文件在 git 中，不属于项目源码 — 需用户确认是否删除
3. **结构性评估**: 39+ 个文件 >500 行，9 个 >1000 行
4. **`pprof` feature gate**: `--all-features` 在 Windows 编译失败（pprof 仅 Linux）
5. **Python 测试中 `env::set_var` 线程安全问题**: Rust 1.66+ 中在多线程环境不安全
