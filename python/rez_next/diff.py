"""Bridge to rez_next diff module (context diff).

Aligns with rez.diff API:
- ``diff_contexts()`` — compare two resolved contexts
- ``PackageDiff`` — individual package difference
- ``ContextDiff`` — full context diff result
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "diff.py"
)
globals().update(runpy.run_path(str(_IMPL)))
