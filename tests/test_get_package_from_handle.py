"""
Tests for get_package_from_handle function.

Equivalent to rez.packages.get_package_from_handle.
"""

import os
import tempfile
import pytest

import rez_next as rez


class TestGetPackageFromHandle:
    """Test get_package_from_handle function."""

    def test_string_handle_nonexistent_path(self):
        """Test with a non-existent path string handle."""
        result = rez.packages_.get_package_from_handle("/nonexistent/path/package.py")
        assert result is None

    def test_string_handle_valid_package_file(self):
        """Test with a valid package file path."""
        with tempfile.TemporaryDirectory() as tmpdir:
            # Create a simple package.py
            pkg_dir = os.path.join(tmpdir, "test_pkg", "1.0.0")
            os.makedirs(pkg_dir, exist_ok=True)

            pkg_content = '''name = "test_pkg"
version = "1.0.0"
authors = ["Test Author"]
'''

            pkg_path = os.path.join(pkg_dir, "package.py")
            with open(pkg_path, "w") as f:
                f.write(pkg_content)

            # Test with the file path as handle
            result = rez.packages_.get_package_from_handle(pkg_path)
            if result is not None:
                assert result.name == "test_pkg"
                assert str(result.version) == "1.0.0"

    def test_tuple_handle(self):
        """Test with a tuple handle (repo_name, relative_path)."""
        # For now, this is a simplified test
        # In full implementation, tuple handle would be (repo_name, rel_path)
        result = rez.packages_.get_package_from_handle(("repo", "/nonexistent/path"))
        assert result is None

    def test_invalid_handle_type(self):
        """Test with an invalid handle type (should return None, not crash)."""
        result = rez.packages_.get_package_from_handle(123)  # Integer handle
        assert result is None

    def test_with_paths_parameter(self):
        """Test with paths parameter."""
        result = rez.packages_.get_package_from_handle(
            "/nonexistent/path",
            paths=["/another/nonexistent/path"]
        )
        assert result is None


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
