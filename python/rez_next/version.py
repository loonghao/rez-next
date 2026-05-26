"""Bridge to rez_next version module.

Provides version-related classes and utilities aligning with ``rez.version`` API:

- ``Version`` — version string parsing and comparison
- ``VersionRange`` — version range parsing and matching
- ``Requirement`` — package requirement (name + version range)
- ``RequirementList`` — list of requirements
- ``VersionedObject`` — versioned object base class
- ``VersionToken``, ``NumericToken``, ``AlphanumericVersionToken`` — token types
- ``ParseException``, ``VersionError`` — version-related exceptions
- ``reverse_sort_key()`` — reverse sort key for descending version ordering
"""
from __future__ import annotations

from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "vendor"
    / "version.py"
)

# Export version classes and utilities
_version_globals = runpy.run_path(str(_IMPL))
globals().update(_version_globals)

# Additional re-exports for convenience
try:
    from rez_next.vendor.version import (  # type: ignore[import-untyped]  # noqa: F401,F811
        Version as Version,
        VersionRange as VersionRange,
    )
except ImportError:
    pass

# Version-related exception classes
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
            return tuple(-x if isinstance(x, (int, float)) else x for x in version._cmp_key)
    except Exception:
        pass

    from rez_next.vendor.version import Version as V

    if isinstance(version, V):
        # Use negative of the version's components for reverse order
        parts = str(version).split(".")
        return tuple(-int(p) if p.isdigit() else p for p in parts)
    return version
