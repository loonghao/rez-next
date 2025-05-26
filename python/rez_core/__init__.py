"""
Rez Core - High-performance core components for Rez package manager

This package provides Python bindings for the Rust-based rez-core library,
offering high-performance version resolution, dependency management, and
package repository operations.
"""

from ._rez_core import (
    # Configuration
    Config,
    # Error types
    RezCoreError,
    # Version-related classes and functions
    Version,
    VersionParseError,
    VersionRange,
    VersionToken,
    parse_version,
    parse_version_range,
)

__version__ = "0.1.0"
__author__ = "Long Hao"
__email__ = "hal.long@outlook.com"

__all__ = [
    # Version management
    "Version",
    "VersionRange",
    "VersionToken",
    "parse_version",
    "parse_version_range",
    # Error handling
    "RezCoreError",
    "VersionParseError",
    # Configuration
    "Config",
    # Metadata
    "__version__",
    "__author__",
    "__email__",
]
