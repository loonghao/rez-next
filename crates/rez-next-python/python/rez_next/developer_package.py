"""Developer package support for rez-next.

This module provides the same API as rez.developer_package for drop-in compatibility.
It represents a package that exists as source code in a developer's working directory,
before being built and released to a package repository.

API Reference: rez.developer_package
  - DeveloperPackage: load, inspect, and manage developer (source) packages
  - PreprocessMode: when the local preprocess function runs relative to the global one
"""

import rez_next._native  # noqa: F401
from rez_next._native.developer_package import (  # noqa: F401
    DeveloperPackage,
    PreprocessMode,
)

# Re-export for convenience
__all__ = [
    "DeveloperPackage",
    "PreprocessMode",
]
