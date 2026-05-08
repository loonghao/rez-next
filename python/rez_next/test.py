"""rez_next.test - Package test functionality.

Exposes rez.package_test compatible API.
"""

import rez_next._native  # ensure extension module is initialized  # noqa: F401
from rez_next._native.test import (  # noqa: F401,F403
    PackageTestRunner,
    PackageTestResults,
    SUCCESS,
    FAILED,
    SKIPPED,
    ERROR,
)
