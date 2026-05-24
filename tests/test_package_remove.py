"""Tests for rez_next.package_remove module."""

import os
import tempfile

import pytest
from rez_next.package_remove import (
    remove_package,
    remove_package_family,
    remove_packages_ignored_since,
)


class TestPackageRemoveImport:
    """Verify the module is importable and exports correct names."""

    def test_module_importable(self):
        from rez_next import package_remove
        assert hasattr(package_remove, "remove_package")
        assert hasattr(package_remove, "remove_package_family")
        assert hasattr(package_remove, "remove_packages_ignored_since")

    def test_functions_callable(self):
        assert callable(remove_package)
        assert callable(remove_package_family)
        assert callable(remove_packages_ignored_since)


class TestRemovePackageFamily:
    """Test remove_package_family function."""

    def test_remove_nonexistent_family(self):
        """Removing a family that doesn't exist should return False."""
        with tempfile.TemporaryDirectory() as tmpdir:
            result = remove_package_family(
                "__nonexistent_family__", tmpdir, force=True
            )
            assert result is False

    def test_remove_nonexistent_family_no_force(self):
        """Removing a family that doesn't exist without force should not raise."""
        with tempfile.TemporaryDirectory() as tmpdir:
            result = remove_package_family(
                "__nonexistent_family__", tmpdir, force=False
            )
            assert result is False

    def test_remove_family_invalid_path(self):
        """Removing a family from an invalid path returns False."""
        result = remove_package_family("test_pkg", "/nonexistent/path", force=True)
        assert result is False

    def test_remove_family_empty_dir(self):
        """Removing from an empty directory returns False (no family exists)."""
        with tempfile.TemporaryDirectory() as tmpdir:
            result = remove_package_family("any_pkg", tmpdir, force=True)
            assert result is False


class TestRemovePackage:
    """Test remove_package function."""

    def test_remove_nonexistent_package(self):
        """Removing a package that doesn't exist should return False."""
        with tempfile.TemporaryDirectory() as tmpdir:
            result = remove_package(
                "__nonexistent_pkg__", "1.0.0", tmpdir
            )
            assert result is False

    def test_remove_package_invalid_path(self):
        """Removing from an invalid path returns False."""
        result = remove_package("test_pkg", "1.0", "/nonexistent/path")
        assert result is False


class TestRemovePackagesIgnoredSince:
    """Test remove_packages_ignored_since function."""

    def test_remove_ignored_since_empty_dir(self):
        """Removing ignored packages from empty dir returns 0."""
        with tempfile.TemporaryDirectory() as tmpdir:
            result = remove_packages_ignored_since(
                30, paths=[tmpdir], dry_run=True
            )
            assert isinstance(result, int)
            assert result == 0

    def test_remove_ignored_since_requires_paths(self):
        """paths argument is required."""
        with pytest.raises(ValueError):
            remove_packages_ignored_since(30)

    def test_remove_ignored_since_verbose(self):
        """verbose=True should not raise."""
        with tempfile.TemporaryDirectory() as tmpdir:
            result = remove_packages_ignored_since(
                30, paths=[tmpdir], dry_run=True, verbose=True
            )
            assert isinstance(result, int)
