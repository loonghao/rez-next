"""Rez-compatible resolved_context module.

Aligns with ``rez.resolved_context`` API:

- ``ResolvedContext`` — resolved dependency environment (native)
- ``RezToolsVisibility`` — enum controlling rez CLI visibility in resolved env
- ``SuiteVisibility`` — enum controlling suite visibility in resolved env
- ``PatchLock`` — enum for patch-level version locking
- ``get_lock_request()`` — generate a locked package request from version + lock
"""
from __future__ import annotations

from enum import Enum

import rez_next._native  # ensure extension module is initialized  # noqa: F401
from rez_next._native.resolved_context import *  # noqa: F401,F403


# ── Enums (pure Python, no native equivalent) ─────────────────────────


class RezToolsVisibility(Enum):
    """Determines if/how rez CLI tools are added back to PATH within a
    resolved environment.

    Rez API: ``rez.resolved_context.RezToolsVisibility``
    """
    #: Don't expose rez in resolved env
    never = 0
    #: Append to PATH in resolved env
    append = 1
    #: Prepend to PATH in resolved env
    prepend = 2


class SuiteVisibility(Enum):
    """Defines what suites on $PATH stay visible when a new rez environment is
    resolved.

    Rez API: ``rez.resolved_context.SuiteVisibility``
    """
    #: Don't attempt to keep any suites visible in a new env
    never = 0
    #: Keep suites visible in any new env
    always = 1
    #: Keep only the parent suite of a tool visible
    parent = 2
    #: Keep all suites visible and the parent takes precedence
    parent_priority = 3


class PatchLock(Enum):
    """Enum to represent the 'lock type' used when patching context objects.

    Rez API: ``rez.resolved_context.PatchLock``
    """
    no_lock = ("No locking", -1)
    lock_2 = ("Minor version updates only (X.*)", 1)
    lock_3 = ("Patch version updates only (X.X.*)", 2)
    lock_4 = ("Build version updates only (X.X.X.*)", 3)
    lock = ("Exact version", -1)

    def __init__(self, description: str, rank: int) -> None:
        self.description = description
        self.rank = rank


# ── Standalone functions ─────────────────────────────────────────────


def get_lock_request(name: str, version, patch_lock: PatchLock,
                     weak: bool = True):
    """Given a package name, version and patch lock type, return the
    equivalent package request.

    For example, for ``name='foo'``, ``version='1.2.1'`` and
    ``patch_lock=PatchLock.lock_3``, the equivalent request is
    ``'~foo-1.2'``, restricting updates to patch-or-lower changes only.

    Args:
        name: Package name.
        version: Package version (rez Version object or string).
        patch_lock: Lock type to apply.
        weak: If True (default), prefix the request with ``~`` (weak).

    Returns:
        A package request string, or None if there is no equivalent request
        (e.g. when ``patch_lock == PatchLock.no_lock``).
    """
    from rez_next._native.vendor.version import Version

    if isinstance(version, str):
        version = Version(version)

    ch = '~' if weak else ''
    if patch_lock == PatchLock.lock:
        return "%s%s==%s" % (ch, name, str(version))
    elif patch_lock == PatchLock.no_lock or not version:
        return None

    version_ = version.trim(patch_lock.rank)
    s = "%s%s-%s" % (ch, name, str(version_))
    return s
