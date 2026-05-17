"""
Tests for package_remove module.

Test the package removal functions.
"""

import pytest
import tempfile
import os

from rez_next.package_remove import (
    remove_package,
    remove_package_family,
    remove_packages_ignored_since,
)


class TestRemovePackage:
    """Test remove_package function."""

    def test_remove_package_not_found(self):
        """Test removing a non-existent package returns False."""
        with tempfile.TemporaryDirectory() as tmpdir:
            result = remove_package("nonexistent", "1.0.0", tmpdir)
            assert result is False

    def test_remove_package_family_not_found(self):
        """Test removing a non-existent package family returns False."""
        with tempfile.TemporaryDirectory() as tmpdir:
            result = remove_package_family("nonexistent", tmpdir)
            assert result is False


class TestRemovePackagesIgnoredSince:
    """Test remove_packages_ignored_since function."""

    def test_remove_ignored_since_dry_run(self):
        """Test dry run mode counts but doesn't remove."""
        with tempfile.TemporaryDirectory() as tmpdir:
            count = remove_packages_ignored_since(30, paths=[tmpdir], dry_run=True)
            assert isinstance(count, int)
            assert count >= 0


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
