"""Tests for rez_next.package_search module.

Note: The Python package_search.py file has a native namespace conflict
with _native.package_search submodule. The module accessible at
rez_next.package_search exposes the native _native.package_search contents.
"""

import pytest


class TestPackageSearchImport:
    """Verify the module is importable and exports expected names."""

    def test_module_importable(self):
        """Package search module is accessible from rez_next."""
        from rez_next import package_search
        assert package_search is not None

    def test_native_attrs_available(self):
        """Native attributes from _native.package_search are accessible."""
        from rez_next import package_search
        assert hasattr(package_search, "ResourceSearchResult")
        assert hasattr(package_search, "get_plugins")
        assert hasattr(package_search, "get_reverse_dependency_tree")

    def test_resource_search_result_class(self):
        from rez_next.package_search import ResourceSearchResult
        assert ResourceSearchResult is not None

    def test_get_plugins_function(self):
        from rez_next.package_search import get_plugins
        assert callable(get_plugins)

    def test_get_reverse_dependency_tree_function(self):
        from rez_next.package_search import get_reverse_dependency_tree
        assert callable(get_reverse_dependency_tree)


class TestSearchNativeModule:
    """Search functionality is available via _native.search submodule."""

    def test_native_search_has_searcher(self):
        import rez_next
        assert hasattr(rez_next._native.search, "PackageSearcher")
        assert hasattr(rez_next._native.search, "SearchResult")
        assert hasattr(rez_next._native.search, "search_packages")
        assert hasattr(rez_next._native.search, "search_package_names")
        assert hasattr(rez_next._native.search, "search_latest_packages")
