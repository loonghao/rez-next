"""Bridge to rez_next package_serialise module.

Aligns with rez.package_serialise API:
- ``dump_package_data()`` — serialise package data to Python/YAML format
- ``FileFormat`` — file format enum
- ``package_key_order`` — recommended key ordering for package definitions

See https://github.com/AcademySoftwareFoundation/rez/blob/main/src/rez/package_serialise.py
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "serialise_.py"
)
_serialise_impl = runpy.run_path(str(_IMPL))

# Re-export package serialisation symbols
dump_package_data = _serialise_impl["dump_package_data"]
dump_yaml = _serialise_impl["dump_yaml"]
as_block_string = _serialise_impl.get("as_block_string")
dict_to_attributes_code = _serialise_impl.get("dict_to_attributes_code")
package_key_order = _serialise_impl["package_key_order"]
FileFormat = _serialise_impl["FileFormat"]

__all__ = [
    "dump_package_data",
    "dump_yaml",
    "as_block_string",
    "dict_to_attributes_code",
    "package_key_order",
    "FileFormat",
]
