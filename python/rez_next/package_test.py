"""Bridge to rez_next package_test module.

Aligns with rez.package_test API:
- ``PackageTestRunner`` — runs tests defined in a package's ``tests`` attribute
- ``PackageTestResults`` — stores and summarises test results
- ``SUCCESS`` / ``FAILED`` / ``SKIPPED`` / ``ERROR`` — result status constants

See https://github.com/AcademySoftwareFoundation/rez/blob/main/src/rez/package_test.py
"""

from rez_next.test import (  # noqa: F401
    PackageTestRunner,
    PackageTestResults,
    SUCCESS,
    FAILED,
    SKIPPED,
    ERROR,
)

__all__ = [
    "PackageTestRunner",
    "PackageTestResults",
    "SUCCESS",
    "FAILED",
    "SKIPPED",
    "ERROR",
]
