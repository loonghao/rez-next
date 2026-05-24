"""Bridge to rez_next package_py_utils module (package.py utilities)."""
from pathlib import Path
import runpy
_IMPL = Path(__file__).resolve().parents[2] / "crates" / "rez-next-python" / "python" / "rez_next" / "package_py_utils.py"
globals().update(runpy.run_path(str(_IMPL)))
