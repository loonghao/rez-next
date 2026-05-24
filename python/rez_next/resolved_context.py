"""Bridge to rez_next resolved_context module.

Aligns with rez.resolved_context API:
- ``ResolvedContext`` — a resolved package environment
- ``diff()`` — compare two resolved contexts
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "resolved_context.py"
)
globals().update(runpy.run_path(str(_IMPL)))
