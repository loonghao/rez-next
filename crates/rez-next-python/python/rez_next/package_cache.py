import rez_next._native  # noqa: F401
from dataclasses import dataclass
import time

from rez_next._native.package_cache import *  # noqa: F401,F403


@dataclass
class CacheStats:
    hits: int = 0
    misses: int = 0
    puts: int = 0
    removes: int = 0


class CachedPackage:
    def __init__(self, name, version, path, data):
        self.name = name
        self.version = version
        self.path = path
        self.data = data
        self.cached_at = time.time()
        self.ttl = None
        try:
            self.source_mtime = __import__("os").path.getmtime(path)
        except OSError:
            self.source_mtime = None

    def is_valid(self, source_mtime=None):
        if self.ttl is not None and time.time() - self.cached_at > self.ttl:
            return False
        if source_mtime is not None and self.source_mtime is not None:
            return source_mtime <= self.source_mtime
        return True


class InMemoryCache:
    def __init__(self):
        self._items = {}
        self._stats = CacheStats()
        self._default_ttl = None

    def get(self, path):
        item = self._items.get(path)
        if item is None or not item.is_valid(None):
            self._stats.misses += 1
            return None
        self._stats.hits += 1
        return item

    def put(self, path, package):
        if package.ttl is None:
            package.ttl = self._default_ttl
        self._items[path] = package
        self._stats.puts += 1

    def remove(self, path):
        self._items.pop(path, None)
        self._stats.removes += 1

    def clear(self):
        self._items.clear()

    def stats(self):
        return self._stats

    def set_default_ttl(self, ttl):
        self._default_ttl = ttl


class PackageCache(InMemoryCache):
    @classmethod
    def new_in_memory(cls):
        return cls()

    @classmethod
    def new_file_based(cls, _path):
        return cls()
