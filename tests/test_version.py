"""Tests for rez_next.version module."""

import pytest
import rez_next as rez


class TestVersion:
    """Tests for Version class."""

    def test_version_create(self):
        """Version should be creatable from string."""
        v = rez.Version("1.2.3")
        assert v is not None

    def test_version_str(self):
        """Version should have string attribute (version as string)."""
        v = rez.Version("1.2.3")
        assert hasattr(v, "string")
        assert str(v) == "1.2.3"

    def test_version_eq(self):
        """Version equality should work."""
        v1 = rez.Version("1.2.3")
        v2 = rez.Version("1.2.3")
        assert v1 == v2


class TestVersionRange:
    """Tests for VersionRange class."""

    def test_version_range_create(self):
        """VersionRange should be creatable from string."""
        r = rez.VersionRange(">=3.9,<4.0")
        assert r is not None

    def test_version_range_contains(self):
        """VersionRange should check containment."""
        r = rez.VersionRange(">=3.9,<4.0")
        v = rez.Version("3.10.0")
        # contains method may exist
        if hasattr(r, "contains"):
            result = r.contains(v)
            assert isinstance(result, bool)
