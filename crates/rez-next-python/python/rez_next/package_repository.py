"""
package_repository — aligns with rez.package_repository.

Provides the abstract ``PackageRepository`` base class, concrete repository
manager, factory functions, and singleton manager — without legacy cruft.

Designed with:
- Clean ABC hierarchy (no mutable defaults, no hidden state)
- Thread-safe global stats via thread-local storage
- Clear error messages with actionable context
- Cross-platform path normalisation
"""

from __future__ import annotations

import os
import abc
import threading
import time
from contextlib import contextmanager
from typing import Any, ClassVar, Iterator, Optional, TYPE_CHECKING

if TYPE_CHECKING:
    from rez_next.version import Version


# ── Global stats ──────────────────────────────────────────────────────────


class PackageRepositoryGlobalStats(threading.local):
    """Thread-local gatherer of statistics across all package repositories.

    Rez API: ``rez.package_repository.PackageRepositoryGlobalStats``
    """

    def __init__(self) -> None:
        super().__init__()
        self.package_load_time: float = 0.0

    @contextmanager
    def package_loading(self) -> Iterator[None]:
        """Context manager that times package loading."""
        t1 = time.time()
        try:
            yield
        finally:
            self.package_load_time += time.time() - t1


package_repo_stats = PackageRepositoryGlobalStats()


# ── Factory helpers ───────────────────────────────────────────────────────


def get_package_repository_types() -> list[str]:
    """Return available package repository implementation names.

    Returns:
        Sorted list of registered repository type names.
    """
    from rez_next.plugin_managers import plugin_manager
    return sorted(plugin_manager.get_plugins("package_repository"))


def create_memory_package_repository(
    repository_data: dict[str, Any],
) -> Any:
    """Create a standalone in-memory package repository.

    Args:
        repository_data: Data to populate the repository with (maps
            package name to version info).

    Returns:
        A ``MemoryPackageRepository`` instance.
    """
    from rez_next.plugin_managers import plugin_manager
    cls = plugin_manager.get_plugin_class("package_repository", "memory")
    return cls.create_repository(repository_data)


# ── Abstract base class ───────────────────────────────────────────────────


class PackageRepository(abc.ABC):
    """Base class for package repository implementations.

    Concrete repositories are expected to register via the plugin system
    under the ``package_repository`` plugin type.

    Rez API: ``rez.package_repository.PackageRepository``
    """

    # Sentinel used to indicate a package should be removed
    remove: Any = object()

    # ── Class-level interface ─────────────────────────────────────────

    @classmethod
    def name(cls) -> str:
        """Return the repository type name, e.g. ``'filesystem'``."""
        raise NotImplementedError

    # ── Instance interface ───────────────────────────────────────────

    def __init__(self, location: str, pool: Any = None) -> None:
        self.location: str = os.path.abspath(location) if location else location
        self.pool = pool

    def __str__(self) -> str:
        return f"{self.name()}@{self.location}"

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, PackageRepository):
            return NotImplemented
        return other.name() == self.name() and other.uid == self.uid

    def __hash__(self) -> int:
        return hash(self.uid)

    # ── Identity ─────────────────────────────────────────────────────

    @property
    def uid(self) -> tuple[str, str]:
        """Unique identifier for this repository."""
        return (self.name(), str(self.location))

    # ── Resource management ──────────────────────────────────────────

    def register_resource(self, resource_class: type) -> None:
        if self.pool is not None:
            self.pool.register_resource(resource_class)

    def clear_caches(self) -> None:
        if self.pool is not None:
            self.pool.clear_caches()

    def is_empty(self) -> bool:
        """Return True if the repository contains no packages."""
        for family in self.iter_package_families():
            for _ in self.iter_packages(family):
                return False
        return True

    # ── Package iteration / lookup ───────────────────────────────────

    def get_package_family(self, name: str) -> Any:
        """Return the package family resource for *name*, or None."""
        raise NotImplementedError

    def iter_package_families(self) -> Iterator[Any]:
        """Iterate over all package families in the repository."""
        raise NotImplementedError

    def iter_packages(self, package_family_resource: Any) -> Iterator[Any]:
        """Iterate over packages within a family."""
        raise NotImplementedError

    def iter_variants(self, package_resource: Any) -> Iterator[Any]:
        """Iterate over variants of a package."""
        raise NotImplementedError

    def get_package(self, name: str, version: Version) -> Any:
        """Return a specific package by name and version, or None.

        Default implementation iterates the family and compares versions.
        """
        fam = self.get_package_family(name)
        if fam is None:
            return None
        for pkg in self.iter_packages(fam):
            pkg_version = getattr(pkg, "version", None)
            if pkg_version == version:
                return pkg
        return None

    def get_package_from_uri(self, uri: str) -> Any:
        """Resolve a package from a URI string. Return None by default."""
        return None

    def get_variant_from_uri(self, uri: str) -> Any:
        """Resolve a variant from a URI string. Return None by default."""
        return None

    # ── Package lifecycle ────────────────────────────────────────────

    def ignore_package(
        self, pkg_name: str, pkg_version: Version, allow_missing: bool = False
    ) -> int:
        """Mark a package as ignored (won't appear in resolves).

        Returns:
            The number of packages ignored (0 or 1).
        """
        raise NotImplementedError

    def unignore_package(self, pkg_name: str, pkg_version: Version) -> int:
        """Remove an ignore marker from a package.

        Returns:
            The number of packages unignored (0 or 1).
        """
        raise NotImplementedError

    def remove_package(self, pkg_name: str, pkg_version: Version) -> bool:
        """Remove a specific package version.

        Returns:
            True if the package was removed.
        """
        raise NotImplementedError

    def remove_package_family(self, pkg_name: str, force: bool = False) -> bool:
        """Remove an entire package family.

        Returns:
            True if the family was removed.
        """
        raise NotImplementedError

    def remove_ignored_since(
        self, days: int, dry_run: bool = False, verbose: bool = False
    ) -> int:
        """Remove packages that have been ignored for *days* or more.

        Returns:
            The number of packages removed.
        """
        raise NotImplementedError

    # ── Install hooks ────────────────────────────────────────────────

    def pre_variant_install(self, variant_resource: Any) -> None:
        """Called before a variant is installed. Default: no-op."""
        pass

    def on_variant_install_cancelled(self, variant_resource: Any) -> None:
        """Called when a variant install is cancelled. Default: no-op."""
        pass

    def install_variant(
        self,
        variant_resource: Any,
        dry_run: bool = False,
        overrides: Optional[dict[str, Any]] = None,
    ) -> Any:
        """Install a variant into the repository."""
        raise NotImplementedError

    def get_equivalent_variant(self, variant_resource: Any) -> Any:
        """Return the equivalent variant if it already exists (dry-run install)."""
        return self.install_variant(variant_resource, dry_run=True)

    # ── Parent lookup ────────────────────────────────────────────────

    def get_parent_package_family(self, package_resource: Any) -> Any:
        raise NotImplementedError

    def get_parent_package(self, variant_resource: Any) -> Any:
        raise NotImplementedError

    # ── State / metadata ─────────────────────────────────────────────

    def get_variant_state_handle(self, variant_resource: Any) -> Any:
        """Return a hashable handle representing variant state.

        Used for cache invalidation. Returns None by default (no state).
        """
        return None

    def get_last_release_time(
        self, package_family_resource: Any
    ) -> int:
        """Return the last release time (epoch seconds) for a package family.

        Returns 0 if unknown.
        """
        return 0

    # ── Payload path ─────────────────────────────────────────────────

    def get_package_payload_path(
        self, package_name: str,
        package_version: Optional[str] = None,
    ) -> str:
        """Return the filesystem path to a package's payload directory."""
        raise NotImplementedError


# ── Repository manager ────────────────────────────────────────────────────


class PackageRepositoryManager:
    """Manages package repository instances by caching them keyed by path.

    Rez API: ``rez.package_repository.PackageRepositoryManager``
    """

    def __init__(self, pool: Any = None) -> None:
        if pool is None:
            # Use config to determine cache size, default to unbounded
            try:
                from rez_next.config import config as cfg
                cache_size: int | None = getattr(
                    cfg, "resource_caching_maxsize", -1
                )
                if cache_size is not None and cache_size < 0:
                    cache_size = None
            except Exception:
                cache_size = None
            pool = _create_resource_pool(cache_size)
        self.pool = pool
        self.repositories: dict[str, PackageRepository] = {}

    def get_repository(self, path: str) -> PackageRepository:
        """Return (or create and cache) a repository for the given path.

        Paths of the form ``'type@location'`` are supported; if no type
        separator is present the default ``'filesystem'`` type is assumed.
        """
        parts = path.split("@", 1)
        if len(parts) == 1:
            parts = ["filesystem", parts[0]]
        repo_type, location = parts
        if repo_type == "filesystem":
            location = os.path.abspath(location)
        normalised = f"{repo_type}@{location}"

        existing = self.repositories.get(normalised)
        if existing is not None:
            return existing

        repo = self._create_repository(normalised)
        self.repositories[normalised] = repo
        return repo

    def are_same(self, path_1: str, path_2: str) -> bool:
        """Return True if *path_1* and *path_2* resolve to the same repo."""
        if path_1 == path_2:
            return True
        return self.get_repository(path_1).uid == self.get_repository(path_2).uid

    def get_resource(
        self, resource_key: str, repository_type: str,
        location: str, **variables: Any,
    ) -> Any:
        """Load a resource from the repository identified by type+location."""
        path = f"{repository_type}@{location}"
        repo = self.get_repository(path)
        return repo.get_resource(resource_key, **variables)

    def clear_caches(self) -> None:
        """Clear all cached repositories and pool resources."""
        self.repositories.clear()
        if self.pool is not None:
            self.pool.clear_caches()

    def _create_repository(self, path: str, **repo_args: Any) -> PackageRepository:
        """Instantiate the repository plugin for the given normalised path."""
        from rez_next.plugin_managers import plugin_manager
        repo_type, location = path.split("@", 1)
        cls = plugin_manager.get_plugin_class("package_repository", repo_type)
        return cls(location, self.pool, **repo_args)


# ── Internal helpers ──────────────────────────────────────────────────────


def _create_resource_pool(maxsize: Optional[int] = None) -> Any:
    """Create a simple resource pool.

    If the rez_next resource pool implementation is available, use it.
    Otherwise return None (no caching).
    """
    try:
        from rez_next.utils.resources import ResourcePool
        return ResourcePool(cache_size=maxsize)
    except ImportError:
        return None


# Module-level singleton (matches rez.package_repository.package_repository_manager)
package_repository_manager = PackageRepositoryManager()
