"""Tests for rez_next.package_search module.

This module tests the package_search functionality
aligned with Rez's package_search.py interface.
"""

import pytest
import rez_next
from rez_next.package_search import (
    ResourceSearchResult,
    get_plugins,
    get_reverse_dependency_tree,
)


class TestResourceSearchResult:
    """Test ResourceSearchResult class."""

    def test_create(self):
        """Test creating a ResourceSearchResult."""
        result = ResourceSearchResult("python", "family")
        assert result.resource == "python"
        assert result.resource_type == "family"
        assert result.validation_error is None

    def test_create_with_error(self):
        """Test creating a ResourceSearchResult with validation error."""
        result = ResourceSearchResult("python", "family")
        # Note: validation_error is read-only property
        # The error would be set during search, not during creation
        assert result.validation_error is None

    def test_repr(self):
        """Test string representation."""
        result = ResourceSearchResult("python", "family")
        repr_str = repr(result)
        assert "python" in repr_str
        assert "family" in repr_str


class TestGetPlugins:
    """Test get_plugins function."""

    def test_nonexistent_package(self):
        """Test with non-existent package name."""
        result = get_plugins("nonexistent_package_xyz", None)
        assert isinstance(result, list)
        assert len(result) == 0

    def test_empty_paths(self):
        """Test with empty paths."""
        result = get_plugins("python", [])
        assert isinstance(result, list)

    def test_nonexistent_path(self):
        """Test with non-existent path."""
        result = get_plugins("python", ["C:\\nonexistent\\path"])
        assert isinstance(result, list)
        assert len(result) == 0


class TestGetReverseDependencyTree:
    """Test get_reverse_dependency_tree function."""

    def test_nonexistent_package(self):
        """Test with non-existent package name."""
        layers, graph = get_reverse_dependency_tree(
            "nonexistent_package_xyz", None, None, False, False
        )
        assert isinstance(layers, list)
        assert isinstance(graph, dict)
        # Layer 0 should contain the package itself
        assert len(layers) >= 1
        assert "nonexistent_package_xyz" in layers[0]

    def test_empty_paths(self):
        """Test with empty paths."""
        layers, graph = get_reverse_dependency_tree(
            "python", None, [], False, False
        )
        assert isinstance(layers, list)
        assert isinstance(graph, dict)

    def test_with_depth_limit(self):
        """Test with depth limit."""
        layers, graph = get_reverse_dependency_tree(
            "python", 2, None, False, False
        )
        assert isinstance(layers, list)
        assert isinstance(graph, dict)
        # Should not exceed depth limit
        assert len(layers) <= 3  # Layer 0 + up to 2 more layers


class TestModuleIntegration:
    """Test module integration with rez_next."""

    def test_import_from_rez_next(self):
        """Test that functions can be imported from rez_next.package_search."""
        assert hasattr(rez_next.package_search, "get_plugins")
        assert hasattr(rez_next.package_search, "get_reverse_dependency_tree")
        assert hasattr(rez_next.package_search, "ResourceSearchResult")

    def test_function_callable(self):
        """Test that functions are callable."""
        assert callable(get_plugins)
        assert callable(get_reverse_dependency_tree)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
