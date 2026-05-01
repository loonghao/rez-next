# PyO3 Function Registration Issue (Cycle 234)

## Status: OPEN

## Issue
`get_buildsys_types()`, `get_build_process_types()`, `create_build_system()` defined in `build_functions.rs`, registered in `lib.rs` (lines 265-267), but NOT accessible from Python.

### Symptoms
```python
import rez_next._native as n
print(dir(n.build_))
# Output: ['build_package', 'get_build_system']
# Missing: get_buildsys_types, get_build_process_types, create_build_system

from rez_next.build_ import get_buildsys_types
# ImportError: cannot import name 'get_buildsys_types'
```

## Root Cause (Unknown)
- `wrap_pyfunction!` macro might be failing silently (but `?` operator should propagate errors)
- Functions are `pub` and imported at `lib.rs` line 47
- Registration code in `lib.rs` lines 265-267:
  ```rust
  build_mod.add_function(wrap_pyfunction!(get_buildsys_types, &build_mod)?)?;
  build_mod.add_function(wrap_pyfunction!(get_build_process_types, &build_mod)?)?;
  build_mod.add_function(wrap_pyfunction!(create_build_system, &build_mod)?)?;
  ```

## Attempted Fixes
1. **Used full path in `wrap_pyfunction!`**: `wrap_pyfunction!(build_functions::get_buildsys_types, &build_mod)?` → still not accessible
2. **Updated `build_.py`**: `from rez_next._native.build_ import get_buildsys_types` → fails (function not in module)
3. **Tried `PyFunction::new(py, func)?`**: haven't tried yet (Cycle 235 task)

## Next Steps
1. Debug why `wrap_pyfunction!` not adding functions to `rez_next._native.build_` module
2. Try `PyFunction::new(py, build_functions::get_buildsys_types)?` to manually create function objects
3. If that fails, check if `get_buildsys_types` is actually `pub` and in scope at registration point
4. If still fails, consider upgrading PyO3 to newer version (0.28.3 might have bug)

## Priority
Medium (blocks `build_` from being `✅ Stable`; currently `⚠️ Partial` in `docs/python-integration.md`)

## Related Files
- `crates/rez-next-python/src/lib.rs` (lines 260-273)
- `crates/rez-next-python/src/build_functions.rs` (lines 145-175)
- `crates/rez-next-python/python/rez_next/build_.py`

## Cycle History
- Cycle 231: Found `BuildType::from_str()` vs `from_str_opt()` issue, fixed in `tests.rs`
- Cycle 232: Updated `build_.py` to manually import missing functions
- Cycle 233: Debugged PyO3 registration, found functions not accessible from Python
- Cycle 234: Added this issue to TODO, will try `PyFunction::new()` in Cycle 235
