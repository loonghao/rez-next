"""
Rez-compatible package_serialise module.

Aligns with ``rez.package_serialise`` API:
- ``dump_package_data()`` — serialise package data to Python/YAML format
- ``package_key_order`` — recommended key ordering for package definitions
- ``FileFormat`` — file format enum

See https://github.com/AcademySoftwareFoundation/rez/blob/main/src/rez/package_serialise.py
"""

from __future__ import annotations

from .serialise_ import (  # type: ignore[import]
    dump_package_data,
    dump_yaml,
    as_block_string,
    dict_to_attributes_code,
    package_key_order,
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
