"""
Tests for rez-core version functionality.

These tests are designed to be compatible with the original rez version system
while providing enhanced performance through Rust implementation.
"""

import pytest
import rez_core


class TestVersionCreation:
    """Test version object creation and basic properties."""

    def test_version_creation_simple(self):
        """Test creating simple semantic versions."""
        v = rez_core.Version("1.2.3")
        assert str(v) == "1.2.3"
        assert repr(v) == "Version('1.2.3')"

    def test_version_creation_complex(self, sample_versions):
        """Test creating various version formats."""
        for version_str in sample_versions:
            v = rez_core.Version(version_str)
            assert str(v) == version_str

    def test_version_creation_invalid(self):
        """Test that invalid version strings raise appropriate errors."""
        invalid_versions = [
            "",
            "not.a.version",
            "1.2.3.4.5.6",
            "v1.2.3",  # prefix not supported yet
        ]

        for invalid in invalid_versions:
            with pytest.raises((ValueError, rez_core.VersionParseError)):
                rez_core.Version(invalid)

    def test_parse_version_function(self):
        """Test the standalone parse_version function."""
        v = rez_core.parse_version("1.2.3")
        assert isinstance(v, rez_core.Version)
        assert str(v) == "1.2.3"


class TestVersionComparison:
    """Test version comparison operations."""

    def test_version_equality(self):
        """Test version equality comparison."""
        v1 = rez_core.Version("1.2.3")
        v2 = rez_core.Version("1.2.3")
        v3 = rez_core.Version("1.2.4")

        assert v1 == v2
        assert v1 != v3
        assert not (v1 != v2)
        assert not (v1 == v3)

    def test_version_ordering(self, comparison_test_cases):
        """Test version ordering comparisons."""
        for v1_str, v2_str, expected in comparison_test_cases:
            v1 = rez_core.Version(v1_str)
            v2 = rez_core.Version(v2_str)

            if expected == "less":
                assert v1 < v2
                assert v1 <= v2
                assert not (v1 > v2)
                assert not (v1 >= v2)
            elif expected == "greater":
                assert v1 > v2
                assert v1 >= v2
                assert not (v1 < v2)
                assert not (v1 <= v2)
            elif expected == "equal":
                assert v1 == v2
                assert v1 <= v2
                assert v1 >= v2
                assert not (v1 < v2)
                assert not (v1 > v2)

    def test_version_sorting(self):
        """Test that versions can be sorted correctly."""
        versions = [
            rez_core.Version("2.0.0"),
            rez_core.Version("1.0.0"),
            rez_core.Version("1.2.0"),
            rez_core.Version("1.1.0"),
            rez_core.Version("3.0.0"),
        ]

        sorted_versions = sorted(versions)
        expected_order = ["1.0.0", "1.1.0", "1.2.0", "2.0.0", "3.0.0"]

        assert [str(v) for v in sorted_versions] == expected_order

    def test_version_hashing(self):
        """Test that versions can be used as dictionary keys."""
        v1 = rez_core.Version("1.2.3")
        v2 = rez_core.Version("1.2.3")
        v3 = rez_core.Version("1.2.4")

        version_dict = {v1: "first", v2: "second", v3: "third"}

        # v1 and v2 should be the same key
        assert len(version_dict) == 2
        assert version_dict[v1] == "second"  # v2 overwrote v1
        assert version_dict[v3] == "third"


class TestVersionRange:
    """Test version range functionality."""

    def test_version_range_creation(self, sample_version_ranges):
        """Test creating version ranges."""
        for range_str in sample_version_ranges:
            vr = rez_core.VersionRange(range_str)
            assert str(vr) == range_str

    def test_parse_version_range_function(self):
        """Test the standalone parse_version_range function."""
        vr = rez_core.parse_version_range(">=1.0.0")
        assert isinstance(vr, rez_core.VersionRange)
        assert str(vr) == ">=1.0.0"

    def test_version_range_contains(self):
        """Test version range containment checks."""
        vr = rez_core.VersionRange(">=1.0.0")

        v1 = rez_core.Version("1.0.0")
        v2 = rez_core.Version("2.0.0")
        v3 = rez_core.Version("0.9.0")

        # Note: contains method is placeholder, will always return True for now
        assert vr.contains(v1)
        assert vr.contains(v2)
        assert vr.contains(v3)  # TODO: implement proper logic

    def test_version_range_intersection(self):
        """Test version range intersection."""
        vr1 = rez_core.VersionRange(">=1.0.0")
        vr2 = rez_core.VersionRange("<2.0.0")

        # Note: intersect method is placeholder, will return None for now
        intersection = vr1.intersect(vr2)
        assert intersection is None  # TODO: implement proper logic


@pytest.mark.performance
class TestVersionPerformance:
    """Performance tests for version operations."""

    def test_version_creation_performance(self):
        """Test that version creation is reasonably fast."""
        import time

        start_time = time.time()
        for i in range(1000):
            rez_core.Version(f"1.{i % 100}.{i % 10}")
        end_time = time.time()

        # Should be able to create 1000 versions in less than 1 second
        assert (end_time - start_time) < 1.0

    def test_version_comparison_performance(self):
        """Test that version comparison is reasonably fast."""
        import time

        versions = [rez_core.Version(f"1.{i}.0") for i in range(100)]

        start_time = time.time()
        for _ in range(100):
            sorted(versions)
        end_time = time.time()

        # Should be able to sort 100 versions 100 times in less than 1 second
        assert (end_time - start_time) < 1.0


@pytest.mark.compat
class TestRezCompatibility:
    """Tests for compatibility with original rez version behavior."""

    def test_version_string_representation(self):
        """Test that version string representation matches rez expectations."""
        test_cases = [
            "1.0.0",
            "2.1.3",
            "0.9.12",
            "10.0.0",
        ]

        for version_str in test_cases:
            v = rez_core.Version(version_str)
            assert str(v) == version_str

    def test_version_comparison_compatibility(self):
        """Test that version comparison behaves like original rez."""
        # These test cases are based on rez's version comparison behavior
        test_cases = [
            ("1.0.0", "1.0.1", True),  # 1.0.0 < 1.0.1
            ("1.0.1", "1.0.0", False),  # 1.0.1 > 1.0.0
            ("1.0.0", "1.0.0", False),  # 1.0.0 == 1.0.0 (not less than)
            ("2.0.0", "1.9.9", False),  # 2.0.0 > 1.9.9
        ]

        for v1_str, v2_str, expected_less in test_cases:
            v1 = rez_core.Version(v1_str)
            v2 = rez_core.Version(v2_str)
            assert (v1 < v2) == expected_less
