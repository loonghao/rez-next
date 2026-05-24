"""Bridge to rez_next packages module (package iteration and listing).

Aligns with rez.packages API:
- ``iter_packages()`` — iterate over available packages
- ``get_latest_package()`` — get latest version of a package
- ``get_package()`` — get specific package
- ``get_installed_packages()`` — list installed packages
- ``Package`` — package type alias
- ``PackageVariant`` — package variant type
- ``sort_packages()`` — sort packages by version
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "packages.py"
)
globals().update(runpy.run_path(str(_IMPL)))
