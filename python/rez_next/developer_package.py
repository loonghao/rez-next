"""Bridge to rez_next developer_package module (developer/source packages)."""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "developer_package.py"
)
globals().update(runpy.run_path(str(_IMPL)))
