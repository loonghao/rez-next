"""Bridge to rez_next shell module (shell plugins).

Aligns with rez.shell API:
- ``Shell`` — shell plugin base class
- ``create_shell()`` — create a shell instance
- ``get_shell_types()`` — list available shell types
- ``get_shell_class()`` — get a shell class by name
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "shell.py"
)
globals().update(runpy.run_path(str(_IMPL)))
