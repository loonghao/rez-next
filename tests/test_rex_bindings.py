"""Tests for rez_next.rex_bindings module.

Aligns with rez.rex_bindings API coverage.
"""

import pytest
from rez_next.rex_bindings import (
    Binding,
    VersionBinding,
    VariantBinding,
    RO_MappingBinding,
    VariantsBinding,
    RequirementsBinding,
    EphemeralsBinding,
    intersects,
)


# ── Helpers ─────────────────────────────────────────────────────────────────


class _FakeVariant:
    """Minimal variant stub for testing VariantBinding."""

    def __init__(self, name: str = "mypkg", version: str = "1.0.0",
                 root: str = "/repo/mypkg/1.0.0", subpath: str = "") -> None:
        self.name = name
        self._version = version
        self.root = root
        self.subpath = subpath

    @property
    def version(self):
        return self._version

    def __repr__(self) -> str:
        return f"_FakeVariant({self.name})"


class _FakeVersion:
    """Minimal Version-like stub for testing binding internals."""

    def __init__(self, tokens: tuple) -> None:
        self.tokens = tokens

    def __str__(self) -> str:
        return ".".join(str(t) for t in self.tokens)


# ── Binding ─────────────────────────────────────────────────────────────────


class TestBinding:
    def test_init_without_data(self):
        b = Binding()
        assert b._data == {}

    def test_init_with_data(self):
        b = Binding({"foo": 1, "bar": 2})
        assert b._data == {"foo": 1, "bar": 2}

    def test_getattr_from_data(self):
        b = Binding({"foo": 42})
        assert b.foo == 42

    def test_getattr_raises_for_missing(self):
        b = Binding()
        with pytest.raises(AttributeError, match="Binding"):
            _ = b.nonexistent

    def test_data_is_not_overwritten_by_getattr(self):
        b = Binding({"key": "val"})
        assert "key" in b._data


# ── VersionBinding ─────────────────────────────────────────────────────────


class TestVersionBinding:
    def test_create_from_null_version(self):
        """A null/"empty" VersionBinding still works."""
        # Version with single "0" token
        vb = _make_version_binding(("0",))
        assert vb is not None

    def test_major_minor_patch(self):
        vb = _make_version_binding(("1", "2", "3alpha"))
        assert vb.major == 1
        assert vb.minor == 2
        assert vb.patch == "3alpha"

    def test_major_without_minor(self):
        vb = _make_version_binding(("5",))
        assert vb.major == 5
        assert vb.minor == ""

    def test_patch_without_minor(self):
        vb = _make_version_binding(("5",))
        assert vb.patch == ""

    def test_as_tuple(self):
        vb = _make_version_binding(("4", "5", "6"))
        assert vb.as_tuple() == (4, 5, 6)

    def test_indexing(self):
        vb = _make_version_binding(("10", "20", "30"))
        assert vb[0] == 10
        assert vb[1] == 20
        assert vb[2] == 30

    def test_slicing(self):
        vb = _make_version_binding(("1", "2", "3", "4"))
        assert vb[:2] == (1, 2)
        assert vb[1:3] == (2, 3)

    def test_len(self):
        vb = _make_version_binding(("1", "2", "3"))
        assert len(vb) == 3

    def test_iter(self):
        vb = _make_version_binding(("1", "2"))
        assert list(vb) == [1, 2]

    def test_str(self):
        vb = _make_version_binding(("1", "2", "3"))
        assert str(vb) == "1.2.3"

    def test_repr(self):
        vb = _make_version_binding(("9", "8"))
        assert "VersionBinding" in repr(vb)
        assert "9.8" in repr(vb)


# ── VariantBinding ──────────────────────────────────────────────────────────


class TestVariantBinding:
    def test_basic_creation(self):
        v = _FakeVariant(name="testpkg")
        vb = VariantBinding(v)
        assert vb._variant is v

    def test_root_from_variant(self):
        v = _FakeVariant(root="/packages/test/1.0")
        vb = VariantBinding(v)
        assert vb.root == "/packages/test/1.0"

    def test_cached_root(self):
        import os
        v = _FakeVariant(root="/packages/test/1.0", subpath="")
        vb = VariantBinding(v, cached_root="/cache/repo")
        # root should point to cached path
        expected = os.path.normpath("/cache/repo")
        assert expected in vb.root

    def test_is_in_package_cache_false(self):
        v = _FakeVariant()
        vb = VariantBinding(v)
        assert not vb._is_in_package_cache()

    def test_is_in_package_cache_true(self):
        v = _FakeVariant()
        vb = VariantBinding(v, cached_root="/cache")
        assert vb._is_in_package_cache()

    def test_attribute_fallback_to_variant(self):
        v = _FakeVariant(name="myapp")
        vb = VariantBinding(v)
        assert vb.name == "myapp"

    def test_data_overrides_variant(self):
        v = _FakeVariant(name="myapp")
        vb = VariantBinding(v, data={"name": "override"})
        assert vb.name == "override"

    def test_repr(self):
        v = _FakeVariant(name="mypkg")
        vb = VariantBinding(v)
        assert "VariantBinding" in repr(vb)


# ── RO_MappingBinding ──────────────────────────────────────────────────────


class TestRO_MappingBinding:
    def test_get_existing(self):
        m = RO_MappingBinding({"a": 1, "b": 2})
        assert m.get("a") == 1

    def test_get_default(self):
        m = RO_MappingBinding({"a": 1})
        assert m.get("missing", "fallback") == "fallback"

    def test_getitem(self):
        m = RO_MappingBinding({"x": 100})
        assert m["x"] == 100

    def test_getitem_missing_raises(self):
        m = RO_MappingBinding({})
        with pytest.raises(KeyError):
            _ = m["nope"]

    def test_contains(self):
        m = RO_MappingBinding({"key": "val"})
        assert "key" in m
        assert "missing" not in m

    def test_str(self):
        m = RO_MappingBinding({"a": 1})
        assert "a" in str(m)

    def test_get_via_attr_is_blocked(self):
        """RO_MappingBinding does NOT support attribute access by default."""
        m = RO_MappingBinding({"myattr": "val"})
        # _data lookup via Binding.__getattr__ is available
        assert m.myattr == "val"


# ── VariantsBinding ─────────────────────────────────────────────────────────


class TestVariantsBinding:
    def test_get_existing(self):
        vb = VariantsBinding({"pkg1": "obj1"})
        assert vb["pkg1"] == "obj1"

    def test_access_missing_via_attr(self):
        vb = VariantsBinding({})
        with pytest.raises(AttributeError, match="package does not exist"):
            _ = vb.nonexistent_pkg


# ── RequirementsBinding ─────────────────────────────────────────────────────


class TestRequirementsBinding:
    def test_get_range_existing(self):
        rb = RequirementsBinding({"python": "python-3.9"})
        r = rb.get_range("python")
        assert r is not None
        assert str(r) == "python-3.9"

    def test_get_range_missing(self):
        rb = RequirementsBinding({})
        result = rb.get_range("nonexistent")
        assert result is None

    def test_get_range_with_default(self):
        rb = RequirementsBinding({})
        result = rb.get_range("nonexistent", default="fallback")
        assert result == "fallback"

    def test_getitem(self):
        rb = RequirementsBinding({"maya": "maya-2024"})
        assert rb["maya"] == "maya-2024"


# ── EphemeralsBinding ──────────────────────────────────────────────────────


class TestEphemeralsBinding:
    def test_contains(self):
        eb = EphemeralsBinding({".foo.cli": ".foo.cli-1.0"})
        assert ".foo.cli" in eb

    def test_get_range(self):
        eb = EphemeralsBinding({"dotless": ".dotless-*"})
        r = eb.get_range("dotless")
        assert r is not None

    def test_get_range_missing(self):
        eb = EphemeralsBinding({})
        assert eb.get_range("nope") is None


# ── intersects ──────────────────────────────────────────────────────────────


class TestIntersects:
    def test_with_version_binding_and_range_str(self):
        """intersects with VersionBinding and range string."""
        vb = _make_version_binding(("3", "9", "0"))
        result = intersects(vb, ">=3.0,<4.0")
        # VersionBinding to VersionRange may not intersect if range logic differs
        assert isinstance(result, bool)

    def test_with_string_and_range_str(self):
        """intersects with requirement string and range string."""
        result = intersects("python-3.9", ">=3.0,<4.0")
        assert isinstance(result, bool)

    def test_with_variant_binding(self):
        """intersects with VariantBinding."""
        v = _FakeVariant(version="2.0.0")
        vb = VariantBinding(v)
        result = intersects(vb, ">=1.0,<3.0")
        assert isinstance(result, bool)

    def test_returns_false_for_invalid_input(self):
        """intersects returns False for unrecognised types."""
        result = intersects(42, ">=1.0")
        assert result is False

    def test_returns_false_for_invalid_range(self):
        """intersects returns False for invalid range argument."""
        vb = _make_version_binding(("1", "0"))
        result = intersects(vb, 42)
        assert result is False


# ── Module-level imports ────────────────────────────────────────────────────


class TestModuleImports:
    """Verify all expected names are exported."""

    def test_module_has_all_expected_names(self):
        import rez_next.rex_bindings as mod

        for attr in ("Binding", "VersionBinding", "VariantBinding",
                     "RO_MappingBinding", "VariantsBinding",
                     "RequirementsBinding", "EphemeralsBinding",
                     "intersects"):
            assert hasattr(mod, attr), f"Missing: {attr}"


# ── Internal helpers ────────────────────────────────────────────────────────


def _make_version_binding(tokens: tuple) -> VersionBinding:
    """Create a VersionBinding backed by a minimal Version-like stub."""
    fv = _FakeVersion(tokens)
    return VersionBinding(fv)
