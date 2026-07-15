"""Rez-compatible resolved_context module.

Aligns with ``rez.resolved_context`` API:

- ``ResolvedContext`` — resolved dependency environment (native + Python wrappers)
- ``RezToolsVisibility`` — enum controlling rez CLI visibility in resolved env
- ``SuiteVisibility`` — enum controlling suite visibility in resolved env
- ``PatchLock`` — enum for patch-level version locking
- ``get_lock_request()`` — generate a locked package request from version + lock
"""

from __future__ import annotations

import os
import shutil
from enum import Enum
from typing import Any

import rez_next._native  # ensure extension module is initialized  # noqa: F401

# Re-export everything from native for backward compatibility
from rez_next._native.resolved_context import *  # noqa: F401,F403

# Import native ResolvedContext class for monkey-patching
from rez_next._native.resolved_context import ResolvedContext as _NativeResolvedContext

# ── Enums (pure Python, no native equivalent) ─────────────────────────


class RezToolsVisibility(Enum):
    """Determines if/how rez CLI tools are added back to PATH within a
    resolved environment.

    Rez API: ``rez.resolved_context.RezToolsVisibility``
    """

    #: Don't expose rez in resolved env
    never = 0
    #: Append to PATH in resolved env
    append = 1
    #: Prepend to PATH in resolved env
    prepend = 2


class SuiteVisibility(Enum):
    """Defines what suites on $PATH stay visible when a new rez environment is
    resolved.

    Rez API: ``rez.resolved_context.SuiteVisibility``
    """

    #: Don't attempt to keep any suites visible in a new env
    never = 0
    #: Keep suites visible in any new env
    always = 1
    #: Keep only the parent suite of a tool visible
    parent = 2
    #: Keep all suites visible and the parent takes precedence
    parent_priority = 3


class PatchLock(Enum):
    """Enum to represent the 'lock type' used when patching context objects.

    Rez API: ``rez.resolved_context.PatchLock``
    """

    no_lock = ("No locking", -1)
    lock_2 = ("Minor version updates only (X.*)", 1)
    lock_3 = ("Patch version updates only (X.X.*)", 2)
    lock_4 = ("Build version updates only (X.X.X.*)", 3)
    lock = ("Exact version", -1)

    def __init__(self, description: str, rank: int) -> None:
        self.description = description
        self.rank = rank


# ── Standalone functions ─────────────────────────────────────────────


def get_lock_request(name: str, version, patch_lock: PatchLock, weak: bool = True):
    """Given a package name, version and patch lock type, return the
    equivalent package request.

    For example, for ``name='foo'``, ``version='1.2.1'`` and
    ``patch_lock=PatchLock.lock_3``, the equivalent request is
    ``'~foo-1.2'``, restricting updates to patch-or-lower changes only.

    Args:
        name: Package name.
        version: Package version (rez Version object or string).
        patch_lock: Lock type to apply.
        weak: If True (default), prefix the request with ``~`` (weak).

    Returns:
        A package request string, or None if there is no equivalent request
        (e.g. when ``patch_lock == PatchLock.no_lock``).
    """
    from rez_next._native.vendor.version import Version

    if isinstance(version, str):
        version = Version(version)

    ch = "~" if weak else ""
    if patch_lock == PatchLock.lock:
        return f"{ch}{name}=={str(version)}"
    elif patch_lock == PatchLock.no_lock or not version:
        return None

    version_ = version.trim(patch_lock.rank)
    s = f"{ch}{name}-{str(version_)}"
    return s


# ── Resolution diff (pure Python, no native equivalent yet) ──────────


def diff_contexts(request_a: list[str], request_b: list[str]) -> dict[str, Any]:
    """Compare two sets of package requests and return a diff dictionary.

    This is a lightweight Python-level implementation that mirrors
    ``rez.diff.diff_contexts()``.

    Args:
        request_a: First set of package request strings.
        request_b: Second set of package request strings.

    Returns:
        Dict with keys: ``added``, ``removed``, ``changed``, ``unchanged``.
    """

    def _parse(s):
        parts = s.split("-", 1)
        name = parts[0]
        version = parts[1] if len(parts) > 1 else ""
        return name, version

    def _name_only(s):
        return s.split("-", 1)[0] if "-" in s else s

    pkgs_a = {_name_only(s): _parse(s) for s in request_a if s}
    pkgs_b = {_name_only(s): _parse(s) for s in request_b if s}

    names_a = set(pkgs_a.keys())
    names_b = set(pkgs_b.keys())

    added = []
    removed = []
    changed = []
    unchanged = []

    for name in sorted(names_b - names_a):
        added.append(pkgs_b[name])
    for name in sorted(names_a - names_b):
        removed.append(pkgs_a[name])
    for name in sorted(names_a & names_b):
        if pkgs_a[name][1] != pkgs_b[name][1]:
            changed.append((pkgs_a[name], pkgs_b[name]))
        else:
            unchanged.append(pkgs_a[name])

    return {
        "added": added,
        "removed": removed,
        "changed": changed,
        "unchanged": unchanged,
    }


# ── Helper functions for monkey-patched methods ──────────────────────


def _req_name(requirement):
    """Extract package name from a requirement string."""
    requirement = str(requirement)
    if requirement.startswith("!"):
        return None
    for sep in ("<", ">", "=", " "):
        requirement = requirement.split(sep, 1)[0]
    return requirement.split("-", 1)[0]


def _pkg_node(pkg):
    """Create a DOT node label for a package."""
    version = getattr(pkg, "version_str", None)
    if version is None:
        version = getattr(pkg, "version", None)
    return f"{getattr(pkg, 'name', pkg)}-{version}" if version else str(getattr(pkg, "name", pkg))


def _resolve_as_exact_requests(self):
    """Convert resolved packages to exact version requests.

    Rez API: ``context.get_resolve_as_exact_requests()``
    """
    result = []
    for pkg in getattr(self, "resolved_packages", []) or []:
        name = getattr(pkg, "name", "")
        version = getattr(pkg, "version_str", getattr(pkg, "version", None))
        if name and version is not None:
            result.append(f"{name}=={version}")
    return result


# ── Monkey-patch methods onto native ResolvedContext ─────────────────


def _context_copy(self):
    """Return a shallow copy of this context.

    Rez API: ``context.copy()``
    """
    import copy as _copy

    return _copy.copy(self)


def _context_validate(self):
    """Validate the context's package data.

    Raises ``RuntimeError`` if the context is invalid.

    Rez API: ``context.validate()``
    """
    if not self.success:
        raise RuntimeError(
            "Cannot validate a failed context: %s" % (self.failure_description or "unknown")
        )

    # Check that all resolved packages exist and have required fields
    for pkg in getattr(self, "resolved_packages", []) or []:
        name = getattr(pkg, "name", None)
        if not name:
            raise RuntimeError("Found a package with no name in resolved context")
        version = getattr(pkg, "version", None) or getattr(pkg, "version_str", None)
        if not version:
            raise RuntimeError(f"Package {name} has no version in resolved context")


def _context_which(self, cmd, parent_environ=None, fallback=False):
    """Find a program in the resolved environment.

    Searches ``PATH`` from the context's environment variables.

    Args:
        cmd: Program name to find.
        parent_environ: Environment dict to use (defaults to context env).
        fallback: If True, fall back to system ``shutil.which()``.

    Returns:
        Full path to the program, or None if not found.

    Rez API: ``context.which(cmd, parent_environ=None, fallback=False)``
    """
    if parent_environ is None:
        try:
            parent_environ = self.get_environ()
        except Exception:
            parent_environ = os.environ

    path = parent_environ.get("PATH", os.environ.get("PATH", ""))
    for dir_path in path.split(os.pathsep):
        if not dir_path:
            continue
        full_path = os.path.join(dir_path, cmd)
        if os.path.isfile(full_path) and os.access(full_path, os.X_OK):
            return full_path

    if fallback:
        return shutil.which(cmd)
    return None


def _context_get_dependency_graph(self, as_dot=False):
    """Generate the dependency graph.

    Returns a dict with ``nodes`` and ``edges``, or a DOT string if
    ``as_dot`` is True.

    Args:
        as_dot: If True, return a DOT format string.

    Returns:
        Dict or DOT string.

    Rez API: ``context.get_dependency_graph(as_dot=False)``
    """
    packages = list(getattr(self, "resolved_packages", []) or [])
    by_name = {}
    nodes = []
    for pkg in packages:
        name = getattr(pkg, "name", "")
        version = getattr(pkg, "version_str", getattr(pkg, "version", None))
        by_name[name] = pkg
        nodes.append({"name": name, "version": str(version) if version is not None else None})

    edges = []
    for pkg in packages:
        source = getattr(pkg, "name", "")
        for req in getattr(pkg, "requires", []) or []:
            req_name = _req_name(req)
            if req_name and req_name in by_name:
                target = getattr(by_name[req_name], "name", req_name)
                edges.append({"from": source, "to": target})

    if as_dot:
        lines = [
            "digraph dependency_graph {",
            "  rankdir=LR;",
            "  node [shape=box, style=filled, fillcolor=lightblue];",
        ]
        for node in nodes:
            label = (
                "{}-{}".format(node["name"], node["version"]) if node["version"] else node["name"]
            )
            lines.append(f'  "{label}";')
        for edge in edges:
            from_label = "{}-{}".format(
                edge["from"],
                by_name[edge["from"]].version_str
                if hasattr(by_name[edge["from"]], "version_str")
                and by_name[edge["from"]].version_str
                else edge["from"],
            )
            to_label = "{}-{}".format(
                edge["to"],
                by_name[edge["to"]].version_str
                if hasattr(by_name[edge["to"]], "version_str") and by_name[edge["to"]].version_str
                else edge["to"],
            )
            lines.append(f'  "{from_label}" -> "{to_label}";')
        lines.append("}")
        return "\n".join(lines)

    return {"nodes": nodes, "edges": edges}


def _context_graph(self, as_dot=False):
    """Get the resolution graph.

    Delegates to ``get_dependency_graph()``.

    Args:
        as_dot: If True, return a DOT format string.

    Returns:
        Dict or DOT string.

    Rez API: ``context.graph(as_dot=False)``
    """
    return self.get_dependency_graph(as_dot=as_dot)


def _context_failed(self):
    """Whether the context failed to resolve."""
    return not self.success


def _context_requirements(self):
    """Get the requirements list for this context.

    Returns a list of requirement strings (e.g. ``["python-3.9", "maya-2024"]``).
    """
    reqs = getattr(self, "_requested_packages", None) or getattr(self, "requested_packages", None)
    if reqs is None:
        pkgs = getattr(self, "resolved_packages", []) or []
        reqs = [getattr(p, "name", str(p)) for p in pkgs]
    return list(reqs)


def _context_get_current():
    """Get the current context from the environment.

    Checks the ``REZ_RXT_FILE`` environment variable.

    Returns:
        ``ResolvedContext`` instance or ``None``.

    Rez API: ``ResolvedContext.get_current()``
    """
    rxt_file = os.environ.get("REZ_RXT_FILE")
    if rxt_file and os.path.exists(rxt_file):
        try:
            return _NativeResolvedContext.load(rxt_file)
        except Exception:
            pass
    return None


def _context_is_current(self):
    """Check if this context is the currently sourced context.

    Compares against the context loaded from ``REZ_RXT_FILE``.

    Returns:
        ``True``, ``False``, or ``None`` if unable to determine.

    Rez API: ``context.is_current()``
    """
    current = _context_get_current()
    if current is None:
        return None
    try:
        return self.to_dict() == current.to_dict()
    except Exception:
        return None


def _context_print_resolve_diff(self, other, heading=None):
    """Print the resolution diff between this context and another.

    Args:
        other: Another ``ResolvedContext`` instance.
        heading: Optional heading string.

    Rez API: ``context.print_resolve_diff(other, heading=None)``
    """
    reqs_self = list(getattr(self, "resolved_packages", []) or [])
    reqs_other = list(getattr(other, "resolved_packages", []) or [])

    def _to_req_str(pkg):
        name = getattr(pkg, "name", str(pkg))
        version = getattr(pkg, "version_str", getattr(pkg, "version", None))
        if version:
            return f"{name}-{str(version)}"
        return name

    diff = diff_contexts(
        [_to_req_str(p) for p in reqs_self],
        [_to_req_str(p) for p in reqs_other],
    )

    if heading:
        output = [heading]
    else:
        output = ["Resolve diff:"]

    if diff.get("added"):
        output.append("  Added packages:")
        for name, ver in diff["added"]:
            output.append(f"    + {name}-{ver}")
    if diff.get("removed"):
        output.append("  Removed packages:")
        for name, ver in diff["removed"]:
            output.append(f"    - {name}-{ver}")
    if diff.get("changed"):
        output.append("  Changed packages:")
        for (name_a, ver_a), (name_b, ver_b) in diff["changed"]:
            output.append(f"    ~ {name_a}: {ver_a} -> {ver_b}")

    text = "\n".join(output)
    print(text)
    return text


def _context_get_resolve_as_exact_requests(self):
    """Convert resolved packages to exact version requests.

    Returns a list of strings like ``['foo==1.2.3', 'bar==4.5.6']``.

    Rez API: ``context.get_resolve_as_exact_requests()``
    """
    return _resolve_as_exact_requests(self)


def _context_to_dot(self):
    """Generate a DOT graph of the resolved context.

    Rez API: ``context.to_dot()``
    """
    packages = list(getattr(self, "resolved_packages", []) or [])
    lines = [
        "digraph resolved_context {",
        "  rankdir=LR;",
        "  node [shape=box, style=filled, fillcolor=lightblue];",
    ]
    by_name = {getattr(pkg, "name", ""): pkg for pkg in packages}
    for pkg in packages:
        lines.append(f'  "{_pkg_node(pkg)}";')
    for pkg in packages:
        source = _pkg_node(pkg)
        for req in getattr(pkg, "requires", []) or []:
            name = _req_name(req)
            if name and name in by_name:
                lines.append(f'  "{source}" -> "{_pkg_node(by_name[name])}";')
    lines.append("}")
    return "\n".join(lines)


# ── Apply all monkey-patches ─────────────────────────────────────────


def _apply_patches():
    """Apply all missing methods onto the native ResolvedContext class."""
    patches = {
        "copy": _context_copy,
        "validate": _context_validate,
        "which": _context_which,
        "get_dependency_graph": _context_get_dependency_graph,
        "graph": _context_graph,
        "failed": property(_context_failed),
        "requirements": property(_context_requirements),
        "is_current": _context_is_current,
        "to_dot": _context_to_dot,
        "print_resolve_diff": _context_print_resolve_diff,
        "get_resolve_as_exact_requests": _context_get_resolve_as_exact_requests,
    }

    for name, impl in patches.items():
        if not hasattr(_NativeResolvedContext, name):
            setattr(_NativeResolvedContext, name, impl)

    # Classmethods need special handling
    if not hasattr(_NativeResolvedContext, "get_current"):
        _NativeResolvedContext.get_current = classmethod(lambda cls: _context_get_current())


_apply_patches()
