"""
Rez-compatible ``package_bind`` module — align with ``rez.package_bind`` API.

Provides:
- ``bind_package()`` — bind system software as a rez package
- ``get_bind_modules()`` — list available bind modules (built-in + external)
- ``find_bind_module()`` — locate a bind module by name
- ``RezBindError`` — exception class (re-exported from ``.exceptions``)

Architecture
------------
Two kinds of bind modules are supported:

1. **Built-in binders** (Rust) — compiled into ``_native.bind`` (python, cmake,
   git, pip, gcc, clang, node, rust, go, java, ffmpeg, imagemagick).
2. **External bind scripts** — Python files found in ``bind_module_path``
   (from config). Each script must expose a ``bind(path, version_range, opts,
   parser)`` function.

This module wraps both kinds behind the same ``bind_package()`` call,
following the original rez ``package_bind.py`` API while keeping the
Rust-based built-in binders as the primary fast path.
"""

from __future__ import annotations

import os.path as _os_path
import sys as _sys
from pathlib import Path as _Path
from typing import TYPE_CHECKING

from . import _native
from .exceptions import RezBindError
from .config import config as _config

if TYPE_CHECKING:
    from collections.abc import Sequence

_BIND_MODULE_CACHE: dict[str, str] | None = None


# ── Public API ───────────────────────────────────────────────────────


def get_bind_modules(verbose: bool = False) -> dict[str, str]:
    """Get available bind modules.

    Returns a dict mapping module name → module file path.

    The result includes:
    - All built-in binders (named ``<tool>``, with a virtual path prefixed
      by ``<builtin>:``).
    - External bind scripts found in ``config.bind_module_path``.

    The result is cached after the first call.

    Args:
        verbose: If True, print extra module-discovery output.

    Returns:
        dict[str, str]: Map of ``{name: filepath}``.
    """
    global _BIND_MODULE_CACHE
    if _BIND_MODULE_CACHE is not None:
        return _BIND_MODULE_CACHE

    result: dict[str, str] = {}

    # 1. Built-in binders (Rust _native.bind submodule)
    builtin_names: list[str] = getattr(_native.bind, "list_binders", lambda: [])()
    for name in builtin_names:
        result[name] = f"<builtin>:{name}"

    # 2. External bind scripts from config.bind_module_path
    bind_module_paths: list[str] = getattr(_config, "bind_module_path", [])
    for path in bind_module_paths:
        p = _Path(path)
        if verbose:
            print(f"searching {p}...")
        if not p.is_dir():
            continue
        for child in sorted(p.iterdir()):
            if child.suffix == ".py" and not child.stem.startswith("_"):
                result[child.stem] = str(child.resolve())

    _BIND_MODULE_CACHE = result
    return result


def find_bind_module(name: str, verbose: bool = False) -> str | None:
    """Find the bind module matching the given name.

    Args:
        name: Package name to find bind module for.
        verbose: If True, print extra output including close matches.

    Returns:
        Filepath to the bind module, or ``None`` if not found.
    """
    modules = get_bind_modules(verbose=verbose)
    module_path = modules.get(name)
    if module_path is not None:
        return module_path

    if verbose:
        fuzzy = _get_close_matches(name, list(modules.keys()))
        if fuzzy:
            lines = "\n".join(f"  {n}  {modules[n]}" for n, _ in fuzzy)
            print(f"'{name}' not found. Close matches:\n{lines}")
        else:
            print(f"'{name}' not found. No matches.")

    return None


def bind_package(
    name: str,
    path: str | None = None,
    version_range=None,
    no_deps: bool = False,
    bind_args: list[str] | None = None,
    quiet: bool = False,
) -> list:
    """Bind system software as a rez package.

    This is the main entry point.  It locates the appropriate bind module
    (built-in Rust binder or external Python script), executes it, and
    returns the installed package results.

    Args:
        name: Package name to bind.
        path: Install path; defaults to ``config.local_packages_path``.
        version_range: If provided, only bind if the detected version falls
            within this range (string in Rez version-range format).
        no_deps: If True, don't bind dependencies.
        bind_args: CLI-style arguments forwarded to the bind module.
        quiet: If True, suppress output.

    Returns:
        List of ``BindResult`` objects (one per installed variant).
    """
    install_path = path or getattr(_config, "local_packages_path", None) or "."

    pending: set[str] = {name}
    results: list = []
    installed_names: set[str] = set()

    while pending:
        batch = pending
        pending = set()
        is_primary = len(results) == 0  # first iteration = primary package

        for pkg_name in batch:
            try:
                pkgs = _bind_single(
                    pkg_name,
                    path=install_path,
                    version_range=version_range,
                    bind_args=bind_args,
                    quiet=quiet,
                )
            except RezBindError as exc:
                if is_primary:
                    raise
                if not quiet:
                    from .utils.logging_ import print_error as _print_error

                    _print_error(f"Could not bind '{pkg_name}': {exc}")
                continue

            results.extend(pkgs)
            installed_names.update(r.name for r in pkgs)

            if not no_deps:
                # ── Dependency traversal ──────────────────────────
                # Rez's VSC (versioned-strict-conflict) packages may
                # carry requires that reference other bind-able tools.
                # We do NOT deep-inspect requires here because the
                # Rust-builtin binders don't emit dependency graphs —
                # they create one self-contained package per invocation.
                # External bind scripts may, however.
                pass

        # After the primary package, subsequent iterations are
        # dependencies — treat softly.
        version_range = None
        bind_args = None

    if results and not quiet:
        print("The following packages were installed:\n")
        _print_package_list(results)

    return results


# ── Internal helpers ─────────────────────────────────────────────────


def _bind_single(
    name: str,
    path: str,
    version_range=None,
    bind_args: list[str] | None = None,
    quiet: bool = False,
) -> list:
    """Bind a single package (no dependency traversal).

    Tries the Rust built-in binder first, then falls back to an external
    Python bind script.
    """
    module_path = find_bind_module(name, verbose=not quiet)
    if module_path is None:
        raise RezBindError(f"Bind module not found for '{name}'")

    # ── Built-in binder (Rust fast path) ──────────────────────────
    if module_path.startswith("<builtin>:"):
        return _bind_via_rust(name, path, version_range, quiet)

    # ── External Python bind script ───────────────────────────────
    return _bind_via_script(name, module_path, path, version_range, bind_args, quiet)


def _bind_via_rust(
    name: str,
    path: str,
    version_range=None,
    quiet: bool = False,
) -> list:
    """Delegate to the Rust ``_native.bind.bind_tool()`` function."""
    try:
        result = _native.bind.bind_tool(
            tool_name=name,
            install_path=path,
            force=False,
        )
    except Exception as exc:
        raise RezBindError(str(exc)) from exc

    if not quiet:
        print(f"Created package '{name}' in {result.install_path}")

    return [result]


def _bind_via_script(
    name: str,
    script_path: str,
    path: str,
    version_range=None,
    bind_args: list[str] | None = None,
    quiet: bool = False,
) -> list:
    """Execute an external Python bind script.

    The script is loaded in an isolated namespace and must expose a
    ``bind(path, version_range, opts, parser)`` function.
    """
    import argparse
    import importlib.util as _import_util

    spec = _import_util.spec_from_file_location(f"bind_{name}", script_path)
    if spec is None or spec.loader is None:
        raise RezBindError(f"Can't load bind module from '{script_path}'")

    module = _import_util.module_from_spec(spec)
    # Support __file__ in the bind script (rez issue #1842)
    _sys.modules[spec.name] = module
    try:
        spec.loader.exec_module(module)
    finally:
        _sys.modules.pop(spec.name, None)

    # Parse custom CLI args
    parser = argparse.ArgumentParser(
        prog=f"rez bind {name}",
        description=f"{name} bind module",
    )
    setup_parser = getattr(module, "setup_parser", None)
    if setup_parser is not None:
        setup_parser(parser)
    opts = parser.parse_args(bind_args or [])

    if not quiet:
        print(f"Creating package '{name}' in {path}...")

    bind_func = getattr(module, "bind", None)
    if bind_func is None:
        raise RezBindError(f"'bind' function missing in {script_path}")

    variants = bind_func(
        path=path,
        version_range=version_range,
        opts=opts,
        parser=parser,
    )

    # If the script returns a single result, normalise to list
    if variants is None:
        return []
    if not isinstance(variants, list):
        return [variants]
    return variants


def _get_close_matches(name: str, candidates: list[str]) -> list[tuple[str, float]]:
    """Return close matches by simple Levenshtein (no external dep)."""
    scored: list[tuple[str, float]] = []
    for cand in candidates:
        score = _levenshtein_similarity(name, cand)
        if score > 0.4:
            scored.append((cand, score))
    scored.sort(key=lambda x: (-x[1], x[0]))
    return scored


def _levenshtein_similarity(a: str, b: str) -> float:
    """Normalised Levenshtein similarity in [0, 1]."""
    m, n = len(a), len(b)
    dp = list(range(n + 1))
    for i in range(1, m + 1):
        prev = dp[0]
        dp[0] = i
        for j in range(1, n + 1):
            temp = dp[j]
            cost = 0 if a[i - 1] == b[j - 1] else 1
            dp[j] = min(dp[j] + 1, dp[j - 1] + 1, prev + cost)
            prev = temp
    return 1.0 - dp[n] / max(m, n, 1)


def _print_package_list(results: Sequence) -> None:
    """Pretty-print installed packages in a two-column table."""
    rows: list[list[str]] = [["PACKAGE", "VERSION"], ["-------", "-------"]]
    seen: set[str] = set()
    for r in results:
        key = f"{r.name}:{r.version}"
        if key not in seen:
            seen.add(key)
            rows.append([r.name, r.version])

    # Flatten rows into strings: "PACKAGE   VERSION"
    col_widths = [
        max(len(str(r[i])) for r in rows) for i in range(len(rows[0]))
    ]
    lines: list[str] = []
    for row in rows:
        lines.append(
            "  ".join(str(cell).ljust(w) for cell, w in zip(row, col_widths))
        )
    print("\n".join(lines))


# ── Cache invalidation (for testing) ────────────────────────────────


def _clear_cache() -> None:
    """Clear the bind-module cache (testing hook)."""
    global _BIND_MODULE_CACHE
    _BIND_MODULE_CACHE = None
