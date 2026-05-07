"""Package serialisation module for rez_next.

This module provides functions to serialise package data to various formats
(YAML, JSON, Python, TOML).
"""

from rez_next._native.serialise_ import (  # type: ignore[import]
    py_dump_package_data as dump_package_data,
    py_dump_yaml as dump_yaml,
    py_as_block_string as as_block_string,
    py_dict_to_attributes_code as dict_to_attributes_code,
    py_package_key_order as package_key_order,
    FileFormat,
)

__all__ = [
    "dump_package_data",
    "dump_yaml",
    "as_block_string",
    "dict_to_attributes_code",
    "package_key_order",
    "FileFormat",
]
