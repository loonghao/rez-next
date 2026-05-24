"""Bridge to rez_next utils/resources module (resource utilities).

Aligns with rez.utils.resources API:
- Resource registration and lookup utilities
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "utils"
    / "resources.py"
)
globals().update(runpy.run_path(str(_IMPL)))
