"""
rex_bindings — aligns with rez.rex_bindings.

Provides simple wrapper objects used by the Rex executor to expose Rez
objects (Version, Variant, requirements, etc.) to the embedded Rex Python
environment in ``package.py``.

These bindings provide a stable, documented surface for ``package.py``
authors so they do not depend on Rez internal APIs.

Rez API: ``rez.rex_bindings``
"""

from __future__ import annotations

from typing import Any, Iterator, Optional


# ── Base class ─────────────────────────────────────────────────────────────


class Binding:
    """Base binding class backed by a private data dict.

    Attributes are accessed via ``__getattr__``, falling back to the
    internal ``_data`` dictionary.

    Rez API: ``rez.rex_bindings.Binding``
    """

    def __init__(self, data: dict[str, Any] | None = None) -> None:
        self._data: dict[str, Any] = dict(data or {})

    def __getattr__(self, attr: str) -> Any:
        try:
            return self._data[attr]
        except KeyError:
            raise AttributeError(  # noqa: TRY003
                f"'{self.__class__.__name__}' has no attribute '{attr}'"
            ) from None

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__}>"


# ── VersionBinding ─────────────────────────────────────────────────────────


def _parse_version_token(token: str) -> Any:
    """Convert a version token string to int if possible, otherwise keep str."""
    try:
        return int(token)
    except ValueError:
        return token


class VersionBinding(Binding):
    """Wrapper around a ``Version`` for use in Rex ``package.py`` scripts.

    Provides access to version tokens via attributes (``major``, ``minor``,
    ``patch``) as well as indexing and iteration.

    Rez API: ``rez.rex_bindings.VersionBinding``

    Example::

        v = VersionBinding(Version("1.2.3alpha"))
        v.major         # -> 1
        v.minor         # -> 2
        v.patch         # -> "3alpha"
        v[0]            # -> 1
        v[:2]           # -> (1, 2)
        list(v)         # -> [1, 2, "3alpha"]
    """

    def __init__(self, version: Any) -> None:
        self._version = version
        version_str = str(version)
        self._tokens: tuple[Any, ...] = tuple(
            _parse_version_token(t) for t in version_str.split(".")
        )

    @property
    def major(self) -> Any:
        """First version token (int if numeric, otherwise original value)."""
        return self._tokens[0] if self._tokens else ""

    @property
    def minor(self) -> Any:
        """Second version token (int if numeric)."""
        return self._tokens[1] if len(self._tokens) > 1 else ""

    @property
    def patch(self) -> Any:
        """Third version token."""
        return self._tokens[2] if len(self._tokens) > 2 else ""

    def as_tuple(self) -> tuple[Any, ...]:
        """Return all version tokens as a tuple."""
        return self._tokens

    def __getitem__(self, index: int | slice) -> Any:
        return self._tokens[index]

    def __len__(self) -> int:
        return len(self._tokens)

    def __iter__(self) -> Iterator[Any]:
        return iter(self._tokens)

    def __str__(self) -> str:
        return str(self._version)

    def __repr__(self) -> str:
        return f"VersionBinding({self._version})"


# ── VariantBinding ─────────────────────────────────────────────────────────


class VariantBinding(Binding):
    """Wrapper around a ``Variant`` for use in Rex ``package.py`` scripts.

    Provides access to variant attributes with optional ``cached_root``
    path substitution.

    Rez API: ``rez.rex_bindings.VariantBinding``

    Example::

        this = VariantBinding(variant, cached_root="/cache/...")
        resolve.mypkg.root     # uses cached_root when applicable
    """

    def __init__(
        self,
        variant: Any,
        cached_root: str = "",
        interpreter: Any = None,
        data: dict[str, Any] | None = None,
    ) -> None:
        super().__init__(data)
        self._variant = variant
        self._cached_root = cached_root
        self._interpreter = interpreter

    def _is_in_package_cache(self) -> bool:
        """Return True if this variant's path should come from the cache."""
        return bool(self._cached_root)

    @property
    def root(self) -> str:
        """Return the variant root path (cached or actual)."""
        if self._cached_root:
            import os
            variant_subpath = getattr(self._variant, "subpath", "")
            return os.path.normpath(os.path.join(self._cached_root, variant_subpath))
        variant_root = getattr(self._variant, "root", None)
        if variant_root is not None:
            return str(variant_root)
        return str(self._variant)

    def __getattr__(self, attr: str) -> Any:
        try:
            return super().__getattr__(attr)
        except AttributeError:
            pass
        return getattr(self._variant, attr)

    def __repr__(self) -> str:
        return f"VariantBinding({getattr(self._variant, 'name', '?')})"


# ── Read-only mapping bindings ─────────────────────────────────────────────


class RO_MappingBinding(Binding):
    """Read-only dictionary-like binding for Rex environments.

    Provides ``get()``, ``__contains__``, and ``__getitem__`` access to
    backing data without mutation support.

    Rez API: ``rez.rex_bindings.RO_MappingBinding``
    """

    def get(self, name: str, default: Any = None) -> Any:
        try:
            return self._data[name]
        except KeyError:
            return default

    def __getitem__(self, name: str) -> Any:
        return self._data[name]

    def __contains__(self, name: str) -> bool:
        return name in self._data

    def __str__(self) -> str:
        return str(self._data)


class VariantsBinding(RO_MappingBinding):
    """Wraps resolved variants keyed by package name.

    Raises ``AttributeError`` with a descriptive message when a
    non-existent package is accessed.

    Rez API: ``rez.rex_bindings.VariantsBinding``
    """

    def __getattr__(self, attr: str) -> Any:
        try:
            return super().__getattr__(attr)
        except AttributeError:
            raise AttributeError(  # noqa: TRY003
                f"package does not exist: '{attr}'"
            ) from None


class RequirementsBinding(RO_MappingBinding):
    """Wraps requirement strings keyed by package name.

    Provides ``get_range()`` to retrieve a ``VersionRange`` for a
    named requirement.

    Rez API: ``rez.rex_bindings.RequirementsBinding``
    """

    def get_range(self, name: str, default: Any = None) -> Any:
        """Return the ``PackageRequirement`` for *name*, or *default*."""
        req_str = self._data.get(name)
        if req_str is None:
            return default
        from rez_next import PackageRequirement
        return PackageRequirement(req_str)


class EphemeralsBinding(RO_MappingBinding):
    """Wraps resolved ephemeral request strings.

    Keys are automatically stripped of their leading ``.`` prefix (because
    ephemeral requests use the ``.name`` convention while the bindings
    expose the bare name).

    Rez API: ``rez.rex_bindings.EphemeralsBinding``
    """

    def get_range(self, name: str, default: Any = None) -> Any:
        """Return the ``PackageRequirement`` for an ephemeral *name*, or *default*."""
        req_str = self._data.get(name)
        if req_str is None:
            return default
        from rez_next import PackageRequirement
        return PackageRequirement(req_str)


# ── Helper function ────────────────────────────────────────────────────────


def intersects(
    obj: Any,
    range_: str,
) -> bool:
    """Test whether *obj* intersects with *range_*.

    Supports various input types:

    * ``str`` — a requirement string such as ``'maya-2019+'``
    * ``VariantBinding`` — checks the variant's version
    * ``VersionBinding`` — checks the binding's version

    Rez API: ``rez.rex_bindings.intersects()``

    Example::

        if intersects(request.maya, "2019+"):
            info("maya >= 2019 is available")
    """
    from rez_next import VersionRange, PackageRequirement

    # Resolve obj to a VersionRange
    if isinstance(obj, VersionBinding):
        obj_range = VersionRange(str(obj._version))
    elif isinstance(obj, VariantBinding):
        ver = getattr(obj._variant, "version", None)
        if ver is None:
            return False
        obj_range = VersionRange(str(ver))
    elif isinstance(obj, str):
        req = PackageRequirement(obj)
        obj_range = VersionRange(req.version_range)
    elif isinstance(obj, VersionRange):
        obj_range = obj
    elif hasattr(obj, "version_range"):
        obj_range = obj.version_range
    else:
        return False

    # Resolve range_ to a VersionRange
    if isinstance(range_, str):
        test_range = VersionRange(range_)
    elif isinstance(range_, VersionRange):
        test_range = range_
    else:
        return False

    return bool(test_range.intersects(obj_range))


__all__ = [
    "Binding",
    "VersionBinding",
    "VariantBinding",
    "RO_MappingBinding",
    "VariantsBinding",
    "RequirementsBinding",
    "EphemeralsBinding",
    "intersects",
]
