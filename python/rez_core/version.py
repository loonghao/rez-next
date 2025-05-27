"""
Version system module for rez-core.

This module provides a clean interface to the version system,
re-exporting the Rust implementations with Python-friendly names.
"""

from . import (
    Version,
    VersionRange,
    VersionToken,
    NumericToken,
    AlphanumericVersionToken,
    parse_version,
    parse_version_range,
    VersionParseError,
)

__all__ = [
    "Version",
    "VersionRange", 
    "VersionToken",
    "NumericToken",
    "AlphanumericVersionToken",
    "parse_version",
    "parse_version_range",
    "VersionParseError",
]

# Convenience functions for common operations
def create_version(version_str):
    """Create a Version object from a string."""
    return parse_version(version_str)

def create_range(range_str):
    """Create a VersionRange object from a string."""
    return parse_version_range(range_str)

def compare_versions(v1, v2):
    """Compare two version strings, returning -1, 0, or 1."""
    version1 = parse_version(v1) if isinstance(v1, str) else v1
    version2 = parse_version(v2) if isinstance(v2, str) else v2
    
    if version1 < version2:
        return -1
    elif version1 > version2:
        return 1
    else:
        return 0

def sort_versions(versions, reverse=False):
    """Sort a list of version strings or Version objects."""
    version_objects = [
        parse_version(v) if isinstance(v, str) else v 
        for v in versions
    ]
    return sorted(version_objects, reverse=reverse)
