"""Bridge to rez_next bundle_context module (relocatable context bundles)."""
from pathlib import Path
import runpy
_IMPL = Path(__file__).resolve().parents[2] / "crates" / "rez-next-python" / "python" / "rez_next" / "bundle_context.py"
globals().update(runpy.run_path(str(_IMPL)))
