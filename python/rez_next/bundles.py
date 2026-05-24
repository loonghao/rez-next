"""Bridge to rez_next bundles module (context bundle support).

Aligns with rez.bundles API:
- ``bundle_context()`` — create a relocatable context bundle
- ``BundleContext`` — bundle context class
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "bundles.py"
)
globals().update(runpy.run_path(str(_IMPL)))
