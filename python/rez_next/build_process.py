"""Bridge to rez_next build_process module (build and release process framework)."""
from pathlib import Path
import runpy
_IMPL = Path(__file__).resolve().parents[2] / "crates" / "rez-next-python" / "python" / "rez_next" / "build_process.py"
globals().update(runpy.run_path(str(_IMPL)))
