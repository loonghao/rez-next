# SPDX-License-Identifier: Apache-2.0
# Copyright Contributors to the Rez Project


"""Logging utilities for rez_next.

This module provides print_debug/info/warning/error/critical functions
that wrap the standard logging module, matching the API of rez.utils.logging_.
"""

from contextlib import contextmanager
import logging
import time

logger = logging.getLogger(__name__)


def print_debug(msg, *nargs):
    """Print a debug message.

    Args:
        msg (str): Message to print (may contain % formatting).
        *nargs: Arguments for % formatting.
    """
    logger.debug(msg, *nargs)


def print_info(msg, *nargs):
    """Print an info message.

    Args:
        msg (str): Message to print (may contain % formatting).
        *nargs: Arguments for % formatting.
    """
    logger.info(msg, *nargs)


def print_warning(msg, *nargs):
    """Print a warning message.

    Args:
        msg (str): Message to print (may contain % formatting).
        *nargs: Arguments for % formatting.
    """
    logger.warning(msg, *nargs)


def print_error(msg, *nargs):
    """Print an error message.

    Args:
        msg (str): Message to print (may contain % formatting).
        *nargs: Arguments for % formatting.
    """
    logger.error(msg, *nargs)


def print_critical(msg, *nargs):
    """Print a critical message.

    Args:
        msg (str): Message to print (may contain % formatting).
        *nargs: Arguments for % formatting.
    """
    logger.critical(msg, *nargs)


def get_debug_printer(enabled=True):
    """Get a debug printer callable.

    Args:
        enabled (bool): If False, the printer is a no-op.

    Returns:
        _Printer: A callable that prints debug messages.
    """
    return _Printer(enabled, logger.debug)


def get_info_printer(enabled=True):
    """Get an info printer callable.

    Args:
        enabled (bool): If False, the printer is a no-op.

    Returns:
        _Printer: A callable that prints info messages.
    """
    return _Printer(enabled, logger.info)


def get_warning_printer(enabled=True):
    """Get a warning printer callable.

    Args:
        enabled (bool): If False, the printer is a no-op.

    Returns:
        _Printer: A callable that prints warning messages.
    """
    return _Printer(enabled, logger.warning)


def get_error_printer(enabled=True):
    """Get an error printer callable.

    Args:
        enabled (bool): If False, the printer is a no-op.

    Returns:
        _Printer: A callable that prints error messages.
    """
    return _Printer(enabled, logger.error)


def get_critical_printer(enabled=True):
    """Get a critical printer callable.

    Args:
        enabled (bool): If False, the printer is a no-op.

    Returns:
        _Printer: A callable that prints critical messages.
    """
    return _Printer(enabled, logger.critical)


class _Printer:
    """Callable printer that wraps a logging function.

    When disabled, the printer is a no-op.
    """

    def __init__(self, enabled=True, printer_function=None):
        """Create a _Printer.

        Args:
            enabled (bool): If False, printing is disabled.
            printer_function (callable): Logging function to call.
        """
        self.printer_function = printer_function if enabled else None

    def __call__(self, msg, *nargs):
        """Print a message.

        Args:
            msg (str): Message to print.
            *nargs: Arguments for % formatting.
        """
        if self.printer_function:
            if nargs:
                msg = msg % nargs
            self.printer_function(msg)

    def __bool__(self):
        """Check if the printer is enabled."""
        return bool(self.printer_function)


@contextmanager
def log_duration(printer, msg):
    """Context manager to log the duration of an operation.

    Args:
        printer (callable): Printer function to use.
        msg (str): Message to print with the duration.

    Yields:
        None
    """
    t1 = time.time()
    yield None

    t2 = time.time()
    secs = t2 - t1
    printer(msg, str(secs))
