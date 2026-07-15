"""Package ordering strategies for rez-next.

This module provides the same API as rez.package_order for drop-in compatibility.
It defines ordering strategies used during package resolution to determine the
order in which package versions are considered.

API Reference: rez.package_order
  - NullPackageOrder: no reordering
  - SortedOrder: sort by version ascending/descending
  - VersionSplitPackageOrder: versions ≤ threshold come first
  - TimestampPackageOrder: timestamp-based soft ordering
  - PerFamilyOrder: apply different orderers per package family
"""

from collections import OrderedDict as _OrderedDict
from typing import Any, Optional

import rez_next._native  # noqa: F401
from rez_next._native.package_order import (  # noqa: F401
    NullPackageOrder,
    SortedOrder,
    TimestampPackageOrder,
    VersionSplitPackageOrder,
)

# Orderer registry: maps name → class for plugin-like extension
_order_registry: dict[str, type] = {}


def register_orderer(cls: type) -> None:
    """Register a custom orderer class.

    This matches rez.package_order.register_orderer().

    Args:
        cls: A class (subclass of one of the orderer bases) with a `name` attribute.
    """
    name = getattr(cls, "name", None)
    if name is None:
        raise ValueError(f"Orderer class {cls.__name__} must have a 'name' attribute")
    _order_registry[str(name)] = cls


def _build_pod(orderer) -> dict[str, Any]:
    """Convert an orderer instance to its serializable dict form.

    Args:
        orderer: An orderer instance.

    Returns:
        Dict with 'type' key and relevant fields.
    """
    name = getattr(orderer, "name", None)
    if name is not None:
        pod: dict[str, Any] = {"type": name}
        if hasattr(orderer, "packages") and getattr(orderer, "packages") is not None:
            pod["packages"] = list(getattr(orderer, "packages"))
        if hasattr(orderer, "descending"):
            pod["descending"] = bool(getattr(orderer, "descending"))
        if hasattr(orderer, "first_version"):
            pod["first_version"] = str(getattr(orderer, "first_version"))
        if hasattr(orderer, "timestamp"):
            pod["timestamp"] = int(getattr(orderer, "timestamp"))
        if hasattr(orderer, "rank"):
            pod["rank"] = int(getattr(orderer, "rank"))
        return pod
    return {"type": "no_order"}


def to_pod(orderer) -> dict[str, Any]:
    """Convert an orderer to a serializable POD dict.

    This matches rez.package_order.to_pod().

    Args:
        orderer: An orderer instance.

    Returns:
        A dict with 'type' and other fields suitable for YAML serialization.
    """
    return _build_pod(orderer)


def from_pod(data: dict[str, Any]) -> Any:
    """Create an orderer from a POD dict.

    This matches rez.package_order.from_pod().

    Args:
        data: A dict with at least 'type' field.

    Returns:
        An orderer instance.

    Raises:
        ValueError: If the type is unknown.
    """
    orderer_type = data.get("type", "no_order")
    packages = data.get("packages")

    if orderer_type == "sorted" or orderer_type == SortedOrder.name:
        return SortedOrder(
            descending=data.get("descending", True),
            packages=packages,
        )
    elif orderer_type == "no_order" or orderer_type == NullPackageOrder.name:
        return NullPackageOrder(packages=packages)
    elif orderer_type == "version_split" or orderer_type == VersionSplitPackageOrder.name:
        return VersionSplitPackageOrder(
            first_version=data.get("first_version", "0"),
            packages=packages,
        )
    elif orderer_type == "soft_timestamp" or orderer_type == TimestampPackageOrder.name:
        return TimestampPackageOrder(
            timestamp=data.get("timestamp", 0),
            rank=data.get("rank", 0),
            packages=packages,
        )
    elif orderer_type in _order_registry:
        cls = _order_registry[orderer_type]
        return cls(**data)
    else:
        raise ValueError(f"Unknown orderer type: {orderer_type}")


class PerFamilyOrder:
    """Apply different ordering strategies per package family.

    This matches rez.package_order.PerFamilyOrder.

    Args:
        order_dict: Mapping of package family name -> orderer instance.
        default_order: Fallback orderer for families not in order_dict.
    """

    name = "per_family"

    def __init__(
        self,
        order_dict: dict[str, Any],
        default_order: Any = None,
    ):
        self._order_dict = _OrderedDict(order_dict)
        self._default = default_order

    @property
    def packages(self) -> Optional[list[str]]:
        return list(self._order_dict.keys()) if self._order_dict else None

    def get_orderer(self, package_name: str) -> Any:
        """Get the orderer for a specific package family."""
        if package_name in self._order_dict:
            return self._order_dict[package_name]
        return self._default

    def to_pod(self) -> dict[str, Any]:
        """Serialize to POD format."""
        entries: list[dict[str, Any]] = []
        for family, orderer in self._order_dict.items():
            entry = _build_pod(orderer)
            entry["packages"] = [family]
            entries.append(entry)
        pod: dict[str, Any] = {"type": self.name}
        if entries:
            pod["entries"] = entries
        return pod


def get_orderer(
    package_name: str,
    orderers: Optional[list] = None,
) -> Any:
    """Get the applicable orderer for a given package family.

    This matches rez.package_order.get_orderer().

    Args:
        package_name: Package family name (e.g., "python").
        orderers: List of orderer instances to search. If None, uses defaults.

    Returns:
        An orderer instance (defaults to SortedOrder(descending=True)).
    """
    if orderers is None:
        return SortedOrder(descending=True)

    for orderer in orderers:
        pkgs = getattr(orderer, "packages", None)
        if pkgs is None or package_name in pkgs:
            return orderer

    return SortedOrder(descending=True)


# Default orderers
DEFAULT_ORDERERS: list[Any] = [SortedOrder(descending=True, packages=None)]


# Re-export for convenience
__all__ = [
    "NullPackageOrder",
    "SortedOrder",
    "VersionSplitPackageOrder",
    "TimestampPackageOrder",
    "PerFamilyOrder",
    "DEFAULT_ORDERERS",
    "get_orderer",
    "register_orderer",
    "to_pod",
    "from_pod",
]
