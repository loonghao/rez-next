# Rez-Next: High-performance Rust rewrite of Rez
# Compatible with rez API - users can `import rez_next as rez`
import _native
import os
import sys
import types
from pathlib import Path
import runpy

# Version info (matches rez.__version__ API)
__version__ = "0.3.0"
__author__ = "rez-next contributors"
__license__ = "Apache-2.0"

# Module root path (matches rez.module_root_path API)
module_root_path = os.path.dirname(os.path.abspath(__file__))

# Make all public symbols available at rez_next.*
from _native import *

# ── Dynamic submodule loader ──────────────────────────────────────────
# Replaces 54 individual bridge files with a single loading loop.
# All module implementations live in crates/rez-next-python/python/rez_next/.
# This loader auto-discovers and registers them as rez_next.<name> submodules.
_IMPL_DIR = (
    Path(__file__).resolve().parents[2]
    / "crates" / "rez-next-python" / "python" / "rez_next"
)

_pending = []

# Pass 1: Register all modules in sys.modules as empty shells so that
# cross-module imports (e.g. from rez_next.<sibling> import ...) resolve
# correctly during pass 2 execution.

# Flat .py files
for _f in sorted(_IMPL_DIR.glob("*.py")):
    _name = _f.stem
    if _name.startswith("_"):
        continue
    _module_key = f"rez_next.{_name}"
    if _module_key in sys.modules:
        continue
    _mod = types.ModuleType(_name)
    _mod.__file__ = str(_f)
    _mod.__package__ = "rez_next"
    _mod.__path__ = []
    _mod.__loader__ = None
    sys.modules[_module_key] = _mod
    _pending.append((_f, _mod, _module_key))

# Packages (directory-based modules like utils/)
for _d in sorted(_IMPL_DIR.glob("*/")):
    _name = _d.name
    if _name.startswith("_") or _name == "__pycache__":
        continue
    _module_key = f"rez_next.{_name}"
    if _module_key in sys.modules:
        continue
    _init = _d / "__init__.py"
    if _init.is_file():
        _mod = types.ModuleType(_name)
        _mod.__file__ = str(_init)
        _mod.__package__ = f"rez_next.{_name}"
        _mod.__path__ = [str(_d)]
        _mod.__loader__ = None
        sys.modules[_module_key] = _mod
        _pending.append((_init, _mod, _module_key))

# Pass 2: Execute each module. All are pre-registered now, so relative
# imports between sibling modules resolve correctly.
for _f, _mod, _module_key in _pending:
    try:
        _globals = runpy.run_path(str(_f), init_globals=_mod.__dict__)
        _mod.__dict__.update(_globals)
    except Exception:
        pass  # Module may depend on not-yet-available native code

# ── Register native submodules ────────────────────────────────────────
# Fill in any remaining _native submodules that aren't covered by
# the dynamic loader (e.g. packages_, solver_, serialise_).
for _attr_name in dir(_native):
    _attr = getattr(_native, _attr_name)
    if "module" in str(type(_attr)).lower():
        _module_key = f"rez_next.{_attr_name}"
        if _module_key not in sys.modules:
            _full_name = f"rez_next._native.{_attr_name}"
            if _full_name in sys.modules:
                sys.modules[_module_key] = sys.modules[_full_name]

# ── config singleton (matches rez.config) ──────────────────────────────
config = _native.config

# ── Subpackage imports (mirrors rez module hierarchy) ───────────────────────
# These ensure `from rez_next.X import ...` works for pure-Python modules
# that are implemented in crates/rez-next-python/python/rez_next/.
from rez_next import utils  # noqa: F401
from rez_next import build_process  # noqa: F401
from rez_next import build_plugins  # noqa: F401
from rez_next import build_system  # noqa: F401
from rez_next import bundle_context  # noqa: F401
from rez_next import bundles  # noqa: F401
from rez_next import cli  # noqa: F401
from rez_next import command  # noqa: F401
from rez_next import complete  # noqa: F401
from rez_next import data  # noqa: F401
from rez_next import depends  # noqa: F401
from rez_next import deprecations  # noqa: F401
from rez_next import diff  # noqa: F401
from rez_next import env  # noqa: F401
from rez_next import exceptions  # noqa: F401
from rez_next import forward  # noqa: F401
from rez_next import package_bind  # noqa: F401
from rez_next import package_cache  # noqa: F401
from rez_next import package_copy  # noqa: F401
from rez_next import package_help  # noqa: F401
from rez_next import package_move  # noqa: F401
from rez_next import package_py_utils  # noqa: F401
from rez_next import package_remove  # noqa: F401
from rez_next import package_repository  # noqa: F401
from rez_next import package_search  # noqa: F401
from rez_next import packages  # noqa: F401
from rez_next import pip  # noqa: F401
from rez_next import plugin_managers  # noqa: F401
from rez_next import plugins  # noqa: F401
from rez_next import release  # noqa: F401
from rez_next import release_hook  # noqa: F401
from rez_next import release_vcs  # noqa: F401
from rez_next import resolved_context  # noqa: F401
from rez_next import resolver  # noqa: F401
from rez_next import rex  # noqa: F401
from rez_next import rex_bindings  # noqa: F401
from rez_next import search  # noqa: F401
from rez_next import shells  # noqa: F401
from rez_next import solver  # noqa: F401
from rez_next import source  # noqa: F401
from rez_next import status  # noqa: F401
from rez_next import suite  # noqa: F401
from rez_next import system  # noqa: F401
from rez_next import test  # noqa: F401
from rez_next import utils  # noqa: F401
from rez_next import util  # noqa: F401
from rez_next import wrapper  # noqa: F401

# ── Step 2: Dynamic _native submodule registration ─────────────────────
# Register remaining _native submodules that don't have bridge files.
for _attr_name in dir(_native):
    _attr = getattr(_native, _attr_name)
    _type_str = str(type(_attr))
    if "module" in _type_str.lower():
        _full_name = f"rez_next._native.{_attr_name}"
        if _full_name in sys.modules:
            # Don't overwrite bridge modules that were already registered
            _module_key = f"rez_next.{_attr_name}"
            if _module_key not in sys.modules:
                sys.modules[_module_key] = sys.modules[_full_name]

# ── config singleton (matches rez.config) ──────────────────────────────
# Environment variables REZ_PACKAGES_PATH and REZ_LOCAL_PACKAGES_PATH
# are read by the native config loader at init time.
config = _native.config


