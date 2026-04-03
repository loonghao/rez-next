"""
Module-level tests for rez_next version API.

Covers: Version, VersionRange — parsing, comparison, arithmetic,
        edge cases, and rez semantic compatibility.
"""

import pytest

rez = pytest.importorskip(
    "rez_next",
    reason="rez_next not built — run: maturin develop --features extension-module",
)


class TestVersionParsing:
    """Version string parsing — rez semantic versioning."""

    @pytest.mark.parametrize(
        "ver_str",
        [
            "1.0.0",
            "2.1.3",
            "10.20.30",
            "0.0.1",
            "1",
            "1.2",
            "1.2.3.4",
            "1.0.0-alpha1",
            "3.2.1.post1",
        ],
    )
    def test_parse_valid(self, ver_str):
        v = rez.Version(ver_str)
        assert str(v) == ver_str

    @pytest.mark.parametrize("ver_str", ["1.2.3", "2.0.0", "0.1.0"])
    def test_repr_contains_version(self, ver_str):
        v = rez.Version(ver_str)
        assert ver_str in repr(v) or ver_str in str(v)


class TestVersionComparison:
    """Version ordering must match rez semantics."""

    def test_less_than(self):
        assert rez.Version("1.0.0") < rez.Version("2.0.0")

    def test_greater_than(self):
        assert rez.Version("2.0.0") > rez.Version("1.0.0")

    def test_equal(self):
        assert rez.Version("1.2.3") == rez.Version("1.2.3")

    def test_not_equal(self):
        assert rez.Version("1.2.3") != rez.Version("1.2.4")

    def test_le(self):
        assert rez.Version("1.0.0") <= rez.Version("1.0.0")
        assert rez.Version("1.0.0") <= rez.Version("2.0.0")

    def test_ge(self):
        assert rez.Version("2.0.0") >= rez.Version("2.0.0")
        assert rez.Version("2.0.0") >= rez.Version("1.0.0")

    def test_sort(self):
        versions = [rez.Version(v) for v in ["3.0", "1.0", "2.0", "1.5"]]
        sorted_v = sorted(versions)
        strs = [str(v) for v in sorted_v]
        assert strs == sorted(strs)

    def test_minor_patch_ordering(self):
        assert rez.Version("1.9.0") < rez.Version("1.10.0")
        assert rez.Version("1.2.9") < rez.Version("1.2.10")


class TestVersionRange:
    """VersionRange parsing and membership tests."""

    @pytest.mark.parametrize(
        "range_str,version,expected",
        [
            (">=1.0.0", "1.0.0", True),
            (">=1.0.0", "2.5.0", True),
            (">=1.0.0", "0.9.9", False),
            ("<2.0.0", "1.9.9", True),
            ("<2.0.0", "2.0.0", False),
            (">=1.0.0,<2.0.0", "1.5.0", True),
            (">=1.0.0,<2.0.0", "2.0.0", False),
            (">=1.0.0,<2.0.0", "0.9.0", False),
            ("1.0+<2.0", "1.5", True),
            ("", "99.99.99", True),  # empty range = any
        ],
    )
    def test_contains(self, range_str, version, expected):
        r = rez.VersionRange(range_str)
        v = rez.Version(version)
        assert r.contains(v) is expected

    def test_range_repr(self):
        r = rez.VersionRange(">=1.0.0,<2.0.0")
        assert r is not None

    def test_range_from_vendor(self):
        from rez_next.vendor.version import Version, VersionRange

        r = VersionRange(">=3.9")
        assert r.contains(Version("3.9"))
        assert r.contains(Version("3.11"))
        assert not r.contains(Version("3.8"))


class TestVersionEdgeCases:
    """Edge cases and boundary values."""

    def test_zero_version(self):
        v = rez.Version("0.0.0")
        assert str(v) == "0.0.0"

    def test_large_version_numbers(self):
        v = rez.Version("100.200.300")
        assert str(v) == "100.200.300"

    def test_single_component(self):
        v = rez.Version("5")
        assert str(v) == "5"

    def test_range_exact_version(self):
        r = rez.VersionRange("3.9")
        v = rez.Version("3.9")
        assert r.contains(v)


class TestVersionRangeClassMethods:
    """VersionRange.any(), none(), from_str(), as_str() classmethods."""

    def test_any_classmethod_exists(self):
        r = rez.VersionRange.any()
        assert r is not None

    def test_any_classmethod_matches_everything(self):
        r = rez.VersionRange.any()
        assert r.is_any()
        for ver in ["0.0.1", "1.0.0", "999.999.999", "1.2.3.4.5"]:
            assert r.contains(rez.Version(ver)), f"any() should contain {ver}"

    def test_none_classmethod_exists(self):
        r = rez.VersionRange.none()
        assert r is not None

    def test_none_classmethod_matches_nothing(self):
        r = rez.VersionRange.none()
        assert r.is_empty()
        for ver in ["0.0.1", "1.0.0", "99.0"]:
            assert not r.contains(rez.Version(ver)), f"none() should not contain {ver}"

    def test_from_str_static_method(self):
        r = rez.VersionRange.from_str(">=1.0,<2.0")
        assert r is not None
        assert r.contains(rez.Version("1.5"))
        assert not r.contains(rez.Version("2.0"))

    def test_from_str_empty_string_gives_any(self):
        r = rez.VersionRange.from_str("")
        assert r.is_any()

    def test_as_str_method(self):
        r = rez.VersionRange(">=1.0,<2.0")
        assert r.as_str() == ">=1.0,<2.0"

    def test_as_str_any_range(self):
        r = rez.VersionRange.any()
        # any range string may be "" or "*"
        s = r.as_str()
        assert isinstance(s, str)

    def test_any_union_identity(self):
        """any() union anything should remain any."""
        any_r = rez.VersionRange.any()
        r = rez.VersionRange(">=3.0")
        union_r = any_r.union(r)
        assert union_r.is_any()
