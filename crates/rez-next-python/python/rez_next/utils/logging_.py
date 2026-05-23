"""
Conditional logging utilities for Rez-next.

Mirrors `rez.utils.logging_` API:
- print_debug, print_info, print_warning, print_error, print_critical
- get_debug_printer, get_info_printer, ... (enable/disable-able printers)
- log_duration context manager
- _Printer class

Design principle (single-responsibility):
  We own a per-module logger; the Printer wrapper adds on/off semantics
  without polluting the rest of the codebase with "if lg:" guards.
"""
from __future__ import annotations

import logging
import sys
import time
from contextlib import contextmanager
from typing import Callable

# Module-level logger — matches rez's `logger = logging.getLogger(__name__)`
logger = logging.getLogger(__name__)


# ── Convenience functions (wrap stdlib logger) ──────────────────────────────

def print_debug(msg: str, *nargs: object) -> None:
    """Log a DEBUG-level message."""
    logger.debug(msg, *nargs)


def print_info(msg: str, *nargs: object) -> None:
    """Log an INFO-level message."""
    logger.info(msg, *nargs)


def print_warning(msg: str, *nargs: object) -> None:
    """Log a WARNING-level message."""
    logger.warning(msg, *nargs)


def print_error(msg: str, *nargs: object) -> None:
    """Log an ERROR-level message."""
    logger.error(msg, *nargs)


def print_critical(msg: str, *nargs: object) -> None:
    """Log a CRITICAL-level message."""
    logger.critical(msg, *nargs)


# ── Printer (conditional logging wrapper) ──────────────────────────────────

class _Printer:
    """A conditional printer that can be enabled/disabled.

    Acts like a function: ``printer(msg, arg)`` → ``logger.info(msg % arg)``.
    When disabled, all calls are no-ops.

    Args:
        enabled: Whether this printer produces output.
        printer_function: The underlying log method (e.g. ``logger.info``).
    """

    def __init__(self, enabled: bool = True,
                 printer_function: Callable[..., None] | None = None) -> None:
        self.printer_function = printer_function if enabled else None

    def __call__(self, msg: str, *nargs: object) -> None:
        if self.printer_function is not None:
            if nargs:
                msg = msg % nargs
            self.printer_function(msg)

    def __bool__(self) -> bool:
        return self.printer_function is not None


def get_debug_printer(enabled: bool = True) -> _Printer:
    """Return a debug-level Printer."""
    return _Printer(enabled, logger.debug)


def get_info_printer(enabled: bool = True) -> _Printer:
    """Return an info-level Printer."""
    return _Printer(enabled, logger.info)


def get_warning_printer(enabled: bool = True) -> _Printer:
    """Return a warning-level Printer."""
    return _Printer(enabled, logger.warning)


def get_error_printer(enabled: bool = True) -> _Printer:
    """Return an error-level Printer."""
    return _Printer(enabled, logger.error)


def get_critical_printer(enabled: bool = True) -> _Printer:
    """Return a critical-level Printer."""
    return _Printer(enabled, logger.critical)


# ── Duration measurement ──────────────────────────────────────────────────

@contextmanager
def log_duration(
    printer: _Printer,
    msg: str = "Operation took %s seconds",
) -> None:
    """Context manager that logs elapsed time.

    Args:
        printer: A Printer to output the duration.
        msg: Template string; ``%s`` is replaced with the elapsed seconds.

    Usage::

        with log_duration(info_printer, "Fetch took %s s"):
            do_something()
    """
    t1 = time.time()
    yield
    t2 = time.time()
    _ = printer(msg, str(t2 - t1))


# ── Legacy rez compatibility alias ──────────────────────────────────────────

# rez code uses `from rez.utils.logging_ import get_warning_printer`
# Our ``logging_`` module makes this exact import path work.
