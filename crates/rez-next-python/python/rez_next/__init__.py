"""
Rez-Next: High-performance Rust rewrite of Rez.

This module provides a drop-in replacement for Rez.
"""

import os  # noqa: F401

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
from . import bind  # noqa: F401
from . import build_process  # noqa: F401
from . import build_system  # noqa: F401
from . import bundle_context  # noqa: F401
from . import command  # noqa: F401
from . import complete  # noqa: F401
from . import deprecations  # noqa: F401
from . import package_cache  # noqa: F401
from . import package_bind  # noqa: F401  — module (rez.package_bind API: bind_package, get_bind_modules, find_bind_module)
from . import package_help  # noqa: F401
from . import package_py_utils  # noqa: F401
from . import package_remove  # noqa: F401
from . import package_repository  # noqa: F401
from . import plugin_managers  # noqa: F401
from . import release_hook  # noqa: F401
from . import release_vcs  # noqa: F401
from . import resolver  # noqa: F401
from . import rex_bindings  # noqa: F401
from . import solver  # noqa: F401
from . import status  # noqa: F401  — module (rez.status API: from rez.status import status)
from . import system  # noqa: F401  — module (rez.system API: from rez.system import system)
from . import wrapper  # noqa: F401
from .exceptions import *  # noqa: F401,F403
from .config import Config  # noqa: F401

# Package maker — programmatic package creation
from .package_maker import (  # noqa: F401
    PackageMaker,
    create_package,
    install_package,
    make_package,
)

# Ensure native submodules used by bridge modules are registered in sys.modules
# so that "from rez_next.<submodule> import ..." works at runtime.
import sys as _sys
import types as _types
for _name in ('packages', 'packages_', 'package_search', 'search', 'solver_', 'serialise_', 'bind'):
    _attr = getattr(_native, _name, None)
    if _attr is not None and isinstance(_attr, _types.ModuleType):
        _full = 'rez_next.' + _name
        if _full not in _sys.modules:
            _sys.modules[_full] = _attr

__version__: str = getattr(_native, "__version__", "0.3.0")
__author__: str = getattr(_native, "__author__", "rez-next contributors")
__license__: str = "Apache-2.0"

# Module root path (matches rez.module_root_path API)
module_root_path: str = os.path.dirname(os.path.abspath(__file__))

# Emulate rez.action variable (read from env, used for signal handling)
action = os.getenv("REZ_SIGUSR1_ACTION")

# Top-level singletons — compatible with `from rez import config`
config = Config()
config.Config = Config

# Aliases for API compatibility with original rez
resolve = getattr(_native, "resolve_packages", None)
create_context = getattr(_native, "ResolvedContext", None)

# get_completion_script is available via complete.get_completion_script
# For top-level access, we import it explicitly
from .complete import get_completion_script  # noqa: F401


def _package_getitem(self, key):
    if key == "version":
        return getattr(self, "version_str", None)
    if not hasattr(self, key):
        raise KeyError(key)
    return getattr(self, key)


_context_get_sentinel = object()


def _context_get(self, key, default=_context_get_sentinel):
    if key == "success":
        return getattr(self, "success", False)
    if key == "packages":
        return getattr(self, "resolved_packages", [])
    if hasattr(self, key):
        return getattr(self, key)
    if default is not _context_get_sentinel:
        return default
    raise KeyError(key)


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
except (NameError, AttributeError):
    pass  # Core classes not yet available — expected in early imports
