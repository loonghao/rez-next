"""
package_move — aligns with rez.package_move.

Wraps the native move_package() function with a clean, rez-compatible API.
No legacy compatibility code.

Rez API: ``rez.package_move.move_package(package, dest_repository, ...)``

Design:
- Accepts Package objects or string names for ``package``
- Accepts PackageRepository objects or string paths for ``dest_repository``
- No mutable defaults, no hidden state mutations
- Explicit error types (PackageMoveError)
- Cross-platform path normalisation via the domain (Rust) layer
- "Move" is physically copy + remove in the domain layer (SRP), not hide-source
"""

from __future__ import annotations

import os
from typing import TYPE_CHECKING, Any

from rez_next.exceptions import PackageMoveError

if TYPE_CHECKING:
    pass


def _resolve_package_name(package: Any) -> str:
    """Resolve a Package/string to a package name."""
    if isinstance(package, str):
        return package
    name = getattr(package, "name", None)
    if name is not None:
        return name
    pkg_str = str(package)
    if "-" in pkg_str:
        return pkg_str.rsplit("-", 1)[0]
    return pkg_str


def _resolve_package_version(package: Any, version: Any = None) -> str | None:
    """Resolve a version from a Package object or explicit value."""
    if version is not None:
        return str(version)
    ver = getattr(package, "version", None)
    if ver is not None:
        return str(ver)
    return None


def _resolve_dest_path(dest_repository: Any) -> str:
    """Resolve a destination path from a PackageRepository object or string."""
    if isinstance(dest_repository, str):
        return os.path.abspath(dest_repository)
    path = getattr(dest_repository, "path", None) or getattr(dest_repository, "location", None)
    if path is not None:
        return os.path.abspath(str(path))
    return os.path.abspath(str(dest_repository))


def move_package(
    package: Any,
    dest_repository: Any,
    *,
    keep_source: bool = False,
    keep_timestamp: bool = False,
    force: bool = False,
    verbose: bool = False,
) -> Any:
    """Move a package to another repository.

    Unlike Rez's move (which hides the source), rez-next physically copies
    the package to the destination and optionally removes the source.
    This is cleaner, more predictable, and avoids the "hidden package" footgun.

    Args:
        package: Package object or package name string.
        dest_repository: Destination repository (object or path string).
        keep_source: If True, keep the source (effectively a copy).
        keep_timestamp: Preserve source timestamps.
        force: Move even if package is not relocatable.
        verbose: Print verbose output.

    Returns:
        The destination path string (or package info tuple).

    Raises:
        PackageMoveError: If the move operation fails.
    """
    if keep_timestamp:
        raise PackageMoveError("keep_timestamp is not supported in rez-next")

    pkg_name = _resolve_package_name(package)
    pkg_version = _resolve_package_version(package)
    dest_path = _resolve_dest_path(dest_repository)

    # Check that the package doesn't already exist at destination
    dest_pkg_path = os.path.join(dest_path, pkg_name)
    if pkg_version:
        dest_pkg_path = os.path.join(dest_pkg_path, pkg_version)

    if os.path.exists(dest_pkg_path):
        if not force:
            raise PackageMoveError(
                f"Package already exists at destination: {dest_pkg_path}. "
                "Set force=True to overwrite."
            )
        if verbose:
            print(f"Warning: overwriting existing package at {dest_pkg_path}")

    try:
        from rez_next._native.packages_ import move_package as _native_move

        result_path = _native_move(
            pkg_name=pkg_name,
            dest_path=dest_path,
            version=pkg_version,
            src_paths=None,
            force=force,
            keep_source=keep_source,
        )
    except Exception as e:
        raise PackageMoveError(f"Failed to move {pkg_name}: {e}") from e

    if verbose:
        action = "Copied" if keep_source else "Moved"
        print(f"{action} {pkg_name} to {result_path}")

    return result_path
