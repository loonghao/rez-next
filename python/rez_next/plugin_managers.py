"""Bridge to rez_next plugin_managers module (plugin discovery)."""
from pathlib import Path
import runpy
_IMPL = Path(__file__).resolve().parents[2] / "crates" / "rez-next-python" / "python" / "rez_next" / "plugin_managers.py"
globals().update(runpy.run_path(str(_IMPL)))
