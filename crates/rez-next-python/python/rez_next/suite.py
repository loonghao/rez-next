"""Rez-compatible suite module.

Aligns with ``rez.suite`` API:

- ``Suite`` — suite management class
- ``SuiteManager`` — suite discovery and loading
- ``get_suites()`` — list available suites
- ``get_tools()`` — list tools across suites
- ``get_suite_from_tool()`` — find suite containing a tool
"""

from __future__ import annotations

import os

import rez_next._native  # ensure extension module is initialized  # noqa: F401
from rez_next._native.suite import Suite, SuiteManager

# ── Module-level functions for rez API alignment ─────────────────────


def get_suites(paths: list[str] | None = None) -> list[Suite]:
    """Return all available suites visible on the given paths.

    Args:
        paths: List of directory paths to search. If None, uses PATH.

    Returns:
        List of Suite instances.

    Rez API: ``rez.suite.get_suites()``
    """
    manager = SuiteManager()
    if paths:
        for p in paths:
            manager.add_path(p)

    suites: list[Suite] = []
    for name in manager.list_suite_names():
        try:
            suite = manager.load_suite(name)
            suites.append(suite)
        except Exception:
            pass
    return suites


def get_tools(
    pattern: str | None = None,
) -> list[dict[str, str]]:
    """Return tools available across all visible suites.

    Args:
        pattern: Optional glob pattern to filter tool names.

    Returns:
        List of tool dicts with keys: name, suite, package.

    Rez API: ``rez.suite.get_tools()``
    """
    manager = SuiteManager()
    tools: list[dict[str, str]] = []
    seen: set[str] = set()

    for name in manager.list_suite_names():
        try:
            suite = manager.load_suite(name)
            for tool_info in suite.get_tools():
                tname = tool_info.get("name", tool_info.get("alias", ""))
                if pattern and pattern not in tname:
                    continue
                if tname not in seen:
                    seen.add(tname)
                    tools.append(
                        {
                            "name": tname,
                            "suite": name,
                            "package": tool_info.get("package", ""),
                        }
                    )
        except Exception:
            pass

    return tools


def get_suite_from_tool(tool_name: str) -> Suite | None:
    """Find the suite containing the given tool.

    Args:
        tool_name: Name of the tool to search for.

    Returns:
        Suite instance containing the tool, or None if not found.

    Rez API: ``rez.suite.get_suite_from_tool()``
    """
    manager = SuiteManager()

    for name in manager.list_suite_names():
        try:
            suite = manager.load_suite(name)
            for tool_info in suite.get_tools():
                tname = tool_info.get("name", tool_info.get("alias", ""))
                if tname == tool_name or os.path.basename(tname) == tool_name:
                    return suite
        except Exception:
            pass

    return None
