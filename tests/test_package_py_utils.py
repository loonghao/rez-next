"""
Tests for rez_next.package_py_utils module.

Tests the expand_requirement and expand_requirements functions
that correspond to rez's package_py_utils.py interface.
"""

import sys
import os

# Add the crates/rez-next-python/python directory to path
# so we can import rez_next with _native
sys.path.insert(0, os.path.join(
    os.path.dirname(__file__), "..", "crates", "rez-next-python", "python"
))

import rez_next


class TestExpandRequirement:
    """Test expand_requirement function."""

    def test_no_wildcards(self):
        """Test that requirements without wildcards are returned as-is."""
        result = rez_next.package_py_utils.expand_requirement("python-3.9")
        assert result == "python-3.9"

    def test_with_wildcard_no_callback(self):
        """Test that wildcards without callback return original."""
        result = rez_next.package_py_utils.expand_requirement("python-3.*")
        # Without callback, should return original or expanded if possible
        assert result is not None

    def test_with_callback(self):
        """Test expansion with a callback function."""
        def query_func(name, version_spec):
            if name == "python":
                return "3.9.0"
            return None

        result = rez_next.package_py_utils.expand_requirement(
            "python-3.*", query_func=query_func
        )
        assert result is not None

    def test_invalid_requirement(self):
        """Test handling of invalid requirement strings."""
        # Should not crash
        result = rez_next.package_py_utils.expand_requirement("invalid-format")
        assert result is not None


class TestExpandRequirements:
    """Test expand_requirements function."""

    def test_empty_list(self):
        """Test with empty list."""
        result = rez_next.package_py_utils.expand_requirements([])
        assert result == []

    def test_no_wildcards(self):
        """Test with requirements that have no wildcards."""
        reqs = ["python-3.9", "maya-2024"]
        result = rez_next.package_py_utils.expand_requirements(reqs)
        assert len(result) == 2
        assert "python-3.9" in result
        assert "maya-2024" in result

    def test_with_callback(self):
        """Test expansion with callback."""
        def query_func(name, version_spec):
            if name == "python":
                return "3.9.0"
            return None

        reqs = ["python-3.*", "maya-2024"]
        result = rez_next.package_py_utils.expand_requirements(
            reqs, query_func=query_func
        )
        assert len(result) == 2


class TestModuleStructure:
    """Test that the module has the expected structure."""

    def test_module_exists(self):
        """Test that package_py_utils module exists."""
        assert hasattr(rez_next, "package_py_utils")

    def test_expand_requirement_exists(self):
        """Test that expand_requirement function exists."""
        assert hasattr(rez_next.package_py_utils, "expand_requirement")
        assert callable(rez_next.package_py_utils.expand_requirement)

    def test_expand_requirements_exists(self):
        """Test that expand_requirements function exists."""
        assert hasattr(rez_next.package_py_utils, "expand_requirements")
        assert callable(rez_next.package_py_utils.expand_requirements)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
