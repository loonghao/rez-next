"""Package serialisation module for rez_next.

This module provides functions to serialise package data to various formats
(YAML, JSON, Python, TOML).
"""

from rez_next._native.serialise_ import (  # type: ignore[import]
    py_dump_package_data as _native_dump_package_data,
    py_dump_yaml as dump_yaml,
    py_as_block_string as as_block_string,
    py_dict_to_attributes_code as dict_to_attributes_code,
    py_package_key_order as package_key_order,
    FileFormat,
)
from io import BytesIO


def dump_package_data(data, destination=None, format="yaml", skip_attributes=None):
    buf = BytesIO()
    _native_dump_package_data(data, buf, format, skip_attributes)
    text = buf.getvalue().decode("utf-8")
    if destination is None:
        return text
    if hasattr(destination, "write"):
        destination.write(text)
    else:
        with open(destination, "w", encoding="utf-8") as handle:
            handle.write(text)
    return None

__all__ = [
    "dump_package_data",
    "dump_yaml",
    "as_block_string",
    "dict_to_attributes_code",
    "package_key_order",
    "FileFormat",
]
