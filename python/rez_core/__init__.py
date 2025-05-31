"""
Rez Core - High-performance core components for Rez package manager

This package provides Python bindings for the Rust-based rez-core library,
offering high-performance version resolution, dependency management, and
package repository operations.
"""

# Core imports from the Rust extension
from ._rez_core import (
    # Version system
    Version,
    VersionRange,

    # Package system
    Package,
    PackageVariant,
    PackageRequirement,
)

# Try to import error types with fallbacks
try:
    from ._rez_core import PyVersionParseError as VersionParseError
except ImportError:
    VersionParseError = ValueError

try:
    from ._rez_core import RezCoreError
except ImportError:
    RezCoreError = Exception

__version__ = "0.1.0"
__author__ = "Long Hao"
__email__ = "hal.long@outlook.com"

__all__ = [
    # Core version management
    "Version",
    "VersionRange",

    # Package management
    "Package",
    "PackageVariant",
    "PackageRequirement",

    # Error handling
    "RezCoreError",
    "VersionParseError",

    # Metadata
    "__version__",
    "__author__",
    "__email__",
]
