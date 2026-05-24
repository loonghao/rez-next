"""
Filesystem utilities for Rez-next.

Wraps and extends the native filesystem functions (which, copy_file,
safe_remove, ensure_dir_exists, etc.) exposed by ``rez_next.util``.

Mirrors the common subset of ``rez.utils.filesystem``.
"""
from __future__ import annotations

import os
import shutil
import tempfile
from contextlib import contextmanager
from typing import Iterator

from rez_next.util import (
    copy_file,
    ensure_dir_exists,
    ensure_parent_dir_exists,
    expand_user_path,
    is_writable,
    safe_remove,
    which,
    which_all,
)

# Re-export native functions for convenience
__all__ = [
    "copy_file",
    "ensure_dir_exists",
    "ensure_parent_dir_exists",
    "expand_user_path",
    "is_writable",
    "safe_remove",
    "which",
    "which_all",
    "find_matching_symlink",
    "hardlink_or_copy",
    "temp_dir",
    "atomic_write",
    "make_relative_symlink",
]


def find_matching_symlink(directory: str, target: str) -> str | None:
    """Find an existing symlink in *directory* that points to *target*.

    Args:
        directory: Directory to scan.
        target: The symlink's target path.

    Returns:
        Symlink basename if found, else ``None``.
    """
    if not os.path.isdir(directory):
        return None

    norm_target = os.path.normpath(os.path.abspath(target))
    for entry in os.listdir(directory):
        link_path = os.path.join(directory, entry)
        if os.path.islink(link_path):
            link_target = os.path.normpath(os.path.abspath(os.readlink(link_path)))
            if link_target == norm_target:
                return entry
    return None


def hardlink_or_copy(src: str, dst: str) -> int:
    """Hardlink *src* to *dst*; fall back to copy on cross-device / permission errors.

    Returns:
        Number of bytes copied (0 if hardlinked).
    """
    try:
        os.link(src, dst)
        return 0
    except (OSError, NotImplementedError):
        return copy_file(src, dst)


@contextmanager
def temp_dir(prefix: str = "rez_") -> Iterator[str]:
    """Context manager that yields a temporary directory and cleans up on exit.

    Args:
        prefix: Directory name prefix.

    Yields:
        Path to a temporary directory.
    """
    path = tempfile.mkdtemp(prefix=prefix)
    try:
        yield path
    finally:
        shutil.rmtree(path, ignore_errors=True)


@contextmanager
def atomic_write(filepath: str, mode: str = "w", encoding: str = "utf-8") -> Iterator:
    """Atomic file write: write to a temp file then rename.

    Prevents partial writes from being visible to other processes.

    Args:
        filepath: Destination file path.
        mode: File open mode (default "w").
        encoding: Text encoding (default "utf-8").
    """
    ensure_parent_dir_exists(filepath)
    fd, tmp = tempfile.mkstemp(
        dir=os.path.dirname(filepath) or ".",
        prefix=".tmp_",
    )
    os.close(fd)

    try:
        with open(tmp, mode, encoding=encoding) as f:
            yield f
        os.replace(tmp, filepath)
    except BaseException:
        safe_remove(tmp)
        raise


def make_relative_symlink(src: str, dst: str) -> str:
    """Create a symlink from *dst* → *src* using a relative path.

    On Windows this requires developer mode or admin privileges.

    Args:
        src: Target of the symlink (absolute or relative path).
        dst: Symlink path.

    Returns:
        The *dst* path.
    """
    src_abs = os.path.abspath(src)
    dst_dir = os.path.dirname(os.path.abspath(dst))
    rel = os.path.relpath(src_abs, dst_dir)
    os.symlink(rel, dst)
    return dst
