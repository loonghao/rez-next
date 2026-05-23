"""
String formatting utilities for Rez-next.

Mirrors ``rez.utils.formatting`` — provides columnar output, terminal-aware
truncation, and tabular formatting helpers used by ``rez-depends --tree``,
``rez-search``, ``rez-config``, and similar CLI commands.
"""
from __future__ import annotations

import shutil
from typing import Iterable, Sequence


def get_terminal_width() -> int:
    """Return the current terminal width in columns (default 80)."""
    size = shutil.get_terminal_size(fallback=(80, 24))
    return size.columns


def columnise(
    items: Iterable[str],
    width: int | None = None,
    padding: int = 2,
) -> str:
    """Format an iterable of strings into columnar output.

    Items are laid out left-to-right, top-to-bottom, similar to ``ls`` output.

    Args:
        items: Strings to format.
        width: Total available width in columns.  ``None`` → auto-detect.
        padding: Minimum spaces between columns.

    Returns:
        Column-formatted string.
    """
    items_list = list(items)
    if not items_list:
        return ""

    w = width if width is not None else get_terminal_width()
    max_item = max(len(s) for s in items_list) + padding
    cols = max(1, w // max_item)

    # Build rows
    rows: list[list[str]] = []
    for i in range(0, len(items_list), cols):
        rows.append(items_list[i : i + cols])

    lines: list[str] = []
    for row in rows:
        padded = [s.ljust(max_item) for s in row]
        lines.append("".join(padded).rstrip())
    return "\n".join(lines)


def header_line(
    label: str,
    char: str = "-",
    width: int | None = None,
) -> str:
    """Return a centred header line::

        --- label ---

    Args:
        label: Text to centre inside the line.
        char: Repeat character for the line.
        width: Total width.  ``None`` → auto-detect.
    """
    w = width if width is not None else get_terminal_width()
    inner = f" {label} " if label else ""
    needed = max(0, w - len(inner))
    left = needed // 2
    right = needed - left
    return f"{char * left}{inner}{char * right}"


def truncate(
    text: str,
    max_len: int,
    suffix: str = "...",
) -> str:
    """Truncate *text* to *max_len* characters, appending *suffix* if needed.

    This is a pure-Python companion to the native ``rez_next.util.truncate_string``.
    """
    if len(text) <= max_len:
        return text
    return text[: max(0, max_len - len(suffix))] + suffix


def format_table(
    rows: Sequence[Sequence[str]],
    headers: Sequence[str] | None = None,
    col_sep: str = "  ",
) -> str:
    """Format tabular data as an aligned text table.

    Args:
        rows: Data rows (each row is a sequence of strings).
        headers: Optional column headers (same length as rows).
        col_sep: Column separator.

    Returns:
        Formatted table string.
    """
    if not rows and not headers:
        return ""

    all_rows: list[Sequence[str]] = list(rows)
    if headers:
        all_rows.insert(0, headers)

    if not all_rows:
        return ""

    # Compute column widths
    num_cols = max(len(r) for r in all_rows)
    widths = [0] * num_cols
    for row in all_rows:
        for i in range(len(row)):
            widths[i] = max(widths[i], len(row[i]))

    lines: list[str] = []
    for idx, row in enumerate(all_rows):
        padded = [row[i].ljust(widths[i]) for i in range(len(row))]
        lines.append(col_sep.join(padded))

        if headers and idx == 0:
            # Separator under header
            sep = col_sep.join("-" * w for w in widths)
            lines.append(sep)

    return "\n".join(lines)
