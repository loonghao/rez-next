"""Bridge to rez_next build_plugins module (package build helpers)."""

from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "build_plugins.py"
)

globals().update(runpy.run_path(str(_IMPL)))
