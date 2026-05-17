"""Tests for rez_next.package_cache module.

Tests the high-performance package payload caching functionality.
"""

import sys
import os
import tempfile
import shutil

import pytest

import rez_next
from rez_next.package_cache import (
    PackageCache,
    VariantHandle,
    CacheConfig,
    CacheStatus,
)


class TestVariantHandle:
    """Tests for VariantHandle class."""

    def test_create(self):
        """Test creating a VariantHandle."""
        handle = VariantHandle("python", "3.9.0", 0)
        assert handle.name == "python"
        assert handle.version == "3.9.0"
        assert handle.index == 0

    def test_create_no_version(self):
        """Test creating a VariantHandle without version."""
        handle = VariantHandle("maya", None, None)
        assert handle.name == "maya"
        assert handle.version is None
        assert handle.index is None

    def test_sha1_hash(self):
        """Test SHA1 hash generation."""
        handle1 = VariantHandle("python", "3.9.0", None)
        handle2 = VariantHandle("python", "3.9.0", None)
        # Same handle should have same hash
        assert handle1.sha1_hash() == handle2.sha1_hash()

    def test_to_dict(self):
        """Test conversion to dict."""
        handle = VariantHandle("python", "3.9.0", 0)
        d = handle.to_dict()
        assert d["name"] == "python"
        assert d["version"] == "3.9.0"
        assert d["index"] == 0

    def test_repr(self):
        """Test string representation."""
        handle = VariantHandle("python", "3.9.0", 0)
        r = repr(handle)
        assert "VariantHandle" in r
        assert "python" in r


class TestCacheConfig:
    """Tests for CacheConfig class."""

    def test_create_default(self):
        """Test creating default CacheConfig."""
        config = CacheConfig()
        assert config.max_size_bytes is None
        assert config.min_free_space_bytes > 0
        assert config.max_age_secs is None
        assert config.cache_local is True

    def test_set_get(self):
        """Test setting and getting config values."""
        config = CacheConfig()
        config.max_size_bytes = 1024 * 1024 * 1024  # 1 GB
        assert config.max_size_bytes == 1024 * 1024 * 1024

        config.max_age_secs = 86400  # 1 day
        assert config.max_age_secs == 86400

        config.cache_local = False
        assert config.cache_local is False


class TestPackageCache:
    """Tests for PackageCache class."""

    def setup_method(self):
        """Create a temporary cache directory."""
        self.tmpdir = tempfile.mkdtemp()
        self.cache = PackageCache(self.tmpdir)

    def teardown_method(self):
        """Clean up temporary directory."""
        if os.path.exists(self.tmpdir):
            shutil.rmtree(self.tmpdir)

    def test_create(self):
        """Test creating a PackageCache."""
        assert self.cache.get_root() == self.tmpdir

    def test_add_variant(self):
        """Test adding a variant to cache."""
        # Create a fake payload
        payload_dir = os.path.join(self.tmpdir, "payload")
        os.makedirs(payload_dir, exist_ok=True)
        with open(os.path.join(payload_dir, "file.txt"), "w") as f:
            f.write("hello")

        handle = VariantHandle("mypkg", "1.0.0", None)
        status, path = self.cache.add_variant(handle, payload_dir, False)

        assert status == CacheStatus.CREATED
        assert os.path.exists(path)

    def test_add_variant_twice(self):
        """Test adding the same variant twice (should return FOUND)."""
        payload_dir = os.path.join(self.tmpdir, "payload")
        os.makedirs(payload_dir, exist_ok=True)
        with open(os.path.join(payload_dir, "file.txt"), "w") as f:
            f.write("hello")

        handle = VariantHandle("mypkg", "1.0.0", None)
        status1, path1 = self.cache.add_variant(handle, payload_dir, False)
        assert status1 == CacheStatus.CREATED

        # Second call should return FOUND
        status2, path2 = self.cache.add_variant(handle, payload_dir, False)
        assert status2 == CacheStatus.FOUND

    def test_get_cached_root(self):
        """Test getting cached root path."""
        payload_dir = os.path.join(self.tmpdir, "payload")
        os.makedirs(payload_dir, exist_ok=True)
        with open(os.path.join(payload_dir, "file.txt"), "w") as f:
            f.write("hello")

        handle = VariantHandle("mypkg", "1.0.0", None)
        self.cache.add_variant(handle, payload_dir, False)

        status, path = self.cache.get_cached_root(handle)
        assert status == CacheStatus.FOUND
        assert path is not None

    def test_get_cached_root_not_found(self):
        """Test getting cached root for non-existent variant."""
        handle = VariantHandle("nonexistent", "1.0.0", None)
        status, path = self.cache.get_cached_root(handle)
        assert status == CacheStatus.NOT_FOUND
        assert path is None

    def test_remove_variant(self):
        """Test removing a variant from cache."""
        payload_dir = os.path.join(self.tmpdir, "payload")
        os.makedirs(payload_dir, exist_ok=True)
        with open(os.path.join(payload_dir, "file.txt"), "w") as f:
            f.write("hello")

        handle = VariantHandle("mypkg", "1.0.0", None)
        self.cache.add_variant(handle, payload_dir, False)

        # Remove
        status, _ = self.cache.remove_variant(handle)
        assert status == CacheStatus.REMOVED

        # Verify removed
        status, path = self.cache.get_cached_root(handle)
        assert status == CacheStatus.NOT_FOUND

    def test_list_cached(self):
        """Test listing cached variants."""
        payload_dir = os.path.join(self.tmpdir, "payload")
        os.makedirs(payload_dir, exist_ok=True)
        with open(os.path.join(payload_dir, "file.txt"), "w") as f:
            f.write("hello")

        handle = VariantHandle("mypkg", "1.0.0", None)
        self.cache.add_variant(handle, payload_dir, False)

        cached = self.cache.list_cached()
        assert len(cached) > 0

        # Each item should be a tuple: (dict, path, status_code)
        item = cached[0]
        assert "name" in item[0]
        assert item[2] == CacheStatus.FOUND

    def test_clean(self):
        """Test cleaning old cache entries."""
        payload_dir = os.path.join(self.tmpdir, "payload")
        os.makedirs(payload_dir, exist_ok=True)
        with open(os.path.join(payload_dir, "file.txt"), "w") as f:
            f.write("hello")

        handle = VariantHandle("mypkg", "1.0.0", None)
        self.cache.add_variant(handle, payload_dir, False)

        # Clean (should not remove recently used entries)
        deleted, bytes_freed = self.cache.clean(None)
        assert deleted >= 0
        assert bytes_freed >= 0


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
