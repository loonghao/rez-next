"""
Data manipulation utilities for Rez-next.

Mirrors ``rez.utils.data_utils`` public API:

- ``ModifyList``, ``DelayLoad``
- ``deep_update``, ``deep_del``, ``remove_nones``
- ``get_dict_diff``, ``get_dict_diff_str``
- ``cached_property``, ``cached_class_property``
- ``LazySingleton``
- ``AttrDictWrapper``, ``RO_AttrDictWrapper``
- ``convert_dicts``, ``convert_json_safe``
- ``get_object_completions``
- ``AttributeForwardMeta``, ``LazyAttributeMeta``

Design decisions:
- Uses ``functools`` and ``threading.Lock`` instead of vendored Rez code.
- ``LazySingleton`` is adapted from the local copy in ``plugin_managers.py``
  but placed here to align with the Rez public API.
"""
from __future__ import annotations

import functools
import json
import os.path
from threading import Lock
from typing import Any, Callable, Generic, MutableMapping, TYPE_CHECKING, TypeVar

T = TypeVar("T")


# ── List modifier ────────────────────────────────────────────────────────────

class ModifyList:
    """List modifier, used in ``deep_update``.

    This can be used in configs to add to list-based settings, rather than
    overwriting them.
    """

    def __init__(self, append: list | None = None, prepend: list | None = None) -> None:
        for v in (prepend, append):
            if v is not None and not isinstance(v, list):
                raise ValueError(f"Expected list in ModifyList, not {v!r}")
        self.prepend = prepend
        self.append = append

    def apply(self, v: list | None) -> list:
        """Apply prepend/append to *v*."""
        if v is None:
            v = []
        elif not isinstance(v, list):
            raise ValueError(f"Attempted to apply ModifyList to non-list: {v!r}")
        return (self.prepend or []) + v + (self.append or [])


# ── Delayed loading ──────────────────────────────────────────────────────────

class DelayLoad:
    """Used in config to delay-load a value from another file.

    Supported format: JSON (``*.json``).
    """

    def __init__(self, filepath: str) -> None:
        self.filepath = os.path.expanduser(filepath)

    def __str__(self) -> str:
        return f"{self.__class__.__name__}({self.filepath})"

    def get_value(self) -> Any:
        """Load and return the value from the configured file."""
        ext = os.path.splitext(self.filepath)[-1]

        if ext == ".json":

            def _loader(contents: str) -> Any:
                return json.loads(contents)

        else:
            raise ValueError(
                f"Error in DelayLoad - unsupported file format {self.filepath}"
            )

        try:
            with open(self.filepath) as f:
                contents = f.read()
        except Exception as e:
            raise ValueError(
                f"Error reading {self}: {e.__class__.__name__}: {e}"
            )

        try:
            return _loader(contents)
        except Exception as e:
            raise ValueError(
                f"Error loading from {self}: {e.__class__.__name__}: {e}"
            )


# ── Dict helpers ─────────────────────────────────────────────────────────────

def remove_nones(**kwargs) -> dict:
    """Return a dict copy with ``None`` values removed."""
    return {k: v for k, v in kwargs.items() if v is not None}


def deep_update(dict1: dict, dict2: dict) -> None:
    """Perform a deep merge of *dict2* into *dict1*.

    Note that *dict2* and any nested dicts are unchanged.  Supports
    ``ModifyList`` instances.
    """
    def _flatten(v: Any) -> Any:
        if isinstance(v, ModifyList):
            return v.apply([])
        elif isinstance(v, dict):
            return {k: _flatten(v_) for k, v_ in v.items()}
        return v

    def _merge(v1: Any, v2: Any) -> Any:
        if isinstance(v1, dict) and isinstance(v2, dict):
            deep_update(v1, v2)
            return v1
        elif isinstance(v2, ModifyList):
            return v2.apply(v1 if isinstance(v1, list) else _flatten(v1))
        return _flatten(v2)

    for k1, v1 in list(dict1.items()):
        if k1 not in dict2:
            dict1[k1] = _flatten(v1)

    for k2, v2 in dict2.items():
        v1 = dict1.get(k2)
        if v1 is None:
            dict1[k2] = _flatten(v2)
        else:
            dict1[k2] = _merge(v1, v2)


def deep_del(data: dict, fn: Callable[[Any], bool]) -> dict:
    """Create a dict copy with items removed where *fn(value)* is ``True``.

    Recurses into nested dicts.

    Returns:
        New dict with matching items removed.
    """
    result: dict = {}
    for k, v in data.items():
        if not fn(v):
            if isinstance(v, dict):
                result[k] = deep_del(v, fn)
            else:
                result[k] = v
    return result


def get_dict_diff(d1: dict, d2: dict) -> tuple[list[list[str]], list[list[str]], list[list[str]]]:
    """Get added / removed / changed keys between two dicts.

    Each key in the return value is a list representing the namespaced key
    path that was affected.

    Returns:
        3-tuple ``(added, removed, changed)`` where each element is a list
        of key-path lists.
    """
    def _diff(d1_: dict, d2_: dict, namespace: list[str]):
        added: list[list[str]] = []
        removed: list[list[str]] = []
        changed: list[list[str]] = []

        for k1, v1 in d1_.items():
            if k1 not in d2_:
                removed.append(namespace + [k1])
            else:
                v2 = d2_[k1]
                if v2 != v1:
                    if isinstance(v1, dict) and isinstance(v2, dict):
                        ns = namespace + [k1]
                        a, r, c = _diff(v1, v2, ns)
                        added.extend(a)
                        removed.extend(r)
                        changed.extend(c)
                    else:
                        changed.append(namespace + [k1])

        for k2 in d2_:
            if k2 not in d1_:
                added.append(namespace + [k2])

        return added, removed, changed

    return _diff(d1, d2, [])


def get_dict_diff_str(d1: dict, d2: dict, title: str) -> str:
    """Return a human-readable string describing dict differences.

    Same output as ``get_dict_diff()`` but formatted as text.
    """
    added, removed, changed = get_dict_diff(d1, d2)
    lines = [title]

    if added:
        lines.append("Added attributes: %s" % ['.'.join(x) for x in added])
    if removed:
        lines.append("Removed attributes: %s" % ['.'.join(x) for x in removed])
    if changed:
        lines.append("Changed attributes: %s" % ['.'.join(x) for x in changed])

    return '\n'.join(lines)


# ── Caching descriptors ──────────────────────────────────────────────────────

if TYPE_CHECKING:
    cached_property = property  # type: ignore[assignment,misc]
else:
    class cached_property:  # type: ignore[no-redef]
        """Simple instance-level property caching descriptor.

        Example::

            >>> class Foo:
            ...     @cached_property
            ...     def bah(self):
            ...         print('bah')
            ...         return 1
            ...
            >>> f = Foo()
            >>> f.bah
            bah
            1
            >>> f.bah
            1
        """

        def __init__(self, func: Callable, name: str | None = None) -> None:
            self.func = func
            functools.update_wrapper(self, func)
            self.name = name or func.__name__

        def __get__(self, instance: Any, owner: type | None = None) -> Any:
            if instance is None:
                return self
            result = self.func(instance)
            try:
                setattr(instance, self.name, result)
            except AttributeError:
                raise AttributeError(
                    f"can't set attribute {self.name!r} on {instance!r}"
                )
            return result

        def __call__(self) -> None:
            raise RuntimeError("@cached_property should not be called.")

        @classmethod
        def uncache(cls, instance: Any, name: str) -> None:
            """Remove the cached value for *name* on *instance*."""
            if hasattr(instance, name):
                delattr(instance, name)


class cached_class_property(Generic[T]):
    """Simple class-level property caching descriptor.

    Example::

        >>> class Foo:
        ...     @cached_class_property
        ...     def bah(cls):
        ...         print('bah')
        ...         return 1
        ...
        >>> Foo.bah
        bah
        1
        >>> Foo.bah
        1
    """

    def __init__(self, func: Callable[[Any], T], name: str | None = None) -> None:
        self.func = func
        functools.update_wrapper(self, func)  # type: ignore[arg-type]

    def __get__(self, instance: Any, owner: type | None = None) -> T:
        assert owner
        name = "_class_property_" + self.func.__name__
        result = getattr(owner, name, _SENTINEL)
        if result is _SENTINEL:
            result = self.func(owner)
            setattr(owner, name, result)
        return result  # type: ignore[return-value]


_SENTINEL = object()


class LazySingleton(Generic[T]):
    """A threadsafe singleton that initialises when first referenced."""

    def __init__(self, instance_class: type[T], *args: Any, **kwargs: Any) -> None:
        self.instance_class = instance_class
        self._args = args
        self._kwargs = kwargs
        self._lock = Lock()
        self._instance: T | None = None

    def __call__(self) -> T:
        if self._instance is None:
            with self._lock:
                if self._instance is None:
                    self._instance = self.instance_class(*self._args, **self._kwargs)
                    self._args = ()
                    self._kwargs = {}
        return self._instance


# ── AttrDictWrapper ──────────────────────────────────────────────────────────

class AttrDictWrapper(MutableMapping[str, Any]):
    """Wrap a dictionary with attribute-based lookup::

        >>> d = {'one': 1}
        >>> dd = AttrDictWrapper(d)
        >>> assert dd.one == 1
        >>> ddd = dd.copy()
        >>> ddd.one = 2
        >>> assert ddd.one == 2
        >>> assert dd.one == 1
        >>> assert d['one'] == 1
    """

    def __init__(self, data: dict | None = None) -> None:
        self.__dict__['_data'] = {} if data is None else data

    @property
    def _data(self) -> dict:
        return self.__dict__['_data']

    def __getattr__(self, attr: str) -> Any:
        if attr.startswith('__') and attr.endswith('__'):
            d = self.__dict__
        else:
            d = self._data
        try:
            return d[attr]
        except KeyError:
            raise AttributeError(
                f"'{self.__class__.__name__}' object has no attribute '{attr}'"
            )

    def __setattr__(self, attr: str, value: Any) -> None:
        if attr.startswith('__') and attr.endswith('__'):
            super().__setattr__(attr, value)
        self._data[attr] = value

    def __getitem__(self, key: str) -> Any:
        return self._data[key]

    def __setitem__(self, key: str, value: Any) -> None:
        self._data[key] = value

    def __delitem__(self, key: str) -> None:
        del self._data[key]

    def __contains__(self, key: object) -> bool:
        return key in self._data

    def __iter__(self):
        return iter(self._data)

    def __len__(self) -> int:
        return len(self._data)

    def __str__(self) -> str:
        return str(self._data)

    def __repr__(self) -> str:
        return f"{self.__class__.__name__}({self._data!r})"

    def copy(self) -> AttrDictWrapper:
        return self.__class__(self._data.copy())


class RO_AttrDictWrapper(AttrDictWrapper):
    """Read-only version of ``AttrDictWrapper``.

    Setting attributes raises ``AttributeError``.
    """

    def __setattr__(self, attr: str, value: Any) -> None:
        # Skip _data and dunder attributes (Python internals)
        if attr == '_data' or (attr.startswith('__') and attr.endswith('__')):
            super().__setattr__(attr, value)
            return
        if attr in self._data:
            raise AttributeError(
                f"'{self.__class__.__name__}' object attribute "
                f"'{attr}' is read-only"
            )
        raise AttributeError(
            f"'{self.__class__.__name__}' object has no attribute '{attr}'"
        )


# ── Conversion helpers ───────────────────────────────────────────────────────

def convert_dicts(
    d: dict,
    to_class: type = AttrDictWrapper,
    from_class: type = dict,
) -> Any:
    """Recursively convert dict types from *from_class* to *to_class*.

    Note that *d* is unchanged.

    Args:
        to_class: Dict-like type to convert values to.
        from_class: Dict-like type to convert values from. If a tuple,
            multiple types are converted.

    Returns:
        Converted data as *to_class* instance.
    """
    d_ = to_class()
    for key, value in d.items():
        if isinstance(value, from_class):
            d_[key] = convert_dicts(value, to_class=to_class, from_class=from_class)
        else:
            d_[key] = value
    return d_


def convert_json_safe(value: Any) -> Any:
    """Convert data to JSON-safe values.

    Anything not representable (e.g. Python objects) will be stringified.
    """
    try:
        json.dumps(value)
        return value
    except TypeError:
        pass

    if isinstance(value, (list, tuple, set)):
        return type(value)(convert_json_safe(x) for x in value)  # type: ignore[union-attr]

    if isinstance(value, dict):
        return type(value)(
            (convert_json_safe(k), convert_json_safe(v))
            for k, v in value.items()
        )

    return str(value)


# ── Object completion (tab-completion helper) ────────────────────────────────

def get_object_completions(
    instance: Any,
    prefix: str,
    types: tuple | None = None,
    instance_types: tuple | None = None,
) -> list[str]:
    """Get completion strings based on an object's attributes/keys.

    Supports dotted prefixes for nested attribute traversal.

    Args:
        instance: Object to introspect.
        prefix: Prefix to match, can be dot-separated.
        types: Attribute value types to match; any if ``None``.
        instance_types: Class types to recurse into; any if ``None``.

    Returns:
        List of matching completion strings.
    """
    word_toks: list[str] = []
    toks = prefix.split('.')
    while len(toks) > 1:
        attr = toks[0]
        toks = toks[1:]
        word_toks.append(attr)
        try:
            instance = getattr(instance, attr)
        except AttributeError:
            return []
        if instance_types and not isinstance(instance, instance_types):
            return []

    prefix = toks[-1]
    words: list[str] = []
    last_value = None

    attrs = list(dir(instance))
    try:
        for attr in instance:  # type: ignore[arg-type]
            if isinstance(attr, str):
                attrs.append(attr)
    except TypeError:
        pass

    for attr in attrs:
        if attr.startswith(prefix) and not attr.startswith('_') \
                and not hasattr(instance.__class__, attr):
            _value = getattr(instance, attr)
            if types and not isinstance(_value, types):
                continue
            if not callable(_value):
                words.append(attr)
                last_value = _value

    qual_words = ['.'.join(word_toks + [x]) for x in words]

    # Recurse into the single matched object if it's a suitable type
    if len(words) == 1 and last_value is not None:
        if not (instance_types and not isinstance(last_value, instance_types)):
            if not isinstance(last_value, (str, bytes)):
                try:
                    qual_word = qual_words[0]
                    nested = get_object_completions(last_value, '', types)
                    for word in nested:
                        qual_words.append(f"{qual_word}.{word}")
                except AttributeError:
                    pass

    return qual_words


# ── Metaclasses ──────────────────────────────────────────────────────────────

class AttributeForwardMeta(type):
    """Metaclass for forwarding attributes of a class member ``wrapped``
    onto the parent class.

    If the parent class already contains an attribute of the same name,
    forwarding is skipped.  If the wrapped object does not contain an
    attribute, the forwarded value will be ``None``.

    If the parent class contains a method ``_wrap_forwarded``, then
    forwarded values are passed to this method, and its return value
    becomes the attribute value.

    The class **must** contain:
    - ``keys`` (list of str): The attributes to forward.

    Example::

        >>> class Foo:
        ...     def __init__(self):
        ...         self.a = "a_from_foo"
        ...         self.b = "b_from_foo"
        ...
        >>> class Bah(metaclass=AttributeForwardMeta):
        ...     keys = ["a", "b", "c"]
        ...
        ...     @property
        ...     def a(self):
        ...         return "a_from_bah"
        ...
        ...     def __init__(self, child):
        ...         self.wrapped = child
        ...
        >>> x = Foo()
        >>> y = Bah(x)
        >>> print(y.a)
        a_from_bah
        >>> print(y.b)
        b_from_foo
        >>> print(y.c)
        None
    """

    def __new__(
        mcs, name: str, parents: tuple, members: dict,  # type: ignore[override]
    ) -> AttributeForwardMeta:
        def _defined(x: str) -> bool:
            return x in members or any(hasattr(p, x) for p in parents)

        keys = members.get('keys')
        if keys:
            for key in keys:
                if not _defined(key):
                    members[key] = mcs._make_forwarder(key)

        return super().__new__(mcs, name, parents, members)

    @classmethod
    def _make_forwarder(cls, key: str) -> property:
        def getter(self: Any) -> Any:
            value = getattr(self.wrapped, key, None)
            if hasattr(self, "_wrap_forwarded"):
                value = self._wrap_forwarded(key, value)
            return value
        return property(getter)


class LazyAttributeMeta(type):
    """Metaclass for adding properties to a class for accessing top-level keys
    in its ``_data`` dictionary, validating them on first reference.

    Property names are derived from the keys of the class's ``schema`` object.
    If a schema key is optional, the class property will evaluate to ``None``
    if the key is not present in ``_data``.

    The attribute getters created by this metaclass will perform lazy data
    validation, OR, if the class has a ``_validate_key`` method, will call
    this method, passing the key, key value and key schema.

    This metaclass creates:
    - For each key in ``cls.schema``, an attribute of the same name (unless
      already defined).
    - If the attribute already exists, a prefixed ``_{key}`` fallback.
    - ``validate_data``, ``validated_data``, ``_validate_key_impl``, and
      ``_schema_keys`` methods.
    """

    def __new__(
        mcs, name: str, parents: tuple, members: dict,  # type: ignore[override]
    ) -> LazyAttributeMeta:
        def _defined(x: str) -> bool:
            return x in members or any(hasattr(p, x) for p in parents)

        schema = members.get('schema')
        keys: set[str] = set()

        if schema and hasattr(schema, '_schema'):
            try:
                from schema import Optional as SchemaOptional
            except ImportError:
                raise ImportError(
                    "LazyAttributeMeta requires the `schema` package. "
                    "Install it with: pip install schema"
                )
            schema_dict = schema._schema
            for key, key_schema in schema_dict.items():
                optional = isinstance(key, SchemaOptional)
                while hasattr(key, '_schema'):
                    key = key._schema
                if isinstance(key, str):
                    keys.add(key)
                    if _defined(key):
                        attr = f"_{key}"
                        if _defined(attr):
                            raise Exception(f"Couldn't create fallback attr {attr!r}")
                    else:
                        attr = key
                    members[attr] = mcs._make_getter(key, attr, optional, key_schema)

        if schema or '_defined' not in members:
            members["validate_data"] = mcs._make_validate_data()
            members["validated_data"] = mcs._make_validated_data()
            members["_validate_key_impl"] = mcs._make_validate_key_impl()
            members["_schema_keys"] = frozenset(keys)

        return super().__new__(mcs, name, parents, members)

    @classmethod
    def _make_validate_data(cls):
        def validate_data(self) -> None:
            self.validated_data()
        return validate_data

    @classmethod
    def _make_validated_data(cls):
        def validated_data(self):
            if self.schema:
                d = {}
                for key in self._schema_keys:
                    d[key] = getattr(self, key)
                if self._data:
                    akeys = set(self._data.keys()) - set(d.keys())
                    for akey in akeys:
                        d[akey] = self._data[akey]
                return d
            return None
        return validated_data

    @classmethod
    def _make_validate_key_impl(cls):
        def validate_key_impl(self, key, attr, schema):
            try:
                from schema import Schema as SchemaType
            except ImportError:
                raise ImportError(
                    "LazyAttributeMeta requires the `schema` package. "
                    "Install it with: pip install schema"
                )
            s = schema if isinstance(schema, SchemaType) else SchemaType(schema)
            try:
                return s.validate(attr)
            except Exception as e:
                raise self.schema_error(
                    f"Validation of key {key!r} failed: {e}"
                )
        return validate_key_impl

    @classmethod
    def _make_getter(cls, key, attribute, optional, key_schema):
        def getter(self):
            if key not in (self._data or {}):
                if optional:
                    return None
                raise self.schema_error(f"Required key is missing: {key!r}")
            attr = self._data[key]
            if hasattr(self, "_validate_key"):
                return self._validate_key(key, attr, key_schema)
            return self._validate_key_impl(key, attr, key_schema)
        return cached_property(getter, name=attribute)
