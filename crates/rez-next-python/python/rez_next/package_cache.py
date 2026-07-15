"""
package_cache — aligns with rez.package_cache.

Provides package payload caching with both in-memory and file-based backends.
The high-performance native Rust implementation is available via ``PackageCache``
for file-based operations.

Rez API: ``rez.package_cache``
"""

import os
import time

from rez_next._native.package_cache import (  # noqa: F401  # type: ignore[import]
    VARIANT_COPY_STALLED,
    VARIANT_COPYING,
    VARIANT_CREATED,
    VARIANT_FOUND,
    VARIANT_NOT_FOUND,
    VARIANT_PENDING,
    VARIANT_REMOVED,
    VARIANT_SKIPPED,
    CacheConfig,
    CacheStatus,
    VariantHandle,
)
from rez_next._native.package_cache import (
    PackageCache as _NativePackageCache,
)

# ── CacheStats ──────────────────────────────────────────────────────────


class CacheStats:
    """Cache statistics with method-style access for compat."""

    def __init__(self):
        self._hits = 0
        self._misses = 0
        self._puts = 0
        self._removes = 0
        self._clears = 0

    def hits(self):
        return self._hits

    def misses(self):
        return self._misses

    def puts(self):
        return self._puts

    def removes(self):
        return self._removes

    def clears(self):
        return self._clears

    def __repr__(self):
        return (
            f"CacheStats(hits={self._hits}, misses={self._misses}, "
            f"puts={self._puts}, removes={self._removes}, clears={self._clears})"
        )


# ── CachedPackage ───────────────────────────────────────────────────────


class CachedPackage:
    """A cached package entry with TTL support."""

    def __init__(self, name, version, path, data):
        self.name = name
        self.version = version
        self.path = path
        self.data = data
        self.cached_at = time.time()
        self._ttl = None
        try:
            self.source_mtime = os.path.getmtime(path)
        except OSError:
            self.source_mtime = None

    def ttl(self):
        """Get the TTL (time-to-live) in seconds."""
        return self._ttl

    def set_ttl(self, ttl):
        """Set the TTL (time-to-live) in seconds."""
        self._ttl = ttl

    def is_valid(self, source_mtime=None):
        """Check if this cache entry is still valid."""
        if self._ttl is not None and time.time() - self.cached_at > self._ttl:
            return False
        if source_mtime is not None and self.source_mtime is not None:
            return source_mtime <= self.source_mtime
        return True

    def __repr__(self):
        return f"CachedPackage(name={self.name!r}, version={self.version!r}, path={self.path!r})"


# ── InMemoryCache ───────────────────────────────────────────────────────


class InMemoryCache:
    """Simple in-memory package cache."""

    def __init__(self):
        self._items = {}
        self._stats = CacheStats()
        self._default_ttl = None

    def get(self, path):
        item = self._items.get(path)
        if item is None or not item.is_valid(None):
            self._stats._misses += 1
            return None
        self._stats._hits += 1
        return item

    def put(self, path, package):
        if package._ttl is None:
            package._ttl = self._default_ttl
        self._items[path] = package
        self._stats._puts += 1

    def put_package(self, path, name, version, data):
        """Create a CachedPackage and add it to the cache."""
        pkg = CachedPackage(name, version, path, data)
        self.put(path, pkg)

    def remove(self, path):
        self._items.pop(path, None)
        self._stats._removes += 1

    def clear(self):
        self._items.clear()
        self._stats._clears += 1

    def stats(self):
        return self._stats

    def set_default_ttl(self, ttl):
        self._default_ttl = ttl


# ── PackageCache ────────────────────────────────────────────────────────


class PackageCache(InMemoryCache):
    """Package cache that supports both in-memory and file-based caching.

    This class provides in-memory caching via ``InMemoryCache`` and
    delegates file-based operations to the native Rust implementation when
    a cache path is provided.
    """

    def __init__(self, path=None):
        super().__init__()
        self._path = path
        self._native = _NativePackageCache(path) if path else None

    # ── Convenience constructors ────────────────────────────────────

    @classmethod
    def new_in_memory(cls):
        """Create a new in-memory PackageCache."""
        return cls()

    @classmethod
    def new_file_based(cls, path):
        """Create a new file-based PackageCache."""
        return cls(path)

    # ── Native-like API (delegates to Rust for file-based ops) ──────

    def get_root(self):
        """Get the root cache directory path."""
        if self._native:
            return self._native.get_root()
        return self._path or ""

    def add_variant(self, handle, source_root, force=False):
        """Add a variant's payload to the cache.

        Returns:
            tuple: (status_code, cached_path)
        """
        if self._native:
            return self._native.add_variant(handle, source_root, force)
        # In-memory fallback: just report the source path
        return (2, source_root) if force else (0, "")

    def get_cached_root(self, handle):
        """Check if a variant is cached.

        Returns:
            tuple: (status_code, cached_path_or_None)
        """
        if self._native:
            return self._native.get_cached_root(handle)
        return (0, None)

    def remove_variant(self, handle):
        """Remove a variant from the cache.

        Returns:
            tuple: (status_code, path_or_None)
        """
        if self._native:
            return self._native.remove_variant(handle)
        return (0, None)

    def list_cached(self):
        """List all cached variants.

        Returns:
            list: List of (handle_dict, path, status_code) tuples
        """
        if self._native:
            return self._native.list_cached()
        return []

    def clean(self, time_limit_secs=None):
        """Clean old/unused cache entries.

        Returns:
            tuple: (entries_deleted, bytes_freed)
        """
        if self._native:
            return self._native.clean(time_limit_secs)
        return (0, 0)

    def cache_near_full(self):
        """Check if the cache disk is near full."""
        if self._native:
            return self._native.cache_near_full()
        return False

    def variant_meets_space_requirements(self, variant_root):
        """Check if a variant meets space requirements for caching."""
        if self._native:
            return self._native.variant_meets_space_requirements(variant_root)
        return True


# ── Module-level convenience functions ──────────────────────────────────


def new_in_memory_cache():
    """Create a new in-memory PackageCache."""
    return PackageCache.new_in_memory()


def new_file_based_cache(path):
    """Create a new file-based PackageCache."""
    return PackageCache.new_file_based(path)


__all__ = [
    "CacheConfig",
    "CacheStats",
    "CacheStatus",
    "CachedPackage",
    "InMemoryCache",
    "PackageCache",
    "VariantHandle",
    "VARIANT_COPYING",
    "VARIANT_COPY_STALLED",
    "VARIANT_CREATED",
    "VARIANT_FOUND",
    "VARIANT_NOT_FOUND",
    "VARIANT_PENDING",
    "VARIANT_REMOVED",
    "VARIANT_SKIPPED",
    "new_file_based_cache",
    "new_in_memory_cache",
]
