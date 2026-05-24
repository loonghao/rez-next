"""Bridge to rez_next bind module (system tool binding as packages).

Aligns with rez.bind API:
- ``bind_package()`` — bind system software as a rez package
- ``get_bind_modules()`` — list available bind modules
- ``find_bind_module()`` — locate a bind module by name
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "bind.py"
)
globals().update(runpy.run_path(str(_IMPL)))
