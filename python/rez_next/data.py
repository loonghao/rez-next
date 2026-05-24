"""Bridge to rez_next data module (data utilities).

Aligns with rez.data API:
- Data handling utilities used by rez internally
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "data.py"
)
globals().update(runpy.run_path(str(_IMPL)))
