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
    from ._native import *  # noqa: F401,F403
except ImportError:
    try:
        from _native import *  # noqa: F401,F403
    except ImportError:
        pass

# Import submodules
from . import complete  # noqa: F401
from . import deprecations  # noqa: F401
from . import package_help  # noqa: F401
from . import bundle_context  # noqa: F401
from . import wrapper  # noqa: F401
from . import release_vcs  # noqa: F401
from . import build_process  # noqa: F401
from .exceptions import *  # noqa: F401,F403
from .config import Config  # noqa: F401

# Package maker — programmatic package creation
from .package_maker import (  # noqa: F401
    PackageMaker,
    create_package,
    install_package,
    make_package,
)

__version__: str = getattr(_native, "__version__", "0.3.0")
__author__: str = getattr(_native, "__author__", "rez-next contributors")
__license__: str = "Apache-2.0"

# Module root path (matches rez.module_root_path API)
module_root_path: str = os.path.dirname(os.path.abspath(__file__))

# Emulate rez.action variable (read from env, used for signal handling)
action = os.getenv("REZ_SIGUSR1_ACTION")

# Top-level singletons — compatible with `from rez import config` and `from rez import system`
config = Config()
config.Config = Config
system = getattr(_native, "System", lambda: None)()

# Aliases for API compatibility with original rez
resolve = getattr(_native, "resolve_packages", None)
create_context = getattr(_native, "ResolvedContext", None)

# get_completion_script is available via complete.get_completion_script
# For top-level access, we import it explicitly
from .complete import get_completion_script  # noqa: F401


def _package_getitem(self, key):
    if key == "version":
        return getattr(self, "version_str", None)
    return getattr(self, key)


def _context_get(self, key, default=None):
    if key == "success":
        return getattr(self, "success", False)
    if key == "packages":
        return getattr(self, "resolved_packages", [])
    return getattr(self, key, default)


def _pkg_node(pkg):
    version = getattr(pkg, "version_str", None)
    if version is None:
        version = getattr(pkg, "version", None)
    return f"{getattr(pkg, 'name', pkg)}-{version}" if version else str(getattr(pkg, "name", pkg))


def _req_name(requirement):
    requirement = str(requirement)
    if requirement.startswith("!"):
        return None
    for sep in ("<", ">", "=", " "):
        requirement = requirement.split(sep, 1)[0]
    return requirement.split("-", 1)[0]


def _context_to_dot(self):
    packages = list(getattr(self, "resolved_packages", []) or [])
    lines = [
        "digraph resolved_context {",
        "  rankdir=LR;",
        '  node [shape=box, style=filled, fillcolor=lightblue];',
    ]
    by_name = {getattr(pkg, "name", ""): pkg for pkg in packages}
    for pkg in packages:
        lines.append(f'  "{_pkg_node(pkg)}";')
    for pkg in packages:
        source = _pkg_node(pkg)
        for req in getattr(pkg, "requires", []) or []:
            name = _req_name(req)
            if name and name in by_name:
                lines.append(f'  "{source}" -> "{_pkg_node(by_name[name])}";')
    lines.append("}")
    return "\n".join(lines)


try:
    Package.__getitem__ = _package_getitem  # type: ignore[name-defined]
    ResolvedContext.get = _context_get  # type: ignore[name-defined]
    ResolvedContext.to_dot = _context_to_dot  # type: ignore[name-defined]
except Exception:
    pass
