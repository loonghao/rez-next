"""Bridge to rez_next wrapper module (suite tool wrappers)."""
from pathlib import Path
import runpy
_IMPL = Path(__file__).resolve().parents[2] / "crates" / "rez-next-python" / "python" / "rez_next" / "wrapper.py"
globals().update(runpy.run_path(str(_IMPL)))
