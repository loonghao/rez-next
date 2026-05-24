"""Bridge to rez_next rezconfig module (configuration defaults).

Aligns with rez.rezconfig API:
- Contains all configuration default values matching Rez's ``src/rez/rezconfig.py``
- Users can ``from rez import rezconfig`` to inspect default config values

See https://github.com/AcademySoftwareFoundation/rez/blob/main/src/rez/rezconfig.py
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "rezconfig.py"
)
globals().update(runpy.run_path(str(_IMPL)))
