"""Tests for rez_next.package_search module (Cycle #322)."""

import pytest
import rez_next.package_search as package_search


class TestPackageSearchModule:
    """Test that rez_next.package_search has the expected API."""

    def test_module_importable(self):
        """Test that the module can be imported."""
        assert package_search is not None

    def test_has_search_packages(self):
        """Test that search_packages function is available."""
        assert hasattr(package_search, 'search_packages')
        assert callable(getattr(package_search, 'search_packages'))

    def test_has_search_package_names(self):
        """Test that search_package_names function is available."""
        assert hasattr(package_search, 'search_package_names')
        assert callable(getattr(package_search, 'search_package_names'))

    def test_has_search_latest_packages(self):
        """Test that search_latest_packages function is available."""
        assert hasattr(package_search, 'search_latest_packages')
        assert callable(getattr(package_search, 'search_latest_packages'))

    def test_has_package_searcher(self):
        """Test that PackageSearcher class is available."""
        assert hasattr(package_search, 'PackageSearcher')

    def test_has_resource_searcher_alias(self):
        """Test that ResourceSearcher alias is available (for compatibility)."""
        assert hasattr(package_search, 'ResourceSearcher')

    def test_resource_searcher_is_package_searcher(self):
        """Test that ResourceSearcher is an alias for PackageSearcher."""
        assert package_search.ResourceSearcher is package_search.PackageSearcher

    def test_has_get_latest_package(self):
        """Test that get_latest_package function is available."""
        assert hasattr(package_search, 'get_latest_package')
        assert callable(getattr(package_search, 'get_latest_package'))

    def test_has_iter_packages(self):
        """Test that iter_packages function is available."""
        assert hasattr(package_search, 'iter_packages')
        assert callable(getattr(package_search, 'iter_packages'))
