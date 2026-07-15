"""Package search module for rez_next.

This module provides the same API as rez.package_search for drop-in compatibility.
"""

import rez_next._native  # ensure extension module is initialized  # noqa: F401

# Import from native package_search submodule
from rez_next._native.package_search import (  # noqa: F401
    ResourceSearchResult,
    get_plugins,
    get_reverse_dependency_tree,
)
from rez_next._native.search import (  # noqa: F401,F403
    PackageSearcher,
    SearchResult,
    search_latest_packages,
    search_package_names,
    search_packages,
)

# For compatibility with rez.package_search API
# Map rez_next._native.search functions to rez.package_search equivalent names
from rez_next.packages import (  # noqa: F401
    get_latest_package,
    get_package,
    iter_package_families,
    iter_packages,
)

# Expose ResourceSearcher as an alias for PackageSearcher for compatibility
ResourceSearcher = PackageSearcher
# ResourceSearchResult is imported from native package_search submodule above
