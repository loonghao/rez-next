"""Bridge to rez_next package_search module (package search and discovery).

Aligns with rez.package_search API:
- ``PackageSearcher`` — search packages using various criteria
- ``SearchResult`` — individual search result entry
- ``search_packages()`` — search all packages matching criteria
- ``search_latest_packages()`` — search for latest versions
- ``search_package_names()`` — search for package family names
- ``ResourceSearchResult`` — reverse dependency tree result
- ``get_reverse_dependency_tree()`` — get reverse dependency chain
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "package_search.py"
)
globals().update(runpy.run_path(str(_IMPL)))
