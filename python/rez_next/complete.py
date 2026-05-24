"""Bridge to rez_next complete module (shell completion)."""
from pathlib import Path
import runpy
_IMPL = Path(__file__).resolve().parents[2] / "crates" / "rez-next-python" / "python" / "rez_next" / "complete.py"
globals().update(runpy.run_path(str(_IMPL)))
