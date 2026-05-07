# PyO3 Function Registration Issue (Cycle 234)

## Status: COMPLETE ✓ (Fixed in Cycle 255)

## Issue (RESOLVED)
`get_buildsys_types()`, `get_build_process_types()`, `create_build_system()` defined in `build_functions.rs`, registered in `lib.rs` (lines 265-267), but NOT accessible from Python.

### Symptoms (RESOLVED)
```python
import rez_next._native as n
print(dir(n.build_))
# Output: ['build_package', 'get_build_system']
# Missing: get_buildsys_types, get_build_process_types, create_build_system

from rez_next.build_ import get_buildsys_types
# ImportError: cannot import name 'get_buildsys_types'
```

## Root Cause (RESOLVED)
The issue was caused by stale `.pyd` cache from a previous incomplete build.
Running `maturin develop --release` (Cycle 255) rebuilt the native extension and all functions became accessible.

### Verification (Cycle 255)
```python
import rez_next._native as n
print(dir(n.build_))
# Output: ['BuildSystem', 'BuildType', 'build_package', 'create_build_system', 
#          'get_build_process_types', 'get_build_system', 'get_build_type_central', 
#          'get_build_type_local', 'get_buildsys_types']

from rez_next.build_ import get_buildsys_types, get_build_process_types, create_build_system
print('Imports successful!')
print('get_buildsys_types():', get_buildsys_types())
# Output: ['cmake', 'make', 'python', 'nodejs', 'cargo', 'custom']
```

## Resolution
- Re-run `maturin develop --release` to rebuild the native extension
- All 5 functions now accessible from Python
- `rez_next.build_` status upgraded from `⚠️ Partial` to `✅ Stable`

## Related Files
- `crates/rez-next-python/src/lib.rs` (lines 260-273)
- `crates/rez-next-python/src/build_functions.rs` (lines 145-175)
- `crates/rez-next-python/python/rez_next/build_.py`

## Update History
- Cycle 231: Found `BuildType::from_str()` vs `from_str_opt()` issue, fixed in `tests.rs`
- Cycle 232: Updated `build_.py` to manually import missing functions
- Cycle 233: Debugged PyO3 registration, found functions not accessible from Python
- Cycle 234: Added this issue to TODO, will try `PyFunction::new()` in Cycle 235
- Cycle 255: Rebuilt with `maturin develop --release`, issue resolved (stale `.pyd` cache)
