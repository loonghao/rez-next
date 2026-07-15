"""Rez-compatible packages module (Python bridge to native bindings).

Aligns with ``rez.packages`` API:

- ``Package`` — package object (native)
- ``PackageFamily`` — package family (native)
- ``PackageRequirement`` — package requirement (native)
- ``PackageFormat`` — package file format enum (native)
- All functions from ``rez.packages_`` (native bridge)
"""

import rez_next._native  # noqa: F401

# Native classes
from rez_next._native.packages import (  # noqa: F401
    Package,
    PackageFamily,
    PackageFormat,
    PackageRequirement,
)

# Native functions (re-export)
from rez_next._native.packages_ import *  # noqa: F401,F403
