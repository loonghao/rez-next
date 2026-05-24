"""Bridge to rez_next plugins module (plugin system).

Aligns with rez.plugins API:
- ``Plugin`` — plugin base class
- ``load_plugin()`` — load a plugin by name
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "plugins.py"
)
globals().update(runpy.run_path(str(_IMPL)))
