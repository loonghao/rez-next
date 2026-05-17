"""
Package removal module for rez_next.

This module provides functions to remove packages and package families,
aligned with Rez's package_remove.py interface.
"""

from __future__ import annotations

from typing import Optional

try:
    from rez_next._native.package_repository import FilesystemPackageRepository
except ImportError:
    try:
        from _native.package_repository import FilesystemPackageRepository
    except ImportError:
        FilesystemPackageRepository = None


def remove_package_family(
    name: str,
    path: str,
    force: bool = False
) -> bool:
    """
    Remove a package family from a repository.

    Args:
        name: Package family name to remove.
        path: Repository path containing the package family.
        force: If True, remove even if family has packages.

    Returns:
        True if removed, False if not found.

    Raises:
        RezError: If removal fails.
    """
    if FilesystemPackageRepository is None:
        raise RuntimeError("Native package repository module not available")

    repo = FilesystemPackageRepository(path)
    return repo.remove_package_family(name, force)


def remove_package(
    name: str,
    version,
    path: str
) -> bool:
    """
    Remove a specific package version from a repository.

    Args:
        name: Package name to remove.
        version: Package version (string or Version object).
        path: Repository path containing the package.

    Returns:
        True if removed, False if not found.

    Raises:
        RezError: If removal fails.
    """
    if FilesystemPackageRepository is None:
        raise RuntimeError("Native package repository module not available")

    # Convert version to string if needed
    version_str = str(version) if version is not None else None

    repo = FilesystemPackageRepository(path)
    return repo.remove_package(name, version_str)


def remove_packages_ignored_since(
    days: int,
    paths: Optional[list[str]] = None,
    dry_run: bool = False,
    verbose: bool = False
) -> int:
    """
    Remove packages that have been ignored for more than specified days.

    Args:
        days: Remove packages ignored for more than this many days.
        paths: List of repository paths to search. If None, uses default paths.
        dry_run: If True, only count without removing.
        verbose: If True, print verbose output.

    Returns:
        Number of packages removed (or would be removed if dry_run).
    """
    if FilesystemPackageRepository is None:
        raise RuntimeError("Native package repository module not available")

    # If paths not specified, we can only process one path at a time
    # This is a simplified implementation - full implementation would
    # use config.packages_path as default
    if paths is None:
        raise ValueError(
            "paths must be specified (config.packages_path not yet implemented)"
        )

    total_removed = 0
    for path in paths:
        repo = FilesystemPackageRepository(path)
        removed = repo.remove_ignored_since(days, dry_run, verbose)
        total_removed += removed

    return total_removed
