"""Package resource hierarchy for rez-next.

This module provides the same API as rez.package_resources for drop-in compatibility.
It defines the resource hierarchy used by package repositories to represent
package families, individual packages, and their variants.

API Reference: rez.package_resources
  - PackageFamilyResource: a named group of package versions (e.g., "python")
  - PackageResource: a specific version of a package (e.g., "python-3.9")
  - VariantResource: a specific variant of a package version
"""

import rez_next._native  # noqa: F401
from rez_next._native.package_resources import (  # noqa: F401
    PackageFamilyResource,
    PackageResource,
    VariantResource,
)

# Re-export for convenience
__all__ = [
    "PackageFamilyResource",
    "PackageResource",
    "VariantResource",
]
