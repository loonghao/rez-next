"""Bridge to rez_next env module (environment utilities).

Aligns with rez.env API:
- ``Environ`` — environment manipulation
- ``EnvBindings`` — environment bindings
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "env.py"
)
globals().update(runpy.run_path(str(_IMPL)))
