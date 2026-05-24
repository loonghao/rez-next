"""Bridge to rez_next package_remove module (package removal).

Aligns with rez.package_remove API:
- ``remove_package()`` — remove a specific package version
- ``remove_package_family()`` — remove an entire package family
- ``remove_packages_ignored_since()`` — remove packages ignored for N days
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "package_remove.py"
)
globals().update(runpy.run_path(str(_IMPL)))
