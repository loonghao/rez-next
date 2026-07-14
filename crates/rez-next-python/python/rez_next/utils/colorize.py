"""
Terminal output colorization for Rez-next.

Mirrors ``rez.utils.colorize`` public API:

- ``colorama_wrap(stream)`` / ``stream_is_tty(stream)``
- ``critical`` / ``error`` / ``warning`` / ``info`` / ``debug`` / ``heading`` /
  ``local`` / ``implicit`` / ``ephemeral`` / ``alias`` / ``inactive`` / ``notset``
- ``ColorizedStreamHandler``
- ``Printer``

Design decisions:
- No external Colorama dependency; ANSI SGR codes are emitted directly
  (Colorama's RESET_ALL-on-exit side effect caused problems in Rez).
- Config lookups use a lazy import to avoid circular imports with
  ``rez_next.config``.
"""
from __future__ import annotations

import logging
import os
import sys
from functools import lru_cache
from typing import IO


# ── ANSI SGR helpers ────────────────────────────────────────────────────────

def _sgr(params: str) -> str:
    """Return an ANSI SGR escape sequence."""
    return "\x1b[" + params + "m"


_RESET = _sgr("0")

# Named colour mapping (16-color palette)
_COLOR_CODES: dict[str, str | None] = {
    "black": "30",
    "red": "31",
    "green": "32",
    "yellow": "33",
    "blue": "34",
    "magenta": "35",
    "cyan": "36",
    "white": "37",
    "default": "39",
    # bright variants
    "bright_black": "90",
    "bright_red": "91",
    "bright_green": "92",
    "bright_yellow": "93",
    "bright_blue": "94",
    "bright_magenta": "95",
    "bright_cyan": "96",
    "bright_white": "97",
}

_STYLE_CODES: dict[str, str] = {
    "none": "0",
    "bold": "1",
    "dim": "2",
    "italic": "3",
    "underline": "4",
    "blink": "5",
    "reverse": "7",
    "hidden": "8",
    "strikethrough": "9",
    "bright": "1",  # bright = bold in most terminals
}


def _resolve_color(name: str | None) -> str | None:
    """Map a colour name to an SGR parameter, or return ``None``."""
    if name is None or name.lower() in ("none", "default", ""):
        return None
    return _COLOR_CODES.get(name.lower(), name)


def _resolve_style(name: str | None) -> str | None:
    """Map a style name to an SGR parameter, or return ``None``."""
    if name is None or name.lower() in ("none", ""):
        return None
    return _STYLE_CODES.get(name.lower(), name)


# ── Public helpers ──────────────────────────────────────────────────────────

def colorama_wrap(stream: IO[str]) -> IO[str]:
    """Wrap *stream* for cross-platform ANSI support (no-op on non-Windows).

    On Windows, translates ANSI escape sequences into Win32 API calls via
    Colorama.  Since we don't vendor Colorama, on Windows we simply return
    the stream unchanged — modern Windows 10+ terminals (ConPTY) handle
    ANSI natively.
    """
    return stream  # Modern terminals handle ANSI natively


def stream_is_tty(stream: IO[str]) -> bool:
    """Return ``True`` if *stream* is an interactive terminal."""
    try:
        return stream.isatty()
    except Exception:
        return False


# ── Config-driven style lookup ────────────────────────────────────────────

@lru_cache(maxsize=None)
def _color_enabled() -> bool | str:
    """Return the ``color_enabled`` setting (``True``, ``False`` or ``"force"``)."""
    from rez_next.config import get as _cfg_get
    return _cfg_get("color_enabled", True)


def _should_colorize(stream: IO[str]) -> bool:
    """Return ``True`` if colour output should be applied to *stream*."""
    enabled = _color_enabled()
    if isinstance(enabled, str) and enabled.lower() == "force":
        return True
    if not enabled:
        return False
    return stream_is_tty(stream)


def _get_style_from_config(key: str) -> tuple[str | None, str | None, list[str]]:
    """Look up ``*_fore``, ``*_back`` and ``*_styles`` from the rez config.

    Returns:
        ``(foreground, background, styles)`` where each style is an SGR
        parameter string (e.g. ``"31"``), or ``None`` if not configured.
    """
    from rez_next.config import get as _cfg_get
    fore = _cfg_get(f"{key}_fore", None)
    back = _cfg_get(f"{key}_back", None)
    raw_styles = _cfg_get(f"{key}_styles", None)
    if raw_styles is None:
        raw_styles = []
    styles = [_resolve_style(s) for s in raw_styles if _resolve_style(s)]
    return fore, back, styles


# ── Core colouring logic ──────────────────────────────────────────────────

def _color(
    str_: str,
    fore: str | None = None,
    back: str | None = None,
    styles: list[str] | None = None,
) -> str:
    """Wrap *str_* in ANSI SGR escape sequences.

    Args:
        str_: Text to colour.
        fore: Foreground colour name (e.g. ``"red"``, ``"green"``).
        back: Background colour name (e.g. ``"blue"``).
        styles: List of style names (e.g. ``["bold", "underline"]``).

    Returns:
        Coloured string if colours are resolved, otherwise *str_* unchanged.
    """
    fore_code = _resolve_color(fore)
    back_code = _resolve_color(back) if back else None
    style_codes = [_resolve_style(s) for s in (styles or []) if _resolve_style(s)]

    codes = list(style_codes)
    if fore_code:
        codes.append(fore_code)
    if back_code:
        codes.append(str(int(back_code) + 10))  # background = foreground + 10

    if not codes:
        return str_

    return _sgr(";".join(codes)) + str_ + _RESET


def _color_level(str_: str, level: str) -> str:
    """Apply the colour style configured for *level* to *str_*."""
    fore, back, styles = _get_style_from_config(level)
    return _color(str_, fore=fore, back=back, styles=styles)


# ── Public colour helper functions ────────────────────────────────────────

def critical(str_: str) -> str:
    """Return *str_* coloured for a critical message."""
    return _color_level(str_, "critical")


def error(str_: str) -> str:
    """Return *str_* coloured for an error message."""
    return _color_level(str_, "error")


def warning(str_: str) -> str:
    """Return *str_* coloured for a warning message."""
    return _color_level(str_, "warning")


def info(str_: str) -> str:
    """Return *str_* coloured for an info message."""
    return _color_level(str_, "info")


def debug(str_: str) -> str:
    """Return *str_* coloured for a debug message."""
    return _color_level(str_, "debug")


def heading(str_: str) -> str:
    """Return *str_* coloured for a heading."""
    return _color_level(str_, "heading")


def local(str_: str) -> str:
    """Return *str_* coloured for a local message."""
    return _color_level(str_, "local")


def implicit(str_: str) -> str:
    """Return *str_* coloured for an implicit message."""
    return _color_level(str_, "implicit")


def ephemeral(str_: str) -> str:
    """Return *str_* coloured for an ephemeral message."""
    return _color_level(str_, "ephemeral")


def alias(str_: str) -> str:
    """Return *str_* coloured for an alias."""
    return _color_level(str_, "alias")


def inactive(str_: str) -> str:
    """Return *str_* with a dim (inactive) style."""
    return _color(str_, styles=["dim"])


def notset(str_: str) -> str:
    """Return *str_* with no colouring applied."""
    return str_


# ── ColorizedStreamHandler ────────────────────────────────────────────────

_COLOR_MAP: dict[int, str] = {
    logging.CRITICAL: "critical",
    logging.ERROR: "error",
    logging.WARNING: "warning",
    logging.INFO: "info",
    logging.DEBUG: "debug",
    logging.NOTSET: "info",
}


class ColorizedStreamHandler(logging.StreamHandler):
    """A ``logging.StreamHandler`` that colourises output based on log level.

    Colouring is controlled by the ``color_enabled`` rez config setting
    and per-level ``*_fore`` / ``*_back`` / ``*_styles`` settings.
    """

    def __init__(
        self,
        stream: IO[str] | None = None,
    ) -> None:
        super().__init__(stream)
        self._colorize_check = True

    @property
    def is_colorized(self) -> bool:
        """Return ``True`` if this handler will colourise output."""
        return _should_colorize(self.stream)  # type: ignore[arg-type]

    def emit(self, record: logging.LogRecord) -> None:
        """Emit a log record with optional colourisation."""
        try:
            msg = self.format(record)
            level_name = _COLOR_MAP.get(record.levelno, "info")
            if self.is_colorized:
                msg = _color_level(msg, level_name)
            stream = self.stream
            if stream is None:
                stream = sys.stderr
            stream.write(msg + "\n")
            self.flush()
        except (KeyboardInterrupt, SystemExit):
            raise
        except Exception:
            self.handleError(record)


# ── Printer ────────────────────────────────────────────────────────────────

class Printer:
    """General-purpose output printer with optional colourisation.

    Args:
        stream: Output stream (default: ``sys.stdout``).
        colorize: Whether to colourise output.  If ``None`` (default), uses
            the ``color_enabled`` config setting and TTY detection.
    """

    def __init__(
        self,
        stream: IO[str] | None = None,
        colorize: bool | None = None,
    ) -> None:
        self.stream = stream or sys.stdout
        if colorize is None:
            self._colorize = _should_colorize(self.stream)
        else:
            self._colorize = colorize

    def print(self, msg: str = "", style: str | None = None, **kwargs) -> None:
        """Print *msg* to the configured stream.

        Args:
            msg: Message to print.
            style: Optional style name (``"critical"``, ``"error"``, ...).
                If ``None``, the message is printed as-is.
            **kwargs: Forwarded to the underlying ``stream.write()``.
        """
        formatted = str(msg)
        if self._colorize and style:
            formatted = _color_level(formatted, style)
        self.stream.write(formatted + "\n")
        self.stream.flush()
