"""Bridge to rez_next package_filter module (package filtering)."""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "package_filter.py"
)
globals().update(runpy.run_path(str(_IMPL)))
