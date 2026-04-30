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


def _add_dict_access_to_package() -> None:
    """Add __getitem__ to Package class for rez API compatibility."""
    original_package_class = _native.Package

    def package_getitem(self: _native.Package, key: str) -> object:
        """Dict-style access for Package attributes."""
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


def _add_dict_access_to_resolved_context() -> None:
    """Add get() to ResolvedContext class for rez API compatibility."""
    original_context_class = _native.ResolvedContext

    def context_get(
        self: _native.ResolvedContext, key: str, default: object = None
    ) -> object:
        """Dict-style get() for ResolvedContext attributes."""
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


# ── Add to_dot() for dependency graph visualization ─────────────────────
def _add_dot_visualization() -> None:
    """Add to_dot() to ResolvedContext for dependency graph visualization."""
    original_context_class = _native.ResolvedContext

    def to_dot(self: _native.ResolvedContext) -> str:
        """
        Generate a Graphviz DOT representation of the resolved context.

        Returns:
            str: DOT format graph string suitable for graphviz rendering

        Example:
            >>> ctx = rez.resolve_packages(["python-3.9"])
            >>> dot = ctx.to_dot()
            >>> print(dot)
        """
        lines: list[str] = []
        lines.append("digraph ResolvedContext {")
        lines.append("  rankdir=LR;")
        lines.append("  node [shape=box, style=filled, fillcolor=lightblue];")

        # Add nodes
        for pkg in self.resolved_packages:
            lines.append(f'  "{pkg.name}-{pkg.version_str}";')

        # Add edges (dependencies)
        for pkg in self.resolved_packages:
            if pkg.requires:
                for req in pkg.requires:
                    # Parse requirement to get package name (handle version specifiers)
                    req_name = req.split()[0] if " " in req else req
                    # Strip version specifiers like >=, ==, etc.
                    import re
                    req_name = re.split(r"[<>=!~]", req_name)[0]

                    # Find matching resolved package
                    for other in self.resolved_packages:
                        if other.name == req_name:
                            lines.append(
                                f'  "{pkg.name}-{pkg.version_str}" -> '
                                f'"{other.name}-{other.version_str}";'
                            )
                            break

        lines.append("}")
        return "\n".join(lines)

    original_context_class.to_dot = to_dot


_add_dot_visualization()
