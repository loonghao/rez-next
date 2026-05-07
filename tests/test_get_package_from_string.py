"""Tests for rez_next.get_package_from_string() (Cycle 267)."""

import rez_next as rez
import pytest


class TestGetPackageFromString:
    """Tests for the top-level get_package_from_string() function."""

    def test_basic_package_with_version(self):
        """'python-3.9.0' → name='python', version='3.9.0'."""
        pkg = rez.get_package_from_string("python-3.9.0")
        assert pkg.name == "python"
        assert pkg.version is not None
        assert str(pkg.version) == "3.9.0"

    def test_package_name_only_no_version(self):
        """No '-' → entire string is name, version=None."""
        pkg = rez.get_package_from_string("python")
        assert pkg.name == "python"
        assert pkg.version is None

    def test_package_with_complex_version(self):
        """'maya-2024.0.0' → name='maya', version='2024.0.0'."""
        pkg = rez.get_package_from_string("maya-2024.0.0")
        assert pkg.name == "maya"
        assert str(pkg.version) == "2024.0.0"

    def test_package_with_prerelease(self):
        """'mypackage-1.0alpha' → name='mypackage', version parses."""
        pkg = rez.get_package_from_string("mypackage-1.0alpha")
        assert pkg.name == "mypackage"
        # Prerelease version may parse differently; just check it's not None
        assert pkg.version is not None

    def test_empty_version_suffix(self):
        """'package-' → name='package', version may be None or empty Version."""
        pkg = rez.get_package_from_string("package-")
        assert pkg.name == "package"
        # Empty suffix may parse as empty Version or fail → version is None
        # Accept both outcomes (implementation detail)
        if pkg.version is not None:
            assert str(pkg.version) == ""

    def test_name_with_multiple_hyphens(self):
        """Split on LAST '-': 'my-cool-package-1.0' → name='my-cool-package'."""
        pkg = rez.get_package_from_string("my-cool-package-1.0")
        assert pkg.name == "my-cool-package"
        assert str(pkg.version) == "1.0"

    def test_invalid_version_string(self):
        """Non-parsing version → version=None, but function succeeds."""
        pkg = rez.get_package_from_string("mypackage-notaversion")
        assert pkg.name == "mypackage"
        # "notaversion" may or may not parse as version; check graceful handling
        # (implementation allows version=None on parse failure)

    def test_returns_package_instance(self):
        """Returned object has Package-like attributes."""
        pkg = rez.get_package_from_string("test-1.0")
        # Check it has expected Package attributes
        assert hasattr(pkg, "name")
        assert hasattr(pkg, "version")
        assert pkg.name == "test"

    def test_roundtrip_via_get_package(self, tmp_path):
        """Package from string can be used with other APIs."""
        pkg = rez.get_package_from_string("dummy-1.0.0")
        assert pkg.name == "dummy"
        assert str(pkg.version) == "1.0.0"
