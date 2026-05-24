"""Bridge to rez_next serialise module (file serialisation).

Aligns with rez.serialise API:
- ``FileFormat`` — supported file formats
- ``load_from_file()`` — load data from a file
- ``open_file_for_write()`` — write data with NFS-safe local cache
- ``load_py()`` / ``load_yaml()`` / ``load_txt()`` — format-specific loaders
- ``EarlyThis`` — helper for ``@early`` decorated functions
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "serialise.py"
)
globals().update(runpy.run_path(str(_IMPL)))
