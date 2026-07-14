"""Package serialisation module for rez_next.

This module provides functions to serialise package data to various formats
(YAML, JSON, Python, TOML).
"""

from rez_next._native.serialise_ import (  # type: ignore[import]
    py_dump_package_data as _native_dump_package_data,
    py_dump_yaml as dump_yaml,
    py_as_block_string as as_block_string,
    py_dict_to_attributes_code as dict_to_attributes_code,
    py_package_key_order as _native_package_key_order,
    FileFormat,
)
from io import BytesIO

# package_key_order is a list (matches rez.package_serialise.package_key_order)
package_key_order = _native_package_key_order()


def dump_package_data(data, destination=None, format="yaml", skip_attributes=None):
    buf = BytesIO()
    _native_dump_package_data(data, buf, format, skip_attributes)
    content = buf.getvalue()
    if destination is None:
        return content.decode("utf-8")
    if hasattr(destination, "write"):
        if isinstance(destination, BytesIO) or (hasattr(destination, "mode") and "b" in destination.mode):
            destination.write(content)
        else:
            destination.write(content.decode("utf-8"))
    else:
        with open(destination, "w", encoding="utf-8") as handle:
            handle.write(content.decode("utf-8"))
    return None

__all__ = [
    "dump_package_data",
    "dump_yaml",
    "as_block_string",
    "dict_to_attributes_code",
    "package_key_order",
    "FileFormat",
]
