"""Rez-compatible status module.

Provides environment status inspection aligning with ``rez.status`` API:

- ``Status`` class with ``print_info()`` and ``print_tools()``
- ``status`` singleton: ``from rez_next.status import status``
"""
from __future__ import annotations

import os
import sys
import rez_next._native  # ensure extension module is initialized  # noqa: F401
from rez_next._native.status import *  # noqa: F401,F403

# ── Python-level wrappers for Rez API alignment ──────────────────────


def _rez_status_print_info(self, obj=None, buf=None):
    """Print status information about the current Rez environment.

    Args:
        obj: Optional - tool name, package name, context path, suite path,
             or context name within a visible suite. If None, prints a
             general environment overview.
        buf: Output stream (defaults to sys.stdout).

    Returns:
        True if info was printed successfully, False otherwise.

    Rez API: ``rez.status.Status.print_info()``
    """
    if buf is None:
        buf = sys.stdout

    if obj is None:
        # General environment overview
        buf.write("rez-next environment status:\n")
        buf.write(f"  Rez version:     {self.rez_version}\n")
        buf.write(f"  Active context:  {self.context_file or '(none)'}\n")
        buf.write(f"  Resolved pkgs:   {len(self.resolved_packages or [])}\n")
        if self.is_active:
            buf.write("  Status:          Active\n")
        else:
            buf.write("  Status:          Inactive\n")
        return True

    obj_str = str(obj)

    # Try tool name
    try:
        from rez_next.suite import Suite  # type: ignore[import-untyped]
        if os.path.exists(obj_str):
            is_suite = Suite.is_suite(obj_str)
            if is_suite:
                suite = Suite()
                suite.load(obj_str)
                buf.write(f"Suite: {suite.description}\n")
                buf.write(f"  Tools: {len(suite.get_tools())}\n")
                return True
    except Exception:
        pass

    buf.write(f"Rez does not know what '{obj_str}' is\n")
    return False


def _rez_status_print_tools(self, pattern=None, buf=None):
    """Print the list of currently visible tools.

    Args:
        pattern: Optional glob pattern to filter tools (e.g. 'python*').
        buf: Output stream (defaults to sys.stdout).

    Returns:
        True if matching tools were found, False otherwise.

    Rez API: ``rez.status.Status.print_tools()``
    """
    if buf is None:
        buf = sys.stdout

    from rez_next.suite import Suite, SuiteManager  # type: ignore[import-untyped]

    tools = []
    seen = set()

    # Collect tools from visible suites
    sm = SuiteManager()
    suite_names = sm.list_suite_names()
    for name in suite_names:
        try:
            suite = sm.load_suite(name)
            for tool_info in suite.get_tools():
                tname = tool_info.get("name", tool_info.get("alias", ""))
                if pattern and pattern not in tname:
                    continue
                if tname not in seen:
                    seen.add(tname)
                    tools.append((tname, name, tool_info.get("package", "")))
        except Exception:
            pass

    if not tools:
        buf.write("No tools found.\n")
        return False

    # Print header
    buf.write(f"{'TOOL':<25} {'SUITE':<20} {'PACKAGE':<20}\n")
    buf.write("-" * 65 + "\n")
    for tname, sname, pkg in sorted(tools):
        buf.write(f"{tname:<25} {sname:<20} {pkg:<20}\n")

    return bool(tools)


# Patch the native RezStatus class with Python-level API methods
try:
    RezStatus.print_info = _rez_status_print_info  # type: ignore[name-defined]
    RezStatus.print_tools = _rez_status_print_tools  # type: ignore[name-defined]
except NameError:
    pass  # RezStatus not yet available

# Singleton matching rez.status API: `from rez.status import status`
status: "RezStatus" = RezStatus()  # type: ignore[name-defined, misc]
