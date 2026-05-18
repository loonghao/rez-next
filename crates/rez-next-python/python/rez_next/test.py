"""rez_next.test - package test compatibility API."""

import rez_next._native  # noqa: F401
from rez_next._native.test import (  # noqa: F401
    ERROR,
    FAILED,
    SKIPPED,
    SUCCESS,
    PackageTestResults,
    PackageTestRunner,
)
