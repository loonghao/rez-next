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

# ═══════════════════════════════════════════════════════════════════════
# IMPORT ORDER IS IMPORTANT:
#   1. Pure-Python bridge files (system.py, status.py) MUST be imported
#      BEFORE the dynamic _native submodule registration below, so the
#      bridge modules take precedence in sys.modules.
#   2. The dynamic registration then fills in remaining _native submodules
#      that don't have bridge files.
# ═══════════════════════════════════════════════════════════════════════

# ── Step 1: Pure-Python bridge submodules (loaded before native overrides) ─
# These bridge files proxy via runpy to implementations in
# crates/rez-next-python/python/rez_next/.
from rez_next import bind  # noqa: F401
from rez_next import build_  # noqa: F401
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


