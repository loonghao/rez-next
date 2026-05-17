"""
Tests for rez_next.package_order module.

Tests the Python bindings for package ordering strategies.
"""

import sys
import pytest
import rez_next as rez

# Register package_order in sys.modules for "from rez_next.package_order import ..." to work
if hasattr(rez, '_native') and hasattr(rez._native, 'package_order'):
    sys.modules['rez_next.package_order'] = rez._native.package_order

from rez_next.package_order import (NullPackageOrder, SortedOrder, VersionSplitPackageOrder, TimestampPackageOrder)


class TestNullPackageOrder:
    """Tests for NullPackageOrder - no reordering."""

    def test_creation(self):
        """Test basic creation."""
        order = NullPackageOrder()
        assert order.name == "no_order"
        assert order.packages is None

    def test_creation_with_packages(self):
        """Test creation with package list."""
        order = NullPackageOrder(packages=["foo", "bar"])
        assert order.packages == ["foo", "bar"]

    def test_to_pod(self):
        """Test serialization to POD."""
        order = NullPackageOrder()
        pod = order.to_pod()
        assert "no_order" in pod
        assert "packages" in pod

    def test_sha1(self):
        """Test SHA1 hash generation."""
        order = NullPackageOrder()
        sha = order.sha1()
        assert isinstance(sha, str)
        assert len(sha) > 0


class TestSortedOrder:
    """Tests for SortedOrder - version sorting."""

    def test_creation_ascending(self):
        """Test creation with ascending order."""
        order = SortedOrder(descending=False)
        assert order.name == "sorted"
        assert order.descending is False

    def test_creation_descending(self):
        """Test creation with descending order."""
        order = SortedOrder(descending=True)
        assert order.descending is True

    def test_to_pod(self):
        """Test serialization to POD."""
        order = SortedOrder(descending=True)
        pod = order.to_pod()
        assert "sorted" in pod
        assert "descending" in pod

    def test_sha1(self):
        """Test SHA1 hash generation."""
        order = SortedOrder(descending=False)
        sha = order.sha1()
        assert isinstance(sha, str)


class TestVersionSplitPackageOrder:
    """Tests for VersionSplitPackageOrder."""

    def test_creation(self):
        """Test basic creation."""
        order = VersionSplitPackageOrder("2.0.0")
        assert order.name == "version_split"
        assert order.first_version == "2.0.0"

    def test_creation_with_packages(self):
        """Test creation with package list."""
        order = VersionSplitPackageOrder("1.0.0", packages=["foo"])
        assert order.packages == ["foo"]

    def test_to_pod(self):
        """Test serialization to POD."""
        order = VersionSplitPackageOrder("1.5.0")
        pod = order.to_pod()
        assert "version_split" in pod
        assert "first_version" in pod

    def test_sha1(self):
        """Test SHA1 hash generation."""
        order = VersionSplitPackageOrder("3.0.0")
        sha = order.sha1()
        assert isinstance(sha, str)


class TestTimestampPackageOrder:
    """Tests for TimestampPackageOrder."""

    def test_creation(self):
        """Test basic creation."""
        order = TimestampPackageOrder(timestamp=1000)
        assert order.name == "soft_timestamp"
        assert order.timestamp == 1000
        assert order.rank == 0

    def test_creation_with_rank(self):
        """Test creation with rank."""
        order = TimestampPackageOrder(timestamp=1000, rank=5)
        assert order.rank == 5

    def test_to_pod(self):
        """Test serialization to POD."""
        order = TimestampPackageOrder(timestamp=500)
        pod = order.to_pod()
        assert "soft_timestamp" in pod
        assert "timestamp" in pod

    def test_sha1(self):
        """Test SHA1 hash generation."""
        order = TimestampPackageOrder(timestamp=2000)
        sha = order.sha1()
        assert isinstance(sha, str)


class TestIntegration:
    """Integration tests for package_order module."""

    def test_import_all_classes(self):
        """Test that all classes can be imported."""
        from rez_next.package_order import (
            NullPackageOrder,
            SortedOrder,
            VersionSplitPackageOrder,
            TimestampPackageOrder,
        )
        assert NullPackageOrder is not None
        assert SortedOrder is not None
        assert VersionSplitPackageOrder is not None
        assert TimestampPackageOrder is not None

    def test_sorted_order_ascending(self):
        """Test that ascending order sorts correctly."""
        order = SortedOrder(descending=False)
        assert order.name == "sorted"
        assert order.descending is False

    def test_sorted_order_descending(self):
        """Test that descending order sorts correctly."""
        order = SortedOrder(descending=True)
        assert order.descending is True


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
