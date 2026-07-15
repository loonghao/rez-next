"""
package_copy — aligns with rez.package_copy.

Wraps the native copy_package() function with a clean, rez-compatible API.
No legacy compatibility code.

Rez API: ``rez.package_copy.copy_package(package, dest_repository, ...)``

Design:
- Accepts Package objects or string names for ``package``
- Accepts PackageRepository objects or string paths for ``dest_repository``
- No mutable defaults, no hidden state mutations
- Explicit error types (PackageCopyError)
- Cross-platform path normalisation via the domain (Rust) layer
"""

from __future__ import annotations

import os
from typing import TYPE_CHECKING, Any

from rez_next.exceptions import PackageCopyError

if TYPE_CHECKING:
    pass


def _resolve_package_name(package: Any) -> str:
    """Resolve a Package/string to a package name."""
    if isinstance(package, str):
        return package
    name = getattr(package, "name", None)
    if name is not None:
        return name
    # Try to get from version-less representation
    pkg_str = str(package)
    # Rez package str is typically "name-version"
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
    # PackageRepository object — try common attributes
    path = getattr(dest_repository, "path", None) or getattr(dest_repository, "location", None)
    if path is not None:
        return os.path.abspath(str(path))
    return os.path.abspath(str(dest_repository))


def _resolve_src_paths(package_repos: list[Any] | None = None) -> list[str] | None:
    """Resolve source paths from repository objects or strings."""
    if package_repos is None:
        return None
    result: list[str] = []
    for repo in package_repos:
        if isinstance(repo, str):
            result.append(os.path.abspath(repo))
        else:
            path = getattr(repo, "path", None) or getattr(repo, "location", None)
            if path is not None:
                result.append(os.path.abspath(str(path)))
            else:
                result.append(os.path.abspath(str(repo)))
    return result or None


def copy_package(
    package: Any,
    dest_repository: Any,
    *,
    variants: list[int] | None = None,
    shallow: bool = False,
    dest_name: str | None = None,
    dest_version: str | None = None,
    overwrite: bool = False,
    force: bool = False,
    follow_symlinks: bool = False,
    dry_run: bool = False,
    keep_timestamp: bool = False,
    skip_payload: bool = False,
    overrides: dict[str, Any] | None = None,
    verbose: bool = False,
) -> dict[str, list[tuple[Any, Any]]]:
    """Copy a package to another repository.

    Args:
        package: Package object or package name string.
        dest_repository: Destination repository (object or path string).
        variants: Variant indices to copy (None = all).
        shallow: If True, create symlinks instead of copying payload.
        dest_name: Rename the package on copy.
        dest_version: Change the version on copy.
        overwrite: Overwrite existing variants at destination.
        force: Copy even if package is not relocatable.
        follow_symlinks: Follow symlinks when copying payload.
        dry_run: Preview only, don't actually copy.
        keep_timestamp: Preserve source timestamps.
        skip_payload: Only copy metadata, not payload.
        overrides: Additional override attributes for install_variant.
        verbose: Print verbose output.

    Returns:
        dict with "copied" and "skipped" keys (each a list of (src, dst) variant tuples).

    Raises:
        PackageCopyError: If the copy operation fails.
    """
    if shallow:
        raise PackageCopyError("Shallow copy (symlinks) not yet supported in rez-next")

    unsupported = {
        "variants": variants is not None,
        "dest_name": dest_name is not None,
        "dest_version": dest_version is not None,
        "follow_symlinks": follow_symlinks,
        "dry_run": dry_run,
        "keep_timestamp": keep_timestamp,
        "skip_payload": skip_payload,
    }
    requested = [name for name, enabled in unsupported.items() if enabled]
    if requested:
        raise PackageCopyError("Unsupported copy options: " + ", ".join(requested))

    if overrides:
        raise PackageCopyError(
            "Custom overrides not yet supported in rez-next. "
            "For name/version changes, use dest_name/dest_version parameters."
        )

    pkg_name = _resolve_package_name(package)
    pkg_version = _resolve_package_version(package)
    dest_path = _resolve_dest_path(dest_repository)

    try:
        from rez_next._native.packages_ import copy_package as _native_copy

        result_path = _native_copy(
            pkg_name=pkg_name,
            dest_path=dest_path,
            version=pkg_version,
            src_paths=None,
            force=force or overwrite,
        )
    except Exception as e:
        raise PackageCopyError(f"Failed to copy {pkg_name}: {e}") from e

    if verbose:
        print(f"Copied {pkg_name} to {result_path}")

    # Build result dict matching rez's return format
    result: dict[str, list[tuple[Any, Any]]] = {
        "copied": [],
        "skipped": [],
    }

    if result_path:
        # Try to construct a lightweight result
        src_info = (pkg_name, str(pkg_version or "(no version)"))
        dst_info = (pkg_name, str(pkg_version or "(no version)"))
        result["copied"].append((src_info, dst_info))

    return result
