"""
Scope and recursive attribute utilities for Rez-next.

Mirrors ``rez.utils.scope`` — provides ``RecursiveAttribute``,
``ScopeContext``, and ``scoped_formatter``/``scoped_format`` functions.
"""
from __future__ import annotations

from collections import UserDict
from typing import Any, cast, TYPE_CHECKING

if TYPE_CHECKING:
    from typing import Self

from rez_next.utils.formatting import StringFormatMixin, StringFormatType


class RecursiveAttribute(UserDict, StringFormatMixin):
    """An object that can have new attributes added recursively::

        >>> a = RecursiveAttribute()
        >>> a.foo.bah = 5
        >>> a.foo['eek'] = 'hey'
        >>> a.fee = 1
        >>> print(a.to_dict())
        {'foo': {'bah': 5, 'eek': 'hey'}, 'fee': 1}

    A recursive attribute can also be created from a dict and made read-only::

        >>> d = {'fee': {'fi': {'fo': 'fum'}}, 'ho': 'hum'}
        >>> a = RecursiveAttribute(d, read_only=True)
        >>> print(str(a))
        {'fee': {'fi': {'fo': 'fum'}}, 'ho': 'hum'}
        >>> a.new = True
        AttributeError: ...

    Rez API: ``rez.utils.scope.RecursiveAttribute``
    """
    format_expand = StringFormatType.unchanged

    def __init__(self, data: dict | None = None, read_only: bool = False) -> None:
        self.__dict__.update(dict(data={}, read_only=read_only))
        self._update(data or {})

    def __getattr__(self, attr: str) -> Any:
        def _noattrib() -> None:
            raise AttributeError(
                "'%s' object has no attribute '%s'"
                % (self.__class__.__name__, attr)
            )
        d = self.__dict__
        if attr.startswith('__') and attr.endswith('__'):
            try:
                return d[attr]
            except KeyError:
                _noattrib()
        if attr in d["data"]:
            return d["data"][attr]
        if d["read_only"]:
            _noattrib()
        attr_ = self._create_child_attribute(attr)
        assert isinstance(attr_, RecursiveAttribute)
        attr_.__dict__["pending"] = (attr, self)
        return attr_

    def __setattr__(self, attr: str, value: Any) -> None:
        d = self.__dict__
        if d["read_only"]:
            if attr in d["data"]:
                raise AttributeError(
                    "'%s' object attribute '%s' is read-only"
                    % (self.__class__.__name__, attr)
                )
            raise AttributeError(
                "'%s' object has no attribute '%s'"
                % (self.__class__.__name__, attr)
            )
        elif attr.startswith('__') and attr.endswith('__'):
            d[attr] = value
        else:
            d["data"][attr] = value
            self._reparent()

    def __getitem__(self, attr: str) -> Any:
        return getattr(self, attr)

    def __str__(self) -> str:
        return str(self.to_dict())

    def __repr__(self) -> str:
        return "%s(%r)" % (self.__class__.__name__, self.to_dict())

    def _create_child_attribute(self, attr: str) -> RecursiveAttribute:
        """Override this method to create new child attributes."""
        return self.__class__()

    def to_dict(self) -> dict[str, Any]:
        """Get an equivalent dict representation."""
        d: dict[str, Any] = {}
        for k, v in self.__dict__["data"].items():
            if isinstance(v, RecursiveAttribute):
                d[k] = v.to_dict()
            else:
                d[k] = v
        return d

    def copy(self) -> Self:
        return self.__class__(self.__dict__['data'].copy())

    def update(self, data: dict[str, Any]) -> None:  # type: ignore[override]
        """Dict-like update operation."""
        if self.__dict__["read_only"]:
            raise AttributeError("read-only, cannot be updated")
        self._update(data)

    def _update(self, data: dict[str, Any]) -> None:
        for k, v in data.items():
            if isinstance(v, dict):
                v = RecursiveAttribute(v)
            self.__dict__["data"][k] = v

    def _reparent(self) -> None:
        d = self.__dict__
        if "pending" in d:
            attr_, parent = d["pending"]
            parent._reparent()
            parent.__dict__["data"][attr_] = self
            del d["pending"]


class _Scope(RecursiveAttribute):
    def __init__(
        self,
        name: str | None = None,
        context: ScopeContext | None = None,
    ) -> None:
        RecursiveAttribute.__init__(self)
        self.__dict__.update(dict(name=name, context=context, locals=None))

    def __enter__(self) -> _Scope:
        import sys
        locals_ = sys._getframe(1).f_locals
        self.__dict__["locals"] = locals_.copy()
        return self

    def __exit__(self, *args: Any) -> None:
        import sys
        updates: dict[str, Any] = {}
        d = self.__dict__
        locals_ = sys._getframe(1).f_locals
        self_locals = d["locals"]
        for k, v in locals_.items():
            if (
                not (k.startswith("__") and k.endswith("__"))
                and (k not in self_locals or v != self_locals[k])
                and not isinstance(v, _Scope)
            ):
                updates[k] = v
        self.update(updates)
        locals_.clear()
        locals_.update(self_locals)
        self_context = d["context"]
        if self_context:
            self_context._scope_exit(d["name"])

    def _create_child_attribute(self, attr: str) -> RecursiveAttribute:
        return RecursiveAttribute()


class ScopeContext:
    """A context manager for creating nested dictionaries.

    See: ``rez.utils.scope.ScopeContext``.

    Usage::

        >>> scope = ScopeContext()
        >>> with scope("animal"):
        ...     count = 2
        ...     with scope("cat"):
        ...         friendly = False
        >>> print(pprint.pformat(scope.to_dict()))
        {'animal': {'cat': {'friendly': False}, 'count': 2}}

    Rez API: ``rez.utils.scope.ScopeContext``
    """
    def __init__(self) -> None:
        self.scopes: dict[tuple, _Scope] = {}
        self.scope_stack = [_Scope()]

    def __call__(self, name: str) -> _Scope:
        path = tuple([x.name for x in self.scope_stack[1:]] + [name])
        if path in self.scopes:
            scope = self.scopes[path]
        else:
            scope = _Scope(name, self)
            self.scopes[path] = scope
        self.scope_stack.append(scope)
        return scope

    def _scope_exit(self, name: str) -> None:
        scope = self.scope_stack.pop()
        assert self.scope_stack
        assert name == scope.name
        data = {cast(str, scope.name): scope.to_dict()}
        self.scope_stack[-1].update(data)

    def to_dict(self) -> dict[str, Any]:
        """Get an equivalent dict representation."""
        return self.scope_stack[-1].to_dict()

    def __str__(self) -> str:
        names = ('.'.join(y for y in x) for x in self.scopes.keys())
        return "%r" % (tuple(names),)


def scoped_formatter(**objects: Any) -> RecursiveAttribute:
    """Format a string with respect to a set of objects' attributes.

    Use this rather than ``scoped_format`` when you need to reuse the formatter.

    Rez API: ``rez.utils.scope.scoped_formatter()``
    """
    return RecursiveAttribute(objects, read_only=True)


def scoped_format(txt: str, **objects: Any) -> str:
    """Format a string with respect to a set of objects' attributes.

    Example::

        >>> class Foo:
        ...     def __init__(self):
        ...         self.name = "Dave"
        >>> print(scoped_format("hello {foo.name}", foo=Foo()))
        hello Dave

    Args:
        objects: Dict of objects to format with.

    Returns:
        Formatted string.

    Rez API: ``rez.utils.scope.scoped_format()``
    """
    pretty = objects.pop("pretty", RecursiveAttribute.format_pretty)
    expand = objects.pop("expand", RecursiveAttribute.format_expand)
    formatter = scoped_formatter(**objects)
    return formatter.format(txt, pretty=pretty, expand=expand)
