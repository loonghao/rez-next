"""Rez-compatible util module.

Aligns with ``rez.util`` API by re-exporting native utility functions
and adding Python-level implementations of utility functions not
available in the native layer.
"""

from __future__ import annotations

import atexit
import collections.abc
import difflib
import importlib.util
import inspect
import math
import re
from collections.abc import Iterable
from types import ModuleType
from typing import TypeVar

import rez_next._native  # noqa: F401
import rez_next._native.util as _native_util
from rez_next._native.util import *  # noqa: F401,F403

T = TypeVar("T")

# ── Re-export with upstream-compatible signature ─────────────────────


def which(*programs, **shutilwhich_kwargs) -> str | None:
    """Find the first available program in PATH.

    Wraps the native ``_native.util.which()`` to match the upstream
    ``rez.util.which()`` signature which accepts multiple program names
    and returns the first one found.

    Rez API: ``rez.util.which(*programs, **shutilwhich_kwargs)``
    """
    for cmd in programs:
        result = _native_util.which(cmd)
        if result is not None:
            return result
    return None


# ── Python-level utility functions ───────────────────────────────────


def dedup(seq: Iterable[T]) -> Iterable[T]:
    """Remove duplicates from a list while keeping order.

    Rez API: ``rez.util.dedup()``
    """
    seen: set[T] = set()
    for item in seq:
        if item not in seen:
            seen.add(item)
            yield item


_find_unsafe = re.compile(r"[^\\w@%+=`:,./-]").search


def shlex_join(
    value: Iterable[str],
    unsafe_regex=None,
    replacements: Iterable[tuple[str | re.Pattern, str]] | None = None,
    enclose_with: str = '"',
) -> str:
    """Join args into a valid shell command.

    Rez API: ``rez.util.shlex_join()``
    """
    if not is_non_string_iterable(value):
        return str(value)

    unsafe_regex = unsafe_regex or _find_unsafe

    def escape_word(s):
        if not s:
            return "''"
        if unsafe_regex(s) is None:
            return s
        for from_, to_ in replacements or []:
            if isinstance(from_, str):
                s = s.replace(from_, to_)
            else:
                s = from_.sub(to_, s)
        return enclose_with + s + enclose_with

    return " ".join(escape_word(x) for x in value)


def get_close_matches(
    term: str,
    fields: list[str],
    fuzziness: float = 0.4,
    key=None,
) -> list[tuple[str, float]]:
    """Case-insensitive fuzzy string match.

    Rez API: ``rez.util.get_close_matches()``
    """
    term = term.lower()

    def _ratio(a, b):
        return difflib.SequenceMatcher(None, a, b).ratio()

    matches = []
    for field in fields:
        fld = field if key is None else key(field)
        if term == fld:
            matches.append((field, 1.0))
        else:
            name = fld.lower()
            r = _ratio(term, name)
            if name.startswith(term):
                r = math.pow(r, 0.3)
            elif term in name:
                r = math.pow(r, 0.5)
            if r >= (1.0 - fuzziness):
                matches.append((field, min(r, 0.99)))

    return sorted(matches, key=lambda x: -x[1])


def get_close_pkgs(
    pkg: str,
    pkgs: list[str],
    fuzziness: float = 0.4,
) -> list[tuple[str, float]]:
    """Fuzzy string matching on package names.

    Rez API: ``rez.util.get_close_pkgs()``
    """
    matches = get_close_matches(pkg, pkgs, fuzziness=fuzziness)
    fam_matches = get_close_matches(
        pkg.split("-")[0],
        pkgs,
        fuzziness=fuzziness,
        key=lambda x: x.split("-")[0],
    )

    d: dict[str, float] = {}
    for pkg_, r in matches + fam_matches:
        d[pkg_] = d.get(pkg_, 0.0) + r

    combined = [(k, v * 0.5) for k, v in d.items()]
    return sorted(combined, key=lambda x: -x[1])


def find_last_sublist(list_, sublist):
    """Find the last occurrence of a sublist within a list.

    Returns the index where the sublist starts, or None if not found.

    Rez API: ``rez.util.find_last_sublist()``
    """
    for i in reversed(range(len(list_) - len(sublist) + 1)):
        if list_[i] == sublist[0] and list_[i : i + len(sublist)] == sublist:
            return i
    return None


def is_non_string_iterable(arg):
    """Check if arg is an iterable but NOT a string.

    Rez API: ``rez.util.is_non_string_iterable()``
    """
    return isinstance(arg, collections.abc.Iterable) and not isinstance(arg, str)


def get_function_arg_names(func):
    """Get names of a function's positional and keyword-only args.

    Rez API: ``rez.util.get_function_arg_names()``
    """
    spec = inspect.getfullargspec(func)
    return spec.args + spec.kwonlyargs


def load_module_from_file(name: str, filepath: str) -> ModuleType:
    """Load a Python module from a source file without adding to sys.modules.

    Rez API: ``rez.util.load_module_from_file()``
    """
    spec = importlib.util.spec_from_file_location(name, filepath)
    module = importlib.util.module_from_spec(spec)
    if spec and spec.loader:
        spec.loader.exec_module(module)
    return module


def resolve_variant_indices(
    variants: list[int],
    num_variants: int,
) -> tuple[set[int], list[int]]:
    """Resolve possibly-negative variant indices to canonical non-negative ones.

    Rez API: ``rez.util.resolve_variant_indices()``

    Returns ``(resolved_set, invalid_list)`` where invalid contains any
    indices outside the valid range, sorted for deterministic error messages.
    """
    if num_variants <= 0:
        return set(variants), []
    present = set(range(-num_variants, num_variants))
    invalid = sorted(set(variants) - present)
    resolved = {v % num_variants for v in variants}
    return resolved, invalid


# ── Progress bar (wraps tqdm if available, no-op otherwise) ──────────


class ProgressBar:
    """Simple progress bar wrapper.

    Rez API: ``rez.util.ProgressBar``
    """

    def __init__(self, label: str, max: int) -> None:
        self.label = label
        self.max = max
        self._current = 0
        try:
            from tqdm import tqdm

            self._bar = tqdm(total=max, desc=label, unit="item")
            self._use_tqdm = True
        except ImportError:
            self._use_tqdm = False

    def __del__(self) -> None:
        if hasattr(self, "_bar") and hasattr(self._bar, "close"):
            try:
                self._bar.close()
            except Exception:
                pass

    def update(self, n: int = 1) -> None:
        self._current += n
        if self._use_tqdm:
            self._bar.update(n)

    def finish(self) -> None:
        if self._use_tqdm:
            self._bar.close()


# ── Cleanup on exit ─────────────────────────────────────────────────


@atexit.register
def _atexit() -> None:
    """Clean up temporary directories on exit (matches rez.util._atexit)."""
    try:
        from rez_next.resolved_context import ResolvedContext  # type: ignore[import-untyped]

        if hasattr(ResolvedContext, "tmpdir_manager") and hasattr(
            ResolvedContext.tmpdir_manager, "clear"
        ):
            ResolvedContext.tmpdir_manager.clear()
    except Exception:
        pass
