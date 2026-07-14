"""
Utilities for working with dict-based schemas.

Mirrors ``rez.utils.schema`` — provides helpers for converting and
manipulating nested schema dicts.

Note:
    This module does NOT depend on ``rez.vendor.schema`` (skipped in rez-next).
    It provides equivalent helpers using pure Python constructs.
"""
from __future__ import annotations

from typing import Any, TYPE_CHECKING

if TYPE_CHECKING:
    from typing import Callable


class Required:
    """Marker for a required key in a schema dict.

    Mirrors ``rez.vendor.schema.schema.Schema`` when used as a key.
    """
    def __init__(self, key: str) -> None:
        self._key = key

    @property
    def key(self) -> str:
        return self._key

    def __repr__(self) -> str:
        return "Required(%r)" % self._key


class Optional:
    """Marker for an optional key in a schema dict.

    Mirrors ``rez.vendor.schema.schema.Optional`` when used as a key.
    """
    def __init__(self, key: str | type) -> None:
        self._key = key

    @property
    def key(self) -> str | type:
        return self._key

    def __repr__(self) -> str:
        return "Optional(%r)" % self._key


def _get_leaf(value: Any) -> Any:
    """Unwrap nested ``Required``/``Optional`` markers."""
    if isinstance(value, (Required, Optional)):
        return value.key
    return value


def schema_keys(schema) -> set[str]:
    """Get the string values of keys in a dict-based schema.

    Non-string keys are ignored.

    Args:
        schema: A schema object with a ``_schema`` attribute that is a dict.

    Returns:
        Set of string keys from the schema dict.
    """
    keys: set[str] = set()
    dict_ = schema._schema
    assert isinstance(dict_, dict)

    for key in dict_.keys():
        key_ = _get_leaf(key)
        if isinstance(key_, str):
            keys.add(key_)

    return keys


def dict_to_schema(
    schema_dict: dict,
    required: bool,
    allow_custom_keys: bool = True,
    modifier: Callable | None = None,
) -> dict:
    """Convert a dict of markers into a schema.

    Args:
        schema_dict: Nested dict with ``Required``/``Optional`` markers as keys.
        required: Whether to make schema keys required or optional.
        allow_custom_keys: If True, allows arbitrary extra dict keys.
        modifier: Optional callable applied to dict values.

    Returns:
        A schema dict with ``Required``/``Optional`` wrappers applied.

    Example::

        >>> s = dict_to_schema({"name": str, "version": int}, required=True)
        >>> isinstance(list(s.keys())[0], Required)
        True
    """
    def _to(value: Any) -> Any:
        if isinstance(value, dict):
            d: dict = {}
            for k, v in value.items():
                if isinstance(k, str):
                    k = Required(k) if required else Optional(k)
                d[k] = _to(v)
            if allow_custom_keys:
                d[Optional(str)] = object
            return d
        return value

    return _to(schema_dict)


def extensible_schema_dict(schema_dict: dict) -> dict:
    """Create a schema dict that allows arbitrary extra keys.

    This helps keep newer configs or package definitions compatible with
    older rez versions that may not support newer schema fields.
    """
    result: dict = {
        Optional(str): object,
    }
    result.update(schema_dict)
    return result
