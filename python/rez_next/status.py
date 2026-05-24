"""Bridge to rez_next status module (environment status inspection).

Aligns with rez.status API:
- `from rez.status import status` — singleton Status instance
- `status.context_file` — path to active context file
- `status.context` — active ResolvedContext
- `status.suites` — visible suites
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "status.py"
)
globals().update(runpy.run_path(str(_IMPL)))
