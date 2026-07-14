"""Tests for rez_next.utils.data_utils module."""

from __future__ import annotations

import json

import pytest
from rez_next.utils.data_utils import (
    ModifyList,
    DelayLoad,
    remove_nones,
    deep_update,
    deep_del,
    get_dict_diff,
    get_dict_diff_str,
    cached_property,
    cached_class_property,
    LazySingleton,
    AttrDictWrapper,
    RO_AttrDictWrapper,
    convert_dicts,
    convert_json_safe,
    get_object_completions,
    AttributeForwardMeta,
)


# ── ModifyList ───────────────────────────────────────────────────────────────

class TestModifyList:
    """Tests for ModifyList class."""

    def test_append_only(self):
        ml = ModifyList(append=["c", "d"])
        assert ml.apply(["a", "b"]) == ["a", "b", "c", "d"]

    def test_prepend_only(self):
        ml = ModifyList(prepend=["a", "b"])
        assert ml.apply(["c", "d"]) == ["a", "b", "c", "d"]

    def test_both_prepend_and_append(self):
        ml = ModifyList(prepend=["a"], append=["d"])
        assert ml.apply(["b", "c"]) == ["a", "b", "c", "d"]

    def test_none_input(self):
        ml = ModifyList(append=["x"])
        assert ml.apply(None) == ["x"]

    def test_invalid_prepend_type(self):
        with pytest.raises(ValueError, match="Expected list"):
            ModifyList(prepend="not_a_list")  # type: ignore

    def test_invalid_append_type(self):
        with pytest.raises(ValueError, match="Expected list"):
            ModifyList(append=123)  # type: ignore

    def test_apply_to_non_list_raises(self):
        ml = ModifyList(append=["x"])
        with pytest.raises(ValueError, match="non-list"):
            ml.apply("string")  # type: ignore


# ── DelayLoad ────────────────────────────────────────────────────────────────

class TestDelayLoad:
    """Tests for DelayLoad class."""

    def test_str_representation(self):
        dl = DelayLoad("/some/path/config.yaml")
        assert "DelayLoad" in str(dl)
        assert "config.yaml" in str(dl)

    def test_expands_user(self):
        dl = DelayLoad("~/test.json")
        assert "~" not in dl.filepath

    def test_get_value_file_not_found(self):
        dl = DelayLoad("/nonexistent/file.yaml")
        with pytest.raises(ValueError, match="unsupported file format"):
            dl.get_value()

    def test_unsupported_format(self):
        dl = DelayLoad("/tmp/file.txt")
        with pytest.raises(ValueError, match="unsupported file format"):
            dl.get_value()


# ── remove_nones ─────────────────────────────────────────────────────────────

class TestRemoveNones:
    """Tests for remove_nones function."""

    def test_removes_none_values(self):
        result = remove_nones(a=1, b=None, c=3)
        assert result == {"a": 1, "c": 3}

    def test_empty_with_all_nones(self):
        result = remove_nones(a=None, b=None)
        assert result == {}

    def test_no_nones_returns_all(self):
        result = remove_nones(a=1, b=2)
        assert result == {"a": 1, "b": 2}


# ── deep_update ──────────────────────────────────────────────────────────────

class TestDeepUpdate:
    """Tests for deep_update function."""

    def test_simple_merge(self):
        d1 = {"a": 1, "b": 2}
        d2 = {"b": 3, "c": 4}
        deep_update(d1, d2)
        assert d1 == {"a": 1, "b": 3, "c": 4}

    def test_nested_merge(self):
        d1 = {"a": {"x": 1, "y": 2}}
        d2 = {"a": {"y": 3, "z": 4}}
        deep_update(d1, d2)
        assert d1 == {"a": {"x": 1, "y": 3, "z": 4}}

    def test_modify_list_append(self):
        d1 = {"items": ["a", "b"]}
        d2 = {"items": ModifyList(append=["c"])}
        deep_update(d1, d2)
        assert d1 == {"items": ["a", "b", "c"]}

    def test_modify_list_prepend(self):
        d1 = {"items": ["b", "c"]}
        d2 = {"items": ModifyList(prepend=["a"])}
        deep_update(d1, d2)
        assert d1 == {"items": ["a", "b", "c"]}

    def test_new_key_from_dict2(self):
        d1 = {"a": 1}
        d2 = {"b": 2}
        deep_update(d1, d2)
        assert d1 == {"a": 1, "b": 2}

    def test_dict2_not_mutated(self):
        d1 = {"a": {"x": 1}}
        d2 = {"a": {"y": 2}}
        deep_update(d1, d2)
        assert d2 == {"a": {"y": 2}}


# ── deep_del ─────────────────────────────────────────────────────────────────

class TestDeepDel:
    """Tests for deep_del function."""

    def test_remove_none_values(self):
        data = {"a": 1, "b": None, "c": 3}
        result = deep_del(data, lambda v: v is None)
        assert result == {"a": 1, "c": 3}

    def test_remove_nested_none(self):
        data = {"a": {"x": None, "y": 2}}
        result = deep_del(data, lambda v: v is None)
        assert result == {"a": {"y": 2}}

    def test_no_match_returns_copy(self):
        data = {"a": 1, "b": 2}
        result = deep_del(data, lambda v: False)
        assert result == data
        assert result is not data

    def test_original_not_mutated(self):
        data = {"a": None, "b": 2}
        deep_del(data, lambda v: v is None)
        assert "a" in data  # original still has 'a'


# ── get_dict_diff ────────────────────────────────────────────────────────────

class TestGetDictDiff:
    """Tests for get_dict_diff and get_dict_diff_str functions."""

    def test_identical_dicts(self):
        added, removed, changed = get_dict_diff({"a": 1}, {"a": 1})
        assert added == []
        assert removed == []
        assert changed == []

    def test_added_key(self):
        added, removed, changed = get_dict_diff({"a": 1}, {"a": 1, "b": 2})
        assert added == [["b"]]
        assert removed == []
        assert changed == []

    def test_removed_key(self):
        added, removed, changed = get_dict_diff({"a": 1, "b": 2}, {"a": 1})
        assert added == []
        assert removed == [["b"]]
        assert changed == []

    def test_changed_value(self):
        added, removed, changed = get_dict_diff({"a": 1}, {"a": 2})
        assert added == []
        assert removed == []
        assert changed == [["a"]]

    def test_nested_change(self):
        added, removed, changed = get_dict_diff(
            {"a": {"x": 1}},
            {"a": {"x": 2}},
        )
        assert changed == [["a", "x"]]

    def test_get_dict_diff_str_title(self):
        result = get_dict_diff_str({"a": 1}, {"a": 2}, "Diff:")
        assert "Diff:" in result
        assert "Changed" in result

    def test_get_dict_diff_str_no_changes(self):
        result = get_dict_diff_str({"a": 1}, {"a": 1}, "No diff:")
        assert result == "No diff:"


# ── cached_property ──────────────────────────────────────────────────────────

class TestCachedProperty:
    """Tests for cached_property descriptor."""

    def test_basic_caching(self):
        call_count = 0

        class Foo:
            @cached_property
            def value(self):
                nonlocal call_count
                call_count += 1
                return 42

        f = Foo()
        assert f.value == 42
        assert call_count == 1
        assert f.value == 42  # second access, should use cache
        assert call_count == 1

    def test_instance_independence(self):
        class Foo:
            @cached_property
            def value(self):
                return id(self)

        f1 = Foo()
        f2 = Foo()
        assert f1.value != f2.value

    def test_uncache(self):
        call_count = 0

        class Foo:
            @cached_property
            def value(self):
                nonlocal call_count
                call_count += 1
                return 42

        f = Foo()
        _ = f.value
        assert call_count == 1
        cached_property.uncache(f, "value")
        _ = f.value  # should recalculate
        assert call_count == 2

    def test_class_access(self):
        class Foo:
            @cached_property
            def value(self):
                return 1

        # Accessing on the class returns the descriptor
        desc = Foo.value  # type: ignore
        assert desc is not None


# ── cached_class_property ────────────────────────────────────────────────────

class TestCachedClassProperty:
    """Tests for cached_class_property descriptor."""

    def test_basic_caching(self):
        call_count = 0

        class Foo:
            @cached_class_property
            def value(cls):
                nonlocal call_count
                call_count += 1
                return "class_value"

        assert Foo.value == "class_value"  # type: ignore
        assert call_count == 1
        assert Foo.value == "class_value"  # type: ignore
        assert call_count == 1


# ── LazySingleton ────────────────────────────────────────────────────────────

class TestLazySingleton:
    """Tests for LazySingleton class."""

    def test_singleton_returns_same_instance(self):
        class MyClass:
            pass

        ls = LazySingleton(MyClass)
        instance1 = ls()
        instance2 = ls()
        assert instance1 is instance2

    def test_lazy_initialization(self):
        created = False

        class MyClass:
            def __init__(self):
                nonlocal created
                created = True

        ls = LazySingleton(MyClass)
        assert not created  # not yet created
        _ = ls()
        assert created

    def test_constructor_args(self):
        class MyClass:
            def __init__(self, x, y):
                self.x = x
                self.y = y

        ls = LazySingleton(MyClass, 1, y=2)
        instance = ls()
        assert instance.x == 1
        assert instance.y == 2


# ── AttrDictWrapper ──────────────────────────────────────────────────────────

class TestAttrDictWrapper:
    """Tests for AttrDictWrapper class."""

    def test_attribute_access(self):
        d = AttrDictWrapper({"one": 1, "two": 2})
        assert d.one == 1
        assert d.two == 2

    def test_attribute_set(self):
        d = AttrDictWrapper()
        d.key = "value"
        assert d.key == "value"
        assert d["key"] == "value"

    def test_item_access(self):
        d = AttrDictWrapper({"a": 1})
        assert d["a"] == 1

    def test_item_set(self):
        d = AttrDictWrapper()
        d["key"] = "val"
        assert d.key == "val"

    def test_contains(self):
        d = AttrDictWrapper({"a": 1})
        assert "a" in d
        assert "b" not in d

    def test_len(self):
        d = AttrDictWrapper({"a": 1, "b": 2})
        assert len(d) == 2

    def test_iter(self):
        d = AttrDictWrapper({"a": 1, "b": 2})
        assert set(iter(d)) == {"a", "b"}

    def test_delitem(self):
        d = AttrDictWrapper({"a": 1, "b": 2})
        del d["a"]
        assert "a" not in d

    def test_missing_attr_raises(self):
        d = AttrDictWrapper({"a": 1})
        with pytest.raises(AttributeError):
            _ = d.nonexistent

    def test_copy(self):
        d = AttrDictWrapper({"a": 1})
        d2 = d.copy()
        d2.a = 2
        assert d.a == 1
        assert d2.a == 2

    def test_str_repr(self):
        d = AttrDictWrapper({"a": 1})
        assert "a" in str(d)
        assert "AttrDictWrapper" in repr(d)


class TestRO_AttrDictWrapper:
    """Tests for RO_AttrDictWrapper class."""

    def test_read_access(self):
        d = RO_AttrDictWrapper({"a": 1})
        assert d.a == 1

    def test_write_raises(self):
        d = RO_AttrDictWrapper({"a": 1})
        with pytest.raises(AttributeError, match="read-only"):
            d.a = 2

    def test_new_attr_raises(self):
        d = RO_AttrDictWrapper({"a": 1})
        with pytest.raises(AttributeError):
            d.new_attr = 3


# ── convert_dicts ────────────────────────────────────────────────────────────

class TestConvertDicts:
    """Tests for convert_dicts function."""

    def test_convert_to_attrdictwrapper(self):
        d = {"a": 1, "b": 2}
        result = convert_dicts(d)
        assert isinstance(result, AttrDictWrapper)
        assert result.a == 1

    def test_nested_conversion(self):
        d = {"outer": {"inner": 1}}
        result = convert_dicts(d)
        assert isinstance(result.outer, AttrDictWrapper)
        assert result.outer.inner == 1

    def test_original_not_mutated(self):
        d = {"a": {"x": 1}}
        result = convert_dicts(d)
        assert isinstance(result.a, AttrDictWrapper)
        assert isinstance(d["a"], dict)  # original unchanged


# ── convert_json_safe ────────────────────────────────────────────────────────

class TestConvertJsonSafe:
    """Tests for convert_json_safe function."""

    def test_json_safe_int(self):
        assert convert_json_safe(42) == 42

    def test_json_safe_string(self):
        assert convert_json_safe("hello") == "hello"

    def test_json_safe_list(self):
        result = convert_json_safe([1, 2, 3])
        assert result == [1, 2, 3]

    def test_json_safe_dict(self):
        result = convert_json_safe({"a": 1})
        assert result == {"a": 1}

    def test_non_serializable_converted_to_str(self):
        class Custom:
            def __str__(self):
                return "custom_str"

        result = convert_json_safe(Custom())
        assert result == "custom_str"

    def test_mixed_list_with_non_serializable(self):
        class Custom:
            def __str__(self):
                return "obj"

        result = convert_json_safe([1, Custom()])
        assert result == [1, "obj"]


# ── get_object_completions ───────────────────────────────────────────────────

class TestGetObjectCompletions:
    """Tests for get_object_completions function."""

    def test_basic_attributes(self):
        class Obj:
            def __init__(self):
                self.alpha = 1
                self.beta = 2

        result = get_object_completions(Obj(), "a")
        assert "alpha" in result
        assert "beta" not in result

    def test_no_match(self):
        class Obj:
            def __init__(self):
                self.zzz = 1

        result = get_object_completions(Obj(), "a")
        assert result == []

    def test_filter_by_type(self):
        class Obj:
            def __init__(self):
                self.str_val = "hello"
                self.int_val = 42

        result = get_object_completions(Obj(), "s", types=(str,))
        assert "str_val" in result


# ── AttributeForwardMeta ─────────────────────────────────────────────────────

class TestAttributeForwardMeta:
    """Tests for AttributeForwardMeta metaclass."""

    def test_forward_attributes(self):
        class Child:
            def __init__(self):
                self.a = "child_a"
                self.b = "child_b"

        class Parent(metaclass=AttributeForwardMeta):
            keys = ["a", "b"]

            def __init__(self, child):
                self.wrapped = child

        child = Child()
        parent = Parent(child)
        assert parent.a == "child_a"
        assert parent.b == "child_b"

    def test_existing_attr_not_overridden(self):
        class Child:
            def __init__(self):
                self.a = "child_a"

        class Parent(metaclass=AttributeForwardMeta):
            keys = ["a"]

            @property
            def a(self):
                return "parent_a"

            def __init__(self, child):
                self.wrapped = child

        child = Child()
        parent = Parent(child)
        assert parent.a == "parent_a"

    def test_missing_attr_returns_none(self):
        class Child:
            pass

        class Parent(metaclass=AttributeForwardMeta):
            keys = ["missing"]

            def __init__(self, child):
                self.wrapped = child

        parent = Parent(Child())
        assert parent.missing is None

    def test_wrap_forwarded_hook(self):
        class Child:
            def __init__(self):
                self.a = 1

        class Parent(metaclass=AttributeForwardMeta):
            keys = ["a"]

            @staticmethod
            def _wrap_forwarded(key, value):
                return value * 2

            def __init__(self, child):
                self.wrapped = child

        parent = Parent(Child())
        assert parent.a == 2
