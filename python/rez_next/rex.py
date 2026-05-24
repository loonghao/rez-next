"""Bridge to rez_next rex module (Rez Execution language).

Aligns with rez.rex API:
- ``Rex`` — Rez execution language interpreter
- ``rex`` — global Rex instance
- ``execute()`` — execute Rex code
- ``Executable`` — Rex executable node
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "rex.py"
)
globals().update(runpy.run_path(str(_IMPL)))
