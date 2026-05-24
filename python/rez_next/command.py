"""Bridge to rez_next command module (shell command execution)."""
from pathlib import Path
import runpy
_IMPL = Path(__file__).resolve().parents[2] / "crates" / "rez-next-python" / "python" / "rez_next" / "command.py"
globals().update(runpy.run_path(str(_IMPL)))
