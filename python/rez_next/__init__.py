# Rez-Next: High-performance Rust rewrite of Rez
# Compatible with rez API - users can `import rez_next as rez`
#
# This module mirrors the public API surface of rez.__init__.
# See https://github.com/AcademySoftwareFoundation/rez/blob/main/src/rez/__init__.py
#
# Key alignment areas:
# - Package management (copy, move, remove)
# - Configuration system (with env var / file overrides)
# - Utility submodules (logging, filesystem, platform)

import _native
import os
import sys

# Version info (matches rez.__version__ API)
__version__ = "0.3.0"

# Author info (matches rez.__author__ API)
__author__ = "rez-next contributors"

# License info (matches rez.__license__ API)
__license__ = "Apache-2.0"

# Module root path (matches rez.module_root_path API)
module_root_path = os.path.dirname(os.path.abspath(__file__))

# Make all public symbols available at rez_next.*
from _native import *

# ── Submodule registration ──────────────────────────────────────────────
# Register submodules in sys.modules so "from rez_next.X import ..." works.
#
# This mirrors rez's module hierarchy:
#   rez.config, rez.packages_, rez.resolved_context, rez.solver_,
#   rez.package_filter, rez.package_repository, rez.package_resources,
#   rez.package_copy, rez.package_remove, rez.package_order, etc.

# Dynamically register all _native submodules
for _attr_name in dir(_native):
    _attr = getattr(_native, _attr_name)
    _type_str = str(type(_attr))
    if "module" in _type_str.lower():
        _full_name = f"rez_next._native.{_attr_name}"
        if _full_name in sys.modules:
            sys.modules[f"rez_next.{_attr_name}"] = sys.modules[_full_name]

# ── config singleton (matches rez.config) ──────────────────────────────
# Environment variables REZ_PACKAGES_PATH and REZ_LOCAL_PACKAGES_PATH
# are read by the native config loader at init time.
config = _native.config

# ── Subpackage imports (mirrors rez module hierarchy) ───────────────────────
# These ensure `from rez_next.X import ...` works for pure-Python modules
# that are implemented in crates/rez-next-python/python/rez_next/.
from rez_next import utils  # noqa: F401
from rez_next import build_process  # noqa: F401
from rez_next import bundle_context  # noqa: F401
from rez_next import complete  # noqa: F401
from rez_next import package_help  # noqa: F401
from rez_next import release_vcs  # noqa: F401
from rez_next import wrapper  # noqa: F401

# API naming alignment: rez uses `rez.shells`, we provide `rez_next.shells`
from rez_next import shells  # noqa: F401
