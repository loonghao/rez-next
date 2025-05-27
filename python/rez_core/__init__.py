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
    parse_version,
    parse_version_range,

    # Version tokens (rez-compatible)
    VersionToken,
    NumericToken,
    AlphanumericVersionToken,

    # Configuration (using the actual exported name)
    Config as RezCoreConfig,
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

# Alias for easier access
Config = RezCoreConfig

__version__ = "0.1.0"
__author__ = "Long Hao"
__email__ = "hal.long@outlook.com"

# Sub-modules for better organization
from . import version as version_module
from . import tokens as tokens_module
from . import errors as errors_module

__all__ = [
    # Core version management
    "Version",
    "VersionRange",
    "parse_version",
    "parse_version_range",

    # Version tokens (rez-compatible)
    "VersionToken",
    "NumericToken",
    "AlphanumericVersionToken",

    # Error handling
    "RezCoreError",
    "VersionParseError",

    # Configuration
    "Config",
    "RezCoreConfig",

    # Sub-modules
    "version_module",
    "tokens_module",
    "errors_module",

    # Metadata
    "__version__",
    "__author__",
    "__email__",
]
