"""Bridge to rez_next build_ module (build process utilities).

Aligns with rez.build_ API:
- ``BuildProcess`` — build process management
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "build_.py"
)
globals().update(runpy.run_path(str(_IMPL)))
