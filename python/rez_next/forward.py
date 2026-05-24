"""Bridge to rez_next forward module (forwarding utilities).

Aligns with rez.forward API:
- Forwarding support for package requests
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "forward.py"
)
globals().update(runpy.run_path(str(_IMPL)))
