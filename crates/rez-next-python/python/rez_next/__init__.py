import rez_next._native as _native  # noqa: F401
from rez_next._native import *  # noqa: F401,F403

__version__: str = _native.__version__
__author__: str = _native.__author__

# Top-level singletons — compatible with `from rez import config` and `from rez import system`
config = _native.Config()
system = _native.System()

# Aliases for API compatibility with original rez
resolve = _native.resolve_packages
create_context = _native.ResolvedContext


# ── Monkeypatch: add dict-style access to Rust classes ─────────────────────
# (PyO3 classes can't easily have Python-side methods, so we wrap them)


def _add_dict_access_to_package():
    """Add __getitem__ to Package class for rez API compatibility."""
    original_package_class = _native.Package

    def package_getitem(self, key):
        if key == "name":
            return self.name
        elif key == "version":
            return self.version_str
        elif key == "version_str":
            return self.version_str
        elif key == "description":
            return self.description
        elif key == "authors":
            return self.authors
        elif key == "requires":
            return self.requires
        elif key == "build_requires":
            return self.build_requires
        elif key == "variants":
            return self.variants
        elif key == "tools":
            return self.tools
        elif key == "uuid":
            return self.uuid
        elif key == "timestamp":
            return self.timestamp
        elif key == "cachable":
            return self.cachable
        elif key == "relocatable":
            return self.relocatable
        else:
            raise KeyError(f"Package has no attribute '{key}'")

    original_package_class.__getitem__ = package_getitem


def _add_dict_access_to_resolved_context():
    """Add get() to ResolvedContext class for rez API compatibility."""
    original_context_class = _native.ResolvedContext

    def context_get(self, key, default=None):
        if key == "success":
            return self.success
        elif key == "packages":
            return self.resolved_packages
        elif key == "resolved_packages":
            return self.resolved_packages
        elif key == "id":
            return self.id
        elif key == "created_at":
            return self.created_at
        elif key == "num_resolved_packages":
            return self.num_resolved_packages
        else:
            if default is not None:
                return default
            raise KeyError(f"ResolvedContext has no attribute '{key}'")

    original_context_class.get = context_get


# Apply monkeypatches
_add_dict_access_to_package()
_add_dict_access_to_resolved_context()
