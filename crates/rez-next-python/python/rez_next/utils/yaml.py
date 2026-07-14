"""YAML output helpers backed by the bundled native extension.

Rez package definitions in YAML are intentionally unsupported.  The remaining
helpers exist for Rez APIs that write YAML-shaped metadata and do not require a
Python runtime dependency.
"""

from __future__ import annotations

import os
from typing import Any

from rez_next._native.serialise_ import (
    py_dump_yaml as _dump_yaml,
    py_read_package_data as _read_yaml,
)


def dump_yaml(data: Any, default_flow_style: bool = False) -> str:
    """Serialize data through the native serializer.

    ``default_flow_style`` remains accepted for call compatibility.  Formatting
    is selected by the native serializer and is not part of the API contract.
    """

    del default_flow_style
    return _dump_yaml(data).strip()


def load_yaml(filepath: str) -> Any:
    """Load YAML metadata through the bundled native extension."""

    if not os.path.exists(filepath):
        raise FileNotFoundError(f"YAML file not found: {filepath}")
    return _read_yaml(filepath, "yaml")


def save_yaml(filepath: str, **fields: Any) -> None:
    """Write YAML-shaped metadata without a third-party Python dependency."""

    content = dump_yaml(fields)
    os.makedirs(os.path.dirname(filepath) or ".", exist_ok=True)
    with open(filepath, "w", encoding="utf-8") as handle:
        handle.write(content + "\n")


__all__ = ["dump_yaml", "load_yaml", "save_yaml"]
