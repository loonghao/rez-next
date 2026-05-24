"""Bridge to rez_next release module (release utilities).

Aligns with rez.release API:
- ``release_package()`` — release a package
- ``Release`` — release process class
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "release.py"
)
globals().update(runpy.run_path(str(_IMPL)))
