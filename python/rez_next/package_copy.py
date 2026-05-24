"""Bridge to rez_next package_copy module (copy packages between repositories)."""
from pathlib import Path
import runpy
_IMPL = Path(__file__).resolve().parents[2] / "crates" / "rez-next-python" / "python" / "rez_next" / "package_copy.py"
globals().update(runpy.run_path(str(_IMPL)))
