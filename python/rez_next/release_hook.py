"""Bridge to rez_next release_hook module (release hooks)."""
from pathlib import Path
import runpy
_IMPL = Path(__file__).resolve().parents[2] / "crates" / "rez-next-python" / "python" / "rez_next" / "release_hook.py"
globals().update(runpy.run_path(str(_IMPL)))
