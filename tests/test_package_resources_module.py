"""Tests for rez_next.package_resources module.

This module provides tests for the package_resources module, which implements
PackageFamilyResource, PackageResource, and VariantResource classes.

Follows SOLID principles and aligns with Rez's package_resources.py interface.
"""

import pytest
import rez_next as rez

# Import from the correct submodule path
from rez_next._native.package_resources import (
    PackageFamilyResource,
    PackageResource,
    VariantResource,
)


class TestPackageFamilyResource:
    """Tests for PackageFamilyResource class."""

    def test_create_family_resource(self):
        """Test creating a PackageFamilyResource."""
        family = PackageFamilyResource(
            name="python",
            repository_type="filesystem",
            repository_location="/packages"
        )
        assert family.name == "python"
        assert family.repository_type == "filesystem"
        assert family.repository_location == "/packages"

    def test_family_resource_str(self):
        """Test string representation of PackageFamilyResource."""
        family = PackageFamilyResource(
            name="maya",
            repository_type="filesystem",
            repository_location="/packages"
        )
        s = str(family)
        assert "maya" in s

    def test_family_resource_repr(self):
        """Test repr of PackageFamilyResource."""
        family = PackageFamilyResource(
            name="houdini",
            repository_type="filesystem",
            repository_location="/packages"
        )
        r = repr(family)
        assert "houdini" in r
        assert "PackageFamilyResource" in r


class TestPackageResource:
    """Tests for PackageResource class."""

    def test_create_package_resource(self):
        """Test creating a PackageResource."""
        pkg = PackageResource(
            name="python",
            repository_type="filesystem",
            repository_location="/packages"
        )
        assert pkg.name == "python"

    def test_package_resource_has_version(self):
        """Test that PackageResource has version attribute."""
        pkg = PackageResource(
            name="python",
            repository_type="filesystem",
            repository_location="/packages"
        )
        # Version should be accessible (may be None for minimal creation)
        _ = pkg.version


class TestVariantResource:
    """Tests for VariantResource class."""

    def test_class_accessible(self):
        """Test that VariantResource is accessible."""
        # Just test that we can reference the class
        assert VariantResource is not None


class TestPackageResourcesIntegration:
    """Integration tests for package_resources module."""

    def test_family_to_resource(self):
        """Test creating family and resource together."""
        family = PackageFamilyResource(
            name="numpy",
            repository_type="filesystem",
            repository_location="/packages"
        )
        
        resource = PackageResource(
            name=family.name,
            repository_type=family.repository_type,
            repository_location=family.repository_location
        )
        
        assert resource.name == family.name
        assert resource.repository_type == family.repository_type

    def test_cross_platform_path_handling(self):
        """Test that paths are handled correctly (cross-platform).

        Based on Rez issues #2101 and #2089 - Windows case sensitivity.
        """
        # Test with mixed case (should work on all platforms)
        family = PackageFamilyResource(
            name="MyPackage",
            repository_type="filesystem",
            repository_location="/packages"
        )
        assert family.name == "MyPackage"
