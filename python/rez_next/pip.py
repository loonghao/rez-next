"""Bridge to rez_next pip module (pip integration).

Aligns with rez.pip API:
- ``PipIntegration`` — pip-based package installation
- ``install_package()`` — install a pip package as a rez package
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "pip.py"
)
globals().update(runpy.run_path(str(_IMPL)))
