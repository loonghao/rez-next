"""Tests for rez_next.packages_ module."""

import pytest
import rez_next.packages_ as packages_
from rez_next.packages_ import iter_packages, get_latest_package


class TestIterPackages:
    """Tests for iter_packages() function."""

    def test_iter_packages_returns_result(self):
        """iter_packages should return an iterable."""
        result = iter_packages("python")
        assert result is not None

    def test_iter_packages_is_iterable(self):
        """iter_packages result should be iterable."""
        result = iter_packages("python")
        # Try to iterate
        try:
            iterator = iter(result)
            assert iterator is not None
        except TypeError:
            pytest.fail("iter_packages result is not iterable")

    def test_iter_packages_with_path(self):
        """iter_packages should accept paths argument."""
        # Should not raise
        result = iter_packages("python", paths=[])
        assert result is not None


class TestGetLatestPackage:
    """Tests for get_latest_package() function."""

    def test_get_latest_package_returns_none_for_missing(self):
        """get_latest_package should return None for non-existent package."""
        result = get_latest_package("non_existent_package_xyz")
        assert result is None

    def test_get_latest_package_with_version_range(self):
        """get_latest_package should accept range_ argument."""
        # Should not raise
        result = get_latest_package("python", range_=">=3.8")
        # Result may be None or a Package object
        assert result is None or hasattr(result, "name")


class TestGetPackage:
    """Tests for get_package() function."""

    def test_get_package_returns_none_for_missing(self):
        """get_package should return None for non-existent package."""
        from rez_next.packages_ import get_package
        result = get_package("non_existent_package_xyz")
        assert result is None
