# Rez-Next Auto-Cleanup Cycle Memory
## Last Execution: Cycle #339
### Date
2026-05-24

### Environment
- Branch: auto-improve (based on origin/main, rebased in Cycle #338)
- Merge-base with origin/main: 494f6c3
- Working directory: clean after commit

### Changes Made
1. **Implemented wrapper.py** (`crates/rez-next-python/python/rez_next/wrapper.py`):
   - `Wrapper` class for suite tool execution wrappers (YAML-based)
   - Properties: `filepath`, `suite`, `tool_name`, `context_name`
   - Methods: `run()`, `print_about()`, `print_package_versions()`, `peek()`
   - Parses executable YAML wrappers with `suite_path`, `context_name`, `tool_name`, `prefix_char`
   - Aligns with `rez.wrapper.Wrapper`

2. **Implemented bundle_context.py** (`crates/rez-next-python/python/rez_next/bundle_context.py`):
   - `bundle_context()` function for creating relocatable context bundles
   - Wraps native `bundles.bundle_context()` Rust function
   - Parameters: `context`, `dest_dir`, `force`, `skip_non_relocatable`, `quiet`, `patch_libs`, `verbose`
   - Aligns with `rez.bundle_context.bundle_context()`

3. **Implemented release_vcs.py** (`crates/rez-next-python/python/rez_next/release_vcs.py`):
   - `ReleaseVCS` ABC with `__init_subclass__` auto-registration pattern
   - `ReleaseVCSError` exception subclass of `RezSystemError`
   - Factory functions: `get_release_vcs_types()`, `create_release_vcs()`
   - Abstract methods: `name()`, `is_valid_root()`, `search_parents_for_root()`, `find_vcs_root()`, etc.
   - Aligns with `rez.release_vcs.ReleaseVCS`

4. **Updated __init__.py**: Added 3 new submodule exports

### Key Design Decisions
1. **Pure Python** (not Rust): All 3 modules are pure Python facades over existing rez-next native infrastructure (Suites, Bundles), per Clean Architecture — they are abstraction/facade layers, not performance-critical algorithms.
2. **ABC + Auto-Registry**: Used `__init_subclass__` for automatic VCS subclass registration, avoiding rez's manual `release_vcs_manager.py` plugin discovery complexity.
3. **Avoided historical rez issues**: No static type registry, no circular import patterns, no hidden state mutations.

### Files Changed
7 files, +1118/-0 lines:
- `crates/rez-next-python/python/rez_next/` (4 files): `__init__.py` (modified), `wrapper.py`, `bundle_context.py`, `release_vcs.py`
- `tests/` (3 files): `test_wrapper.py`, `test_bundle_context.py`, `test_release_vcs.py`

### Test Results
- New tests: 24 passed (10 wrapper + 6 bundle_context + 8 release_vcs)
- Core tests: 72 passed (24 new + 43 config + 5 version)
- Rust tests: 201 passed (1 pre-existing cmake env failure)

### Commit
- 40805fe: "feat: add wrapper, bundle_context, release_vcs modules (rez API alignment)"
- Author: loonghao <hal.long@outlook.com>
- Pushed to origin/auto-improve (51d1e01..40805fe)

### Next Cycle
- Align remaining missing Python modules with rez API
- Consider implementing: `release_hook`, `package_serialise`, `build_system` 
- Run full CI via GitHub Actions
