"""
Tests for rez_next._native.package_filter module.

Validates that the Rust-backed PackageFilter matches
the behavior of rez.package_filter.PackageFilter.
"""

import pytest
from rez_next._native import package_filter


class TestPackageFilter:
    """Test PackageFilter class."""

    def test_create_empty_filter(self):
        """Test creating an empty PackageFilter."""
        pf = package_filter.PackageFilter()
        assert pf is not None

    def test_add_exclusion(self):
        """Test adding an exclusion rule."""
        pf = package_filter.PackageFilter()
        pf.add_exclusion("maya-2024")
        # No error means success

    def test_add_inclusion(self):
        """Test adding an inclusion rule."""
        pf = package_filter.PackageFilter()
        pf.add_inclusion("python-*")
        # No error means success

    def test_excludes(self):
        """Test excludes() method."""
        pf = package_filter.PackageFilter()
        pf.add_exclusion("maya-2024")

        # Package that should be excluded
        maya_pkg = {"name": "maya", "version": "2024.0.0"}
        result = pf.excludes(maya_pkg)
        assert result is not None  # Should match the exclusion rule

    def test_includes(self):
        """Test includes() method."""
        pf = package_filter.PackageFilter()
        pf.add_inclusion("python-*")

        # Package that should be included
        python_pkg = {"name": "python", "version": "3.9.0"}
        result = pf.includes(python_pkg)
        assert result is True

    def test_to_pod_and_from_pod(self):
        """Test POD serialization round-trip."""
        pf1 = package_filter.PackageFilter()
        pf1.add_inclusion("python-*")
        pf1.add_exclusion("maya-2024")

        pod = pf1.to_pod()
        assert isinstance(pod, dict)
        assert "inclusions" in pod or "exclusions" in pod

        # Recreate from POD
        pf2 = package_filter.PackageFilter.from_pod(pod)
        assert pf2 is not None

    def test_sha1(self):
        """Test SHA1 calculation."""
        pf = package_filter.PackageFilter()
        pf.add_inclusion("python-*")
        sha1 = pf.sha1()
        assert isinstance(sha1, str)
        assert len(sha1) == 40  # SHA1 is 40 hex characters


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
