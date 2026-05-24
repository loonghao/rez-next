"""Bridge to rez_next package_cache module (package payload caching).

Aligns with rez.package_cache API:
- ``PackageCache`` — multi-backend package cache
- ``CachedPackage`` — cached package entry with TTL support
- ``CacheStats`` — cache performance statistics
- ``InMemoryCache`` — pure in-memory package cache
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "package_cache.py"
)
globals().update(runpy.run_path(str(_IMPL)))
