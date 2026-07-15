"""
Rez-Next: High-performance Rust rewrite of Rez.

This module exposes the supported top-level Rez-compatible workflows.
"""

# ruff: noqa: I001

from __future__ import annotations

import os  # noqa: F401
import sys
import warnings  # noqa: F401
from importlib.metadata import PackageNotFoundError
from importlib.metadata import version as distribution_version

try:
    _distribution_version = distribution_version("rez-next")
except PackageNotFoundError:
    _distribution_version = "0+unknown"

# The wheel is a native extension package. A missing or incompatible extension
# is an installation error and must fail at import time instead of exposing
# partial placeholder objects.
from . import _native
from ._native import *  # noqa: F401,F403

# Import submodules
# Ensure native submodules used by bridge modules are registered in sys.modules
# so that "from rez_next.<submodule> import ..." works at runtime.
import sys as _sys
import types as _types

from . import (
    bind,  # noqa: F401
    build_process,  # noqa: F401
    build_system,  # noqa: F401
    bundle_context,  # noqa: F401
    command,  # noqa: F401
    complete,  # noqa: F401
    deprecations,  # noqa: F401
    package_cache,  # noqa: F401
    package_help,  # noqa: F401
    package_py_utils,  # noqa: F401
    package_remove,  # noqa: F401
    package_repository,  # noqa: F401
    package_search,  # noqa: F401
    package_serialise,  # noqa: F401  — module (rez.package_serialise API: dump_package_data)
    package_test,  # noqa: F401  — module (rez.package_test API: PackageTestRunner, PackageTestResults)
    plugin_managers,  # noqa: F401
    release_hook,  # noqa: F401
    release_vcs,  # noqa: F401
    resolver,  # noqa: F401
    rex_bindings,  # noqa: F401
    rezconfig,  # noqa: F401
    serialise,  # noqa: F401
    shells,  # noqa: F401  — module (rez.shells API: create_shell, get_shell_types, get_shell_class)
    solver,  # noqa: F401
    status,  # noqa: F401  — module (rez.status API: from rez.status import status)
    system,  # noqa: F401  — module (rez.system API: from rez.system import system)
    wrapper,  # noqa: F401
)
from .config import Config  # noqa: F401
from .exceptions import *  # noqa: F401,F403

# Package maker — programmatic package creation
from .package_maker import (  # noqa: F401
    PackageMaker,
    create_package,
    install_package,
    make_package,
)

for _name in ("packages_", "package_search", "search", "solver_", "serialise_", "bind"):
    _attr = getattr(_native, _name, None)
    if _attr is not None and isinstance(_attr, _types.ModuleType):
        _full = "rez_next." + _name
        if _full not in _sys.modules:
            _sys.modules[_full] = _attr

# 'packages' needs special handling: register as a Python bridge module
# that re-exports classes from _native.packages and functions from
# _native.packages_. Otherwise the native _native.packages submodule
# shadows packages.py entirely.
_packages_module = _types.ModuleType("rez_next.packages")
_packages_module.__package__ = "rez_next"
_packages_module.__path__ = []
_packages_module.__file__ = __file__
# Native classes
_attr = getattr(_native, "packages", None)
if _attr is not None:
    for _cls_name in ("Package", "PackageFamily", "PackageRequirement", "PackageFormat"):
        setattr(_packages_module, _cls_name, getattr(_attr, _cls_name))
# Functions from packages_
_attr = getattr(_native, "packages_", None)
if _attr is not None:
    for _func_name in (n for n in dir(_attr) if not n.startswith("_")):
        setattr(_packages_module, _func_name, getattr(_attr, _func_name))
# Build __all__ from all public names
_packages_module.__all__ = [n for n in dir(_packages_module) if not n.startswith("_")]
# Register
_sys.modules["rez_next.packages"] = _packages_module
setattr(sys.modules["rez_next"], "packages", _packages_module)
del _cls_name, _func_name, _packages_module

__version__: str = getattr(_native, "__version__", _distribution_version)
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
        _handler.setFormatter(
            logging.Formatter(
                fmt="%(asctime)s %(levelname)-8s %(message)s",
                datefmt="%X",
            )
        )
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

# Import the Python bridge explicitly: the native module also exports an
# attribute named ``complete`` and therefore cannot be used for this alias.
from .complete import get_completion_script as get_completion_script  # noqa: E402


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
        "  node [shape=box, style=filled, fillcolor=lightblue];",
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


_package_class = getattr(_native, "Package", None)
_resolved_context_class = getattr(_native, "ResolvedContext", None)
if _package_class is not None:
    _package_class.__getitem__ = _package_getitem
if _resolved_context_class is not None:
    _resolved_context_class.get = _context_get
    _resolved_context_class.to_dot = _context_to_dot

# ── Deprecation Warnings Filter ──────────────────────────────────────────
# Mirrors upstream rez behavior: log all rez deprecation warnings by default,
# bypassing user-defined warning filters.
if os.getenv("REZ_LOG_DEPRECATION_WARNINGS"):
    warnings.filterwarnings("default", category=deprecations.RezDeprecationWarning)
