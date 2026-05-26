"""
Rez-Next: High-performance Rust rewrite of Rez.

This module provides a drop-in replacement for Rez.
"""

import os  # noqa: F401
import warnings  # noqa: F401
import sys

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

from . import package_help  # noqa: F401
from . import package_py_utils  # noqa: F401
from . import package_remove  # noqa: F401
from . import package_repository  # noqa: F401
from . import package_search  # noqa: F401
from . import package_serialise  # noqa: F401  — module (rez.package_serialise API: dump_package_data)
from . import package_test  # noqa: F401  — module (rez.package_test API: PackageTestRunner, PackageTestResults)
from . import plugin_managers  # noqa: F401
from . import release_hook  # noqa: F401
from . import release_vcs  # noqa: F401
from . import resolver  # noqa: F401
from . import rex_bindings  # noqa: F401
from . import solver  # noqa: F401
from . import status  # noqa: F401  — module (rez.status API: from rez.status import status)
from . import shells  # noqa: F401  — module (rez.shells API: create_shell, get_shell_types, get_shell_class)
from . import system  # noqa: F401  — module (rez.system API: from rez.system import system)
from . import serialise  # noqa: F401
from . import rezconfig  # noqa: F401
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

# ── Logging Initialization ───────────────────────────────────────────────
# Mirrors upstream rez._init_logging()
def _init_logging() -> None:
    """Initialize rez-next logging.

    If ``REZ_LOGGING_CONF`` is set, uses that logging config file.
    Otherwise, configures a default StreamHandler on the ``rez`` logger.
    """
    _logging_conf = os.getenv("REZ_LOGGING_CONF")
    if _logging_conf:
        import logging.config
        logging.config.fileConfig(_logging_conf, disable_existing_loggers=False)
        return

    import logging
    _logger = logging.getLogger("rez")
    _logger.propagate = False
    _logger.setLevel(logging.DEBUG)
    if not _logger.handlers:
        _handler = logging.StreamHandler()
        _handler.setFormatter(logging.Formatter(
            fmt="%(asctime)s %(levelname)-8s %(message)s",
            datefmt="%X",
        ))
        _logger.addHandler(_handler)


_init_logging()

# ── SIGUSR1 / Signal Handler ─────────────────────────────────────────────
# Mirrors upstream rez SIGUSR1 handler for debug / stack trace dumping.
_action = os.getenv("REZ_SIGUSR1_ACTION")
if _action:
    try:
        import signal
        import traceback

        if _action == "print_stack":
            def _callback(sig, frame):  # type: ignore
                txt = "".join(traceback.format_stack(frame))
                print()
                print(txt)
        else:
            _callback = None  # type: ignore

        if _callback:
            signal.signal(signal.SIGUSR1, _callback)
    except (ValueError, OSError, AttributeError):
        pass  # SIGUSR1 not available on this platform (e.g., Windows)

# Module-level variable for API compat (matches rez.action)
action: str | None = _action

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

# ── Deprecation Warnings Filter ──────────────────────────────────────────
# Mirrors upstream rez behavior: log all rez deprecation warnings by default,
# bypassing user-defined warning filters.
if os.getenv("REZ_LOG_DEPRECATION_WARNINGS"):
    warnings.filterwarnings("default", category=deprecations.RezDeprecationWarning)
