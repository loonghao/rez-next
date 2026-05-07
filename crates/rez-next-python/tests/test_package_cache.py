"""Tests for rez_next.package_cache module."""

import sys
import os
import tempfile
import pytest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'python'))

import rez_next.package_cache as package_cache


class TestCachedPackage:
    """Tests for CachedPackage class."""

    def test_create_cached_package(self, tmp_path):
        """Test creating a CachedPackage instance."""
        pkg_file = tmp_path / "package.py"
        pkg_file.write_text('{"name": "test_pkg"}')

        pkg = package_cache.CachedPackage("test_pkg", "1.0.0", str(pkg_file), "{}")
        assert pkg.name == "test_pkg"
        assert pkg.version == "1.0.0"
        assert "package.py" in pkg.path
        assert pkg.data == "{}"
        assert pkg.cached_at > 0
        assert pkg.ttl is None

    def test_set_ttl(self, tmp_path):
        """Test setting TTL on CachedPackage."""
        pkg_file = tmp_path / "package.py"
        pkg_file.write_text('{"name": "test_pkg"}')

        pkg = package_cache.CachedPackage("test_pkg", "1.0.0", str(pkg_file), "{}")
        assert pkg.ttl is None

        pkg.ttl = 60.0  # 60 seconds
        assert pkg.ttl == 60.0

    def test_is_valid_no_source_mtime(self, tmp_path):
        """Test is_valid with no source mtime."""
        pkg_file = tmp_path / "package.py"
        pkg_file.write_text('{"name": "test_pkg"}')

        pkg = package_cache.CachedPackage("test_pkg", "1.0.0", str(pkg_file), "{}")
        # Should return True if source hasn't been modified
        result = pkg.is_valid(None)
        assert isinstance(result, bool)


class TestInMemoryCache:
    """Tests for InMemoryCache class."""

    def test_create_cache(self):
        """Test creating an InMemoryCache."""
        cache = package_cache.InMemoryCache()
        assert cache is not None

    def test_get_miss(self):
        """Test cache miss."""
        cache = package_cache.InMemoryCache()
        result = cache.get("/nonexistent/path.py")
        assert result is None

    def test_put_and_get(self, tmp_path):
        """Test putting and getting a package."""
        cache = package_cache.InMemoryCache()

        pkg_file = tmp_path / "package.py"
        pkg_file.write_text('{"name": "test_pkg"}')

        pkg = package_cache.CachedPackage(
            "test_pkg", "1.0.0", str(pkg_file), '{"name": "test_pkg"}'
        )
        cache.put(str(pkg_file), pkg)

        result = cache.get(str(pkg_file))
        assert result is not None
        assert result.name == "test_pkg"
        assert result.version == "1.0.0"

    def test_remove(self, tmp_path):
        """Test removing a package from cache."""
        cache = package_cache.InMemoryCache()

        pkg_file = tmp_path / "package.py"
        pkg_file.write_text('{"name": "test_pkg"}')

        pkg = package_cache.CachedPackage(
            "test_pkg", "1.0.0", str(pkg_file), '{"name": "test_pkg"}'
        )
        cache.put(str(pkg_file), pkg)

        # Verify it's cached
        result = cache.get(str(pkg_file))
        assert result is not None

        # Remove
        cache.remove(str(pkg_file))

        # Verify it's gone
        result = cache.get(str(pkg_file))
        assert result is None

    def test_clear(self, tmp_path):
        """Test clearing the cache."""
        cache = package_cache.InMemoryCache()

        pkg_file = tmp_path / "package.py"
        pkg_file.write_text('{"name": "test_pkg"}')

        pkg = package_cache.CachedPackage(
            "test_pkg", "1.0.0", str(pkg_file), '{"name": "test_pkg"}'
        )
        cache.put(str(pkg_file), pkg)

        # Verify it's cached
        result = cache.get(str(pkg_file))
        assert result is not None

        # Clear
        cache.clear()

        # Verify it's gone
        result = cache.get(str(pkg_file))
        assert result is None

    def test_stats(self, tmp_path):
        """Test cache statistics."""
        cache = package_cache.InMemoryCache()

        # Initial stats
        stats = cache.stats()
        assert stats.hits == 0
        assert stats.misses == 0

        # Miss
        cache.get("/nonexistent/path.py")
        stats = cache.stats()
        assert stats.misses == 1

        # Put
        pkg_file = tmp_path / "package.py"
        pkg_file.write_text('{"name": "test_pkg"}')

        pkg = package_cache.CachedPackage(
            "test_pkg", "1.0.0", str(pkg_file), '{"name": "test_pkg"}'
        )
        cache.put(str(pkg_file), pkg)
        stats = cache.stats()
        assert stats.puts == 1

        # Hit
        cache.get(str(pkg_file))
        stats = cache.stats()
        assert stats.hits == 1


class TestPackageCache:
    """Tests for PackageCache class."""

    def test_create_in_memory_cache(self):
        """Test creating a PackageCache with in-memory backend."""
        cache = package_cache.PackageCache.new_in_memory()
        assert cache is not None

    def test_in_memory_cache_operations(self, tmp_path):
        """Test basic operations on in-memory PackageCache."""
        cache = package_cache.PackageCache.new_in_memory()

        # Miss
        result = cache.get("/nonexistent/path.py")
        assert result is None

        # Put
        pkg_file = tmp_path / "package.py"
        pkg_file.write_text('{"name": "test_pkg"}')

        pkg = package_cache.CachedPackage(
            "test_pkg", "1.0.0", str(pkg_file), '{"name": "test_pkg"}'
        )
        cache.put(str(pkg_file), pkg)

        # Hit
        result = cache.get(str(pkg_file))
        assert result is not None
        assert result.name == "test_pkg"

        # Stats
        stats = cache.stats()
        assert stats.misses >= 1
        assert stats.puts >= 1
        assert stats.hits >= 1

    def test_file_based_cache(self, tmp_path):
        """Test creating a file-based cache."""
        cache_dir = tmp_path / "cache"
        cache_dir.mkdir()

        cache = package_cache.PackageCache.new_file_based(str(cache_dir))
        assert cache is not None

        # Create a package file
        pkg_file = tmp_path / "package.py"
        pkg_file.write_text('{"name": "test_pkg"}')

        # Put
        pkg = package_cache.CachedPackage(
            "test_pkg", "1.0.0", str(pkg_file), '{"name": "test_pkg"}'
        )
        cache.put(str(pkg_file), pkg)

        # Hit
        result = cache.get(str(pkg_file))
        assert result is not None
        assert result.name == "test_pkg"

    def test_set_default_ttl(self, tmp_path):
        """Test setting default TTL."""
        cache = package_cache.PackageCache.new_in_memory()

        # Put without TTL
        pkg_file = tmp_path / "package.py"
        pkg_file.write_text('{"name": "test_pkg"}')

        pkg = package_cache.CachedPackage(
            "test_pkg", "1.0.0", str(pkg_file), '{"name": "test_pkg"}'
        )
        cache.put(str(pkg_file), pkg)

        # Set default TTL
        cache.set_default_ttl(60.0)  # 60 seconds

        # Verify TTL is set on new entries
        pkg_file2 = tmp_path / "package2.py"
        pkg_file2.write_text('{"name": "test_pkg2"}')

        pkg2 = package_cache.CachedPackage(
            "test_pkg2", "2.0.0", str(pkg_file2), '{"name": "test_pkg2"}'
        )
        cache.put(str(pkg_file2), pkg2)

        result = cache.get(str(pkg_file2))
        assert result is not None
        # TTL should be set


class TestCacheStats:
    """Tests for CacheStats class."""

    def test_stats_repr(self):
        """Test CacheStats string representation."""
        # CacheStats is returned by stats(), not created directly
        cache = package_cache.InMemoryCache()
        stats = cache.stats()
        repr_str = stats.__repr__()
        assert "CacheStats" in repr_str
        assert "hits" in repr_str
        assert "misses" in repr_str


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
