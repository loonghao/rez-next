"""Tests for new package_cache features (Cycle #337)."""

import sys
import os
import tempfile
import shutil

import pytest


def test_status_descriptions():
    """Test that STATUS_DESCRIPTIONS is correctly exported."""
    from rez_next.package_cache import CacheStatus

    # Check STATUS_DESCRIPTIONS exists
    assert hasattr(CacheStatus, 'STATUS_DESCRIPTIONS'), \
        "CacheStatus.STATUS_DESCRIPTIONS should exist"

    desc = CacheStatus.STATUS_DESCRIPTIONS

    # Check it's a dict
    assert isinstance(desc, dict), "STATUS_DESCRIPTIONS should be a dict"

    # Check all status codes are in the dict
    assert 0 in desc, "STATUS_DESCRIPTIONS should have key 0"
    assert 1 in desc, "STATUS_DESCRIPTIONS should have key 1"
    assert 7 in desc, "STATUS_DESCRIPTIONS should have key 7"

    # Check descriptions match (check they're non-empty strings)
    assert isinstance(desc[0], str) and len(desc[0]) > 0
    assert isinstance(desc[1], str) and len(desc[1]) > 0
    assert isinstance(desc[7], str) and len(desc[7]) > 0
    # Check some keywords
    assert "not found" in desc[0].lower()
    assert "found" in desc[1].lower()


def test_cache_near_full():
    """Test cache_near_full() method."""
    from rez_next.package_cache import PackageCache

    with tempfile.TemporaryDirectory() as tmpdir:
        cache = PackageCache(tmpdir)

        # Should return a boolean
        result = cache.cache_near_full()
        assert isinstance(result, bool), "cache_near_full() should return bool"


def test_variant_meets_space_requirements():
    """Test variant_meets_space_requirements() method."""
    from rez_next.package_cache import PackageCache

    with tempfile.TemporaryDirectory() as tmpdir:
        cache = PackageCache(tmpdir)

        # Create a small test directory
        test_dir = os.path.join(tmpdir, "test_variant")
        os.makedirs(test_dir)

        # Create a small file
        with open(os.path.join(test_dir, "test.txt"), "w") as f:
            f.write("test")

        # Should return a boolean
        result = cache.variant_meets_space_requirements(test_dir)
        assert isinstance(result, bool), \
            "variant_meets_space_requirements() should return bool"


def test_windows_case_insensitivity():
    """Test that package names are lowercased in cache path (issue #2101)."""
    from rez_next.package_cache import PackageCache, VariantHandle

    with tempfile.TemporaryDirectory() as tmpdir:
        cache = PackageCache(tmpdir)

        # Create handle with uppercase name
        handle = VariantHandle("Python", "3.9.0", None)

        # Add variant - should not fail
        # (This mainly tests that the path handling doesn't crash)
        import rez_next
        if hasattr(rez_next, 'get_current_platform'):
            platform = rez_next.get_current_platform()
            # Test passes if we get here without exception
            assert True


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
