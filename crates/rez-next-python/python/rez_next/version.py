"""Rez-compatible version module.

Provides version-related classes and utilities aligning with ``rez.version`` API.

This module wraps the native version module and adds Rez-API-compatible
exception classes and utility functions.
"""
from __future__ import annotations

import rez_next._native  # noqa: F401
from rez_next._native.vendor.version import *  # noqa: F401,F403


class ParseException(ValueError):
    """Raised when a version string cannot be parsed.

    Rez API: ``rez.version.ParseException``
    """
    pass


class VersionError(Exception):
    """Raised when a version-related operation fails.

    Rez API: ``rez.version.VersionError``
    """
    pass


def reverse_sort_key(version):
    """Return a key suitable for reverse (descending) sorting of versions.

    Rez API: ``rez.version.reverse_sort_key()``
    """
    try:
        if hasattr(version, "_cmp_key"):
            return tuple(
                -x if isinstance(x, (int, float)) else x
                for x in version._cmp_key
            )
    except Exception:
        pass

    from rez_next._native.vendor.version import Version as V

    if isinstance(version, V):
        parts = str(version).split(".")
        return tuple(-int(p) if p.isdigit() else p for p in parts)
    return version
