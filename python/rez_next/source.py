"""Bridge to rez_next source module (source code utilities).

Aligns with rez.source API:
- Source code handling utilities
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "source.py"
)
globals().update(runpy.run_path(str(_IMPL)))
