"""Tests for rez_next.package_cache module."""

import os
import tempfile
import time

import pytest
from rez_next.package_cache import (
    CacheStats,
    CachedPackage,
    InMemoryCache,
    PackageCache,
    VariantHandle,
    new_file_based_cache,
    new_in_memory_cache,
)


class TestPackageCacheImport:
    """Verify the module is importable and exports correct names."""

    def test_module_importable(self):
        from rez_next import package_cache
        assert hasattr(package_cache, "PackageCache")
        assert hasattr(package_cache, "CachedPackage")
        assert hasattr(package_cache, "CacheStats")
        assert hasattr(package_cache, "InMemoryCache")

    def test_variant_handle_importable(self):
        # VariantHandle may be a class from native module or a dict alias
        assert VariantHandle is not None

    def test_convenience_functions(self):
        assert callable(new_in_memory_cache)
        assert callable(new_file_based_cache)

    def test_convenience_new_in_memory(self):
        cache = new_in_memory_cache()
        assert isinstance(cache, PackageCache)

    def test_convenience_new_file_based(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            cache = new_file_based_cache(tmpdir)
            assert isinstance(cache, PackageCache)


class TestCachedPackage:
    """Test CachedPackage class."""

    def test_create_cached_package(self):
        pkg = CachedPackage("test_pkg", "1.0.0", "/tmp/test", {"key": "val"})
        assert pkg.name == "test_pkg"
        assert pkg.version == "1.0.0"
        assert pkg.path == "/tmp/test"
        assert pkg.data == {"key": "val"}

    def test_cached_package_repr(self):
        pkg = CachedPackage("test_pkg", "1.0.0", "/tmp/test", {})
        r = repr(pkg)
        assert "test_pkg" in r
        assert "1.0.0" in r

    def test_cached_package_valid_default(self):
        pkg = CachedPackage("test_pkg", "1.0.0", "/tmp/test", {})
        assert pkg.is_valid() is True

    def test_cached_package_ttl(self):
        pkg = CachedPackage("test_pkg", "1.0.0", "/tmp/test", {})
        assert pkg.ttl() is None
        pkg.set_ttl(60)
        assert pkg.ttl() == 60

    def test_cached_package_ttl_expired(self):
        pkg = CachedPackage("test_pkg", "1.0.0", "/tmp/test", {})
        pkg.set_ttl(0)  # Immediately expired
        pkg.cached_at = time.time() - 10  # 10 seconds ago
        assert pkg.is_valid() is False

    def test_cached_package_source_mtime(self):
        """Skip if file doesn't exist."""
        pkg = CachedPackage("test_pkg", "1.0.0", "/nonexistent/path", {})
        assert pkg.source_mtime is None


class TestCacheStats:
    """Test CacheStats class."""

    def test_create_stats(self):
        stats = CacheStats()
        assert stats.hits() == 0
        assert stats.misses() == 0
        assert stats.puts() == 0
        assert stats.removes() == 0
        assert stats.clears() == 0

    def test_stats_repr(self):
        stats = CacheStats()
        r = repr(stats)
        assert "0" in r
        assert "hits" in r

    def test_stats_increment(self):
        stats = CacheStats()
        stats._hits = 5
        stats._misses = 3
        assert stats.hits() == 5
        assert stats.misses() == 3


class TestInMemoryCache:
    """Test InMemoryCache class."""

    def test_create_cache(self):
        cache = InMemoryCache()
        assert cache is not None

    def test_put_and_get(self):
        cache = InMemoryCache()
        pkg = CachedPackage("test_pkg", "1.0.0", "/test", {})
        cache.put("/test", pkg)
        result = cache.get("/test")
        assert result is not None
        assert result.name == "test_pkg"

    def test_get_miss(self):
        cache = InMemoryCache()
        result = cache.get("/nonexistent")
        assert result is None

    def test_put_package(self):
        cache = InMemoryCache()
        cache.put_package("/test", "test_pkg", "1.0.0", {})
        result = cache.get("/test")
        assert result is not None
        assert result.name == "test_pkg"
        assert result.version == "1.0.0"

    def test_remove(self):
        cache = InMemoryCache()
        cache.put_package("/test", "test_pkg", "1.0.0", {})
        cache.remove("/test")
        assert cache.get("/test") is None

    def test_clear(self):
        cache = InMemoryCache()
        cache.put_package("/a", "pkg1", "1.0", {})
        cache.put_package("/b", "pkg2", "2.0", {})
        cache.clear()
        assert cache.get("/a") is None
        assert cache.get("/b") is None

    def test_stats(self):
        cache = InMemoryCache()
        stats = cache.stats()
        assert isinstance(stats, CacheStats)

    def test_set_default_ttl(self):
        cache = InMemoryCache()
        cache.set_default_ttl(100)
        cache.put_package("/test", "test", "1.0", {})
        pkg = cache.get("/test")
        assert pkg is not None


class TestPackageCacheIntegration:
    """Integration tests with PackageCache."""

    def test_create_in_memory(self):
        cache = PackageCache()
        assert cache is not None
        assert cache.get_root() == ""

    def test_create_file_based(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            cache = PackageCache(tmpdir)
            assert cache is not None
            root = cache.get_root()
            assert os.path.isabs(root)

    def test_cache_clean(self):
        cache = PackageCache()
        result = cache.clean()
        assert isinstance(result, tuple)
        assert len(result) == 2
