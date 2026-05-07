"""
Tests for rez_next.package_cache module.

This module tests the package caching functionality,
including CachedPackage, InMemoryCache, PackageCache, CacheStats.
"""

import sys
import os
import tempfile
import time

import pytest
import rez_next
import rez_next.package_cache as package_cache


class TestPackageCacheModule:
    """Test that the package_cache module is accessible and has the right functions."""

    def test_module_accessible(self):
        """Test that rez_next.package_cache is accessible."""
        assert hasattr(rez_next, "package_cache")
        assert package_cache is not None

    def test_cached_package_class_exists(self):
        """Test that CachedPackage class exists."""
        assert hasattr(package_cache, "CachedPackage")
        assert callable(package_cache.CachedPackage)

    def test_in_memory_cache_class_exists(self):
        """Test that InMemoryCache class exists."""
        assert hasattr(package_cache, "InMemoryCache")
        assert callable(package_cache.InMemoryCache)

    def test_package_cache_class_exists(self):
        """Test that PackageCache class exists."""
        assert hasattr(package_cache, "PackageCache")
        assert callable(package_cache.PackageCache)

    def test_cache_stats_class_exists(self):
        """Test that CacheStats class exists."""
        assert hasattr(package_cache, "CacheStats")
        assert callable(package_cache.CacheStats)

    def test_new_in_memory_cache_function_exists(self):
        """Test that new_in_memory_cache function exists."""
        assert hasattr(package_cache, "new_in_memory_cache")
        assert callable(package_cache.new_in_memory_cache)

    def test_new_file_based_cache_function_exists(self):
        """Test that new_file_based_cache function exists."""
        assert hasattr(package_cache, "new_file_based_cache")
        assert callable(package_cache.new_file_based_cache)


class TestCachedPackage:
    """Test CachedPackage class."""

    def test_create_cached_package(self):
        """Test creating a CachedPackage instance."""
        pkg = package_cache.CachedPackage("test_package", "1.0.0", "/path/to/package.py", "name='test_package'")
        assert pkg.name == "test_package"
        assert pkg.version == "1.0.0"
        assert "package.py" in pkg.path
        assert pkg.data == "name='test_package'"
        assert isinstance(pkg.cached_at, float)
        assert pkg.ttl() is None

    def test_cached_package_set_ttl(self):
        """Test setting TTL on CachedPackage."""
        pkg = package_cache.CachedPackage("test_package", "1.0.0", "/path/to/package.py", "name='test_package'")
        assert pkg.ttl() is None
        pkg.set_ttl(60.0)  # 60 seconds
        assert pkg.ttl() == 60.0

    def test_cached_package_is_valid(self):
        """Test is_valid method."""
        pkg = package_cache.CachedPackage("test_package", "1.0.0", "/path/to/package.py", "name='test_package'")
        # Without TTL, should be valid
        assert pkg.is_valid() is True
        # With TTL set to 60 seconds, should still be valid
        pkg.set_ttl(60.0)
        assert pkg.is_valid() is True

    def test_cached_package_repr(self):
        """Test __repr__ method."""
        pkg = package_cache.CachedPackage("test_package", "1.0.0", "/path/to/package.py", "name='test_package'")
        repr_str = pkg.__repr__()
        assert "CachedPackage" in repr_str
        assert "test_package" in repr_str
        assert "1.0.0" in repr_str


class TestInMemoryCache:
    """Test InMemoryCache class."""

    def test_create_in_memory_cache(self):
        """Test creating an InMemoryCache instance."""
        cache = package_cache.InMemoryCache()
        assert cache is not None

    def test_put_and_get(self):
        """Test put and get operations."""
        cache = package_cache.InMemoryCache()
        pkg = package_cache.CachedPackage("test_package", "1.0.0", "/path/to/package.py", "name='test_package'")

        # Put package into cache
        cache.put("/path/to/package.py", pkg)

        # Get package from cache
        result = cache.get("/path/to/package.py")
        assert result is not None
        assert result.name == "test_package"
        assert result.version == "1.0.0"

    def test_get_nonexistent(self):
        """Test getting a non-existent package."""
        cache = package_cache.InMemoryCache()
        result = cache.get("/path/to/nonexistent.py")
        assert result is None

    def test_remove(self):
        """Test removing a package from cache."""
        cache = package_cache.InMemoryCache()
        pkg = package_cache.CachedPackage("test_package", "1.0.0", "/path/to/package.py", "name='test_package'")

        # Put then remove
        cache.put("/path/to/package.py", pkg)
        assert cache.get("/path/to/package.py") is not None

        cache.remove("/path/to/package.py")
        assert cache.get("/path/to/package.py") is None

    def test_clear(self):
        """Test clearing all packages from cache."""
        cache = package_cache.InMemoryCache()
        pkg1 = package_cache.CachedPackage("pkg1", "1.0.0", "/path/to/pkg1.py", "name='pkg1'")
        pkg2 = package_cache.CachedPackage("pkg2", "1.0.0", "/path/to/pkg2.py", "name='pkg2'")

        cache.put("/path/to/pkg1.py", pkg1)
        cache.put("/path/to/pkg2.py", pkg2)
        assert cache.get("/path/to/pkg1.py") is not None
        assert cache.get("/path/to/pkg2.py") is not None

        cache.clear()
        assert cache.get("/path/to/pkg1.py") is None
        assert cache.get("/path/to/pkg2.py") is None

    def test_stats(self):
        """Test cache statistics."""
        cache = package_cache.InMemoryCache()
        stats = cache.stats()
        assert stats is not None
        assert isinstance(stats.hits(), int)
        assert isinstance(stats.misses(), int)
        assert isinstance(stats.puts(), int)
        assert isinstance(stats.removes(), int)
        assert isinstance(stats.clears(), int)


class TestPackageCache:
    """Test PackageCache class."""

    def test_create_in_memory(self):
        """Test creating a PackageCache with in-memory backend."""
        cache = package_cache.PackageCache.new_in_memory()
        assert cache is not None

    def test_create_file_based(self):
        """Test creating a PackageCache with file-based backend."""
        with tempfile.TemporaryDirectory() as tmpdir:
            cache = package_cache.PackageCache.new_file_based(tmpdir)
            assert cache is not None

    def test_in_memory_put_and_get(self):
        """Test put and get with in-memory cache."""
        cache = package_cache.PackageCache.new_in_memory()
        pkg = package_cache.CachedPackage("test_package", "1.0.0", "/path/to/package.py", "name='test_package'")

        # Put package into cache
        cache.put("/path/to/package.py", pkg)

        # Get package from cache
        result = cache.get("/path/to/package.py")
        assert result is not None
        assert result.name == "test_package"

    def test_file_based_put_and_get(self):
        """Test put and get with file-based cache."""
        with tempfile.TemporaryDirectory() as tmpdir:
            cache = package_cache.PackageCache.new_file_based(tmpdir)
            pkg = package_cache.CachedPackage("test_package", "1.0.0", "/path/to/package.py", "name='test_package'")

            # Put package into cache
            cache.put("/path/to/package.py", pkg)

            # Get package from cache
            result = cache.get("/path/to/package.py")
            assert result is not None
            assert result.name == "test_package"

    def test_put_package(self):
        """Test put_package method."""
        cache = package_cache.PackageCache.new_in_memory()
        
        # Put package data directly
        cache.put_package("/path/to/package.py", "test_package", "1.0.0", "name='test_package'")
        
        # Get package from cache
        result = cache.get("/path/to/package.py")
        assert result is not None
        assert result.name == "test_package"
        assert result.version == "1.0.0"

    def test_set_default_ttl(self):
        """Test setting default TTL."""
        cache = package_cache.PackageCache.new_in_memory()
        # Should not raise
        cache.set_default_ttl(60.0)
        cache.set_default_ttl(None)  # Reset to default

    def test_remove(self):
        """Test removing a package from PackageCache."""
        cache = package_cache.PackageCache.new_in_memory()
        pkg = package_cache.CachedPackage("test_package", "1.0.0", "/path/to/package.py", "name='test_package'")

        cache.put("/path/to/package.py", pkg)
        assert cache.get("/path/to/package.py") is not None

        cache.remove("/path/to/package.py")
        assert cache.get("/path/to/package.py") is None

    def test_clear(self):
        """Test clearing all packages from PackageCache."""
        cache = package_cache.PackageCache.new_in_memory()
        pkg1 = package_cache.CachedPackage("pkg1", "1.0.0", "/path/to/pkg1.py", "name='pkg1'")
        pkg2 = package_cache.CachedPackage("pkg2", "1.0.0", "/path/to/pkg2.py", "name='pkg2'")

        cache.put("/path/to/pkg1.py", pkg1)
        cache.put("/path/to/pkg2.py", pkg2)

        cache.clear()
        assert cache.get("/path/to/pkg1.py") is None
        assert cache.get("/path/to/pkg2.py") is None

    def test_stats(self):
        """Test cache statistics."""
        cache = package_cache.PackageCache.new_in_memory()
        stats = cache.stats()
        assert stats is not None
        assert hasattr(stats, "hits")
        assert hasattr(stats, "misses")


class TestCacheStats:
    """Test CacheStats class."""

    def test_stats_attributes(self):
        """Test CacheStats attributes."""
        cache = package_cache.PackageCache.new_in_memory()
        stats = cache.stats()
        
        # Test all attributes
        assert isinstance(stats.hits(), (int, float))
        assert isinstance(stats.misses(), (int, float))
        assert isinstance(stats.puts(), (int, float))
        assert isinstance(stats.removes(), (int, float))
        assert isinstance(stats.clears(), (int, float))

    def test_stats_repr(self):
        """Test CacheStats __repr__ method."""
        cache = package_cache.PackageCache.new_in_memory()
        stats = cache.stats()
        repr_str = stats.__repr__()
        assert "CacheStats" in repr_str
        assert "hits=" in repr_str
        assert "misses=" in repr_str


class TestModuleLevelFunctions:
    """Test module-level functions."""

    def test_new_in_memory_cache(self):
        """Test new_in_memory_cache function."""
        cache = package_cache.new_in_memory_cache()
        assert isinstance(cache, package_cache.InMemoryCache)

    def test_new_file_based_cache(self):
        """Test new_file_based_cache function."""
        with tempfile.TemporaryDirectory() as tmpdir:
            cache = package_cache.new_file_based_cache(tmpdir)
            assert isinstance(cache, package_cache.PackageCache)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
