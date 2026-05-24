"""Bridge to rez_next system module (platform, arch, OS, hostname).

Aligns with rez.system API:
- `from rez.system import system` — singleton System instance
- `system.platform` — current platform (e.g. windows, linux, osx)
- `system.arch` — current architecture (e.g. x86_64)
- `system.os` — current operating system
- `system.hostname` — machine hostname
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "system.py"
)
globals().update(runpy.run_path(str(_IMPL)))
