"""Bridge to rez_next search module (package search).

Aligns with rez.search API:
- ``search_packages()`` — search for packages by pattern
- ``PackageSearchResult`` — search result entry
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "search.py"
)
globals().update(runpy.run_path(str(_IMPL)))
