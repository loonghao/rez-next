"""
Typing protocols for Rez-next.

Mirrors ``rez.utils.typing`` — provides Protocols used across the Rez API
for structural subtyping of IO and comparable types.
"""
from __future__ import annotations

from typing import Any, Protocol


class SupportsLessThan(Protocol):
    """Protocol for types that support the ``<`` operator."""
    def __lt__(self, __other: Any) -> bool:
        ...


class SupportsWrite(Protocol):
    """Protocol for types that have a ``write`` method (e.g. file-like)."""
    def write(self, __s: str) -> object:
        ...


class SupportsRead(Protocol):
    """Protocol for types that have a ``read`` method."""
    def read(self) -> str:
        ...
