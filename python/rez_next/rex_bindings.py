"""Bridge to rez_next rex_bindings module (Rex execution bindings)."""
from pathlib import Path
import runpy
_IMPL = Path(__file__).resolve().parents[2] / "crates" / "rez-next-python" / "python" / "rez_next" / "rex_bindings.py"
globals().update(runpy.run_path(str(_IMPL)))
