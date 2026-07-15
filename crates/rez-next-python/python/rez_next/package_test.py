"""
Rez-compatible package_test module.

Aligns with ``rez.package_test`` API:
- ``PackageTestRunner`` — runs tests defined in a package's ``tests`` attribute
- ``PackageTestResults`` — stores and summarises test results
- ``SUCCESS`` / ``FAILED`` / ``SKIPPED`` / ``ERROR`` — result status constants

See https://github.com/AcademySoftwareFoundation/rez/blob/main/src/rez/package_test.py
"""

from __future__ import annotations

# Re-export from the native test module
from ._native.test import (  # type: ignore[import]
    ERROR,
    FAILED,
    SKIPPED,
    SUCCESS,
    PackageTestResults,
    PackageTestRunner,
)

__all__ = [
    "PackageTestRunner",
    "PackageTestResults",
    "SUCCESS",
    "FAILED",
    "SKIPPED",
    "ERROR",
]
