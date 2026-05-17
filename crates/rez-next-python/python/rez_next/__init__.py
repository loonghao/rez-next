"""
Rez-Next: High-performance Rust rewrite of Rez.

This module provides a drop-in replacement for Rez.
"""

import os  # noqa: F401
import sys  # noqa: F401
import warnings  # noqa: F401

# Import _native module (try multiple methods)
try:
    from . import _native
except ImportError:
    try:
        import _native
    except ImportError:
        # If _native is not available, define stubs
        class _native:
            __version__ = "0.3.0"
            __author__ = "rez-next contributors"

            class Config:
                pass

            class System:
                pass

            @staticmethod
            def resolve_packages(*args, **kwargs):
                raise NotImplementedError("_native module not available")

            @staticmethod
            def ResolvedContext(*args, **kwargs):
                raise NotImplementedError("_native module not available")

# Import all attributes from _native
try:
    from _native import *  # noqa: F401,F403
except ImportError:
    pass

# Import submodules
from . import complete  # noqa: F401
from . import deprecations  # noqa: F401
from . import package_help  # noqa: F401

__version__: str = getattr(_native, "__version__", "0.3.0")
__author__: str = getattr(_native, "__author__", "rez-next contributors")
__license__: str = "Apache-2.0"

# Module root path (matches rez.module_root_path API)
module_root_path: str = os.path.dirname(os.path.abspath(__file__))

# Emulate rez.action variable (read from env, used for signal handling)
action = os.getenv("REZ_SIGUSR1_ACTION")

# Top-level singletons — compatible with `from rez import config` and `from rez import system`
config = getattr(_native, "Config", lambda: None)()
system = getattr(_native, "System", lambda: None)()

# Aliases for API compatibility with original rez
resolve = getattr(_native, "resolve_packages", None)
create_context = getattr(_native, "ResolvedContext", None)

# get_completion_script is available via complete.get_completion_script
# For top-level access, we import it explicitly
from .complete import get_completion_script  # noqa: F401
