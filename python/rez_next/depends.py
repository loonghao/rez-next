"""Bridge to rez_next depends module (dependency visualization).

Aligns with rez.depends API:
- ``print_depends()`` — print dependency tree
- ``get_depends()`` — get dependency information
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "depends.py"
)
globals().update(runpy.run_path(str(_IMPL)))
