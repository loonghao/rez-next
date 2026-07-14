"""
Rez-compatible serialise module.

Aligns with ``rez.serialise`` API:
- ``FileFormat`` — supported file formats
- ``load_from_file()`` — load data from a file
- ``open_file_for_write()`` — write data to a file with NFS-safe local cache
- ``load_py()`` / ``load_yaml()`` / ``load_txt()`` — format-specific loaders
- ``EarlyThis`` — helper class for ``@early`` decorated functions
- ``process_python_objects()`` — post-processing for Python package definitions

See https://github.com/AcademySoftwareFoundation/rez/blob/main/src/rez/serialise.py
"""

from __future__ import annotations

import io
import os
import tempfile
from contextlib import contextmanager
from enum import Enum
from typing import Any, Callable, Iterator, TextIO, Optional

# ── FileFormat (matches rez.serialise.FileFormat) ─────────────────────────────


class FileFormat(Enum):
    """Supported file formats for serialisation."""

    py = (".py",)
    yaml = (".yaml",)
    txt = (".txt",)

    def __init__(self, extension: str) -> None:
        self.extension = extension


# ── NFS-safe write cache ─────────────────────────────────────────────────────

_file_cache: dict[str, str] = {}


@contextmanager
def open_file_for_write(filepath: str, mode: Optional[int] = None) -> Iterator[TextIO]:
    """Open a file for writing, creating a local temp cache for NFS safety.

    Args:
        filepath: Path to the target file.
        mode: Optional file permissions (e.g. ``0o644``).

    Yields:
        A text IO stream that buffers writes.
    """
    buf = io.StringIO()
    yield buf
    content = buf.getvalue()

    # Write to target file
    with open(filepath, "w", encoding="utf-8") as f:
        f.write(content)

    if mode is not None:
        os.chmod(filepath, mode)

    # Write local cache copy for NFS read-after-write safety
    tmpdir = tempfile.gettempdir()
    cache_path = os.path.join(tmpdir, os.path.basename(filepath) + ".cache")
    with open(cache_path, "w", encoding="utf-8") as f:
        f.write(content)
    _file_cache[os.path.abspath(filepath)] = cache_path


# ── Main loader ──────────────────────────────────────────────────────────────


def load_from_file(
    filepath: str,
    format_: FileFormat = FileFormat.py,
    update_data_callback: Optional[Callable] = None,
    disable_memcache: bool = False,
) -> Any:
    """Load data from a file.

    Args:
        filepath: Path to the file.
        format_: File format (default: ``FileFormat.py``).
        update_data_callback: Optional callback ``(format_, data) -> data``.
        disable_memcache: If True, skip memcache (no-op in this impl).

    Returns:
        Parsed data: ``dict`` for ``.py`` and ``str`` for ``.txt``.
    """
    abs_path = os.path.abspath(filepath)

    # Check local NFS cache first
    if abs_path in _file_cache:
        cache_path = _file_cache[abs_path]
        if os.path.exists(cache_path):
            with open(cache_path, "r", encoding="utf-8") as f:
                content = f.read()
        else:
            with open(abs_path, "r", encoding="utf-8") as f:
                content = f.read()
    else:
        with open(abs_path, "r", encoding="utf-8") as f:
            content = f.read()

    if format_ == FileFormat.py:
        data = load_py(content, filepath)
    elif format_ == FileFormat.yaml:
        data = load_yaml(content, filepath)
    elif format_ == FileFormat.txt:
        data = content
    else:
        raise ValueError(f"Unsupported format: {format_}")

    if update_data_callback is not None:
        data = update_data_callback(format_, data)

    return data


# ── Format-specific loaders ──────────────────────────────────────────────────


def load_py(stream: str, filepath: Optional[str] = None) -> dict[str, Any]:
    """Load Python-format data from a string.

    This is a simplified placeholder that evaluates the stream safely.
    Full Rez compatibility would require proper importlib-based loading.

    Args:
        stream: Python source code as a string.
        filepath: Optional source file path (for error reporting).

    Returns:
        Parsed data dictionary.
    """
    # Simplified: wrap in a namespace dict and exec
    namespace: dict[str, Any] = {}
    try:
        exec(stream, namespace)
    except Exception as e:
        raise RuntimeError(
            f"Failed to load Python package definition"
            + (f" from {filepath}" if filepath else "")
            + f": {e}"
        )

    # Filter out builtins, modules, and private names
    data = {}
    for key, value in namespace.items():
        if key.startswith("__"):
            continue
        if isinstance(value, type(os)):  # skip modules
            continue
        if callable(value) and not key.startswith("preprocess"):
            continue  # skip non-preprocess functions
        data[key] = value

    return data


def load_yaml(stream: str, filepath: Optional[str] = None) -> dict[str, Any]:
    """Reject the obsolete YAML package-definition format."""

    del stream
    location = f" ({filepath})" if filepath else ""
    raise ValueError(
        f"YAML package definitions are not supported{location}; use package.py"
    )


def load_txt(stream: str, filepath: Optional[str] = None) -> str:
    """Load text-format data from a string.

    Args:
        stream: Text content as a string.
        filepath: Optional source file path (unused, for API compat).

    Returns:
        The text content as-is.
    """
    return stream


# ── Early binding helpers ────────────────────────────────────────────────────


class EarlyThis:
    """Helper class for ``@early`` decorated functions, providing ``this`` access."""

    def __init__(self, data: dict[str, Any]) -> None:
        self._data = data

    def __getattr__(self, attr: str) -> Any:
        if attr in self._data:
            value = self._data[attr]
            if callable(value) and hasattr(value, "_early"):
                raise ValueError(f"Cannot access early-bound function '{attr}' from another early function")
            return value
        raise AttributeError(f"'this' object has no attribute '{attr}'")


_thread_local_vars: dict[str, Any] = {}


@contextmanager
def set_objects(objects: dict[str, Any]) -> Iterator[None]:
    """Set objects available inside ``@early`` functions.

    Args:
        objects: Dict of variable name -> value.
    """
    global _thread_local_vars
    old = _thread_local_vars.copy()
    _thread_local_vars.update(objects)
    try:
        yield
    finally:
        _thread_local_vars = old


def get_objects() -> dict[str, Any]:
    """Get currently set early-binding objects."""
    return _thread_local_vars


# ── Post-processing ──────────────────────────────────────────────────────────


def process_python_objects(data: dict[str, Any], filepath: Optional[str] = None) -> dict[str, Any]:
    """Post-process Python-loaded package data.

    Processes ``@early``/``@late`` decorated functions and rex functions.

    Args:
        data: Raw data dictionary from ``load_py``.
        filepath: Optional source file path.

    Returns:
        Processed data dictionary.
    """
    result = {}
    for key, value in data.items():
        if key.startswith("__"):
            continue
        if isinstance(value, type(os)):
            continue
        if callable(value):
            if hasattr(value, "_early"):
                try:
                    result[key] = value()
                except Exception:
                    result[key] = value
            elif hasattr(value, "_late"):
                result[key] = value
            elif key == "preprocess":
                result[key] = value
            else:
                continue  # skip plain functions
        else:
            result[key] = value
    return result


def clear_file_caches() -> None:
    """Clear all file caches (memcache + local NFS cache)."""
    _file_cache.clear()


# ── Public API ───────────────────────────────────────────────────────────────

__all__ = [
    "FileFormat",
    "open_file_for_write",
    "load_from_file",
    "load_py",
    "load_yaml",
    "load_txt",
    "EarlyThis",
    "set_objects",
    "get_objects",
    "process_python_objects",
    "clear_file_caches",
]
