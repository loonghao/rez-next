"""Deprecation utilities for rez_next.

This module provides compatibility with rez.deprecations.
"""

from __future__ import annotations

import warnings
import os


class RezDeprecationWarning(DeprecationWarning):
    """Custom deprecation warning for Rez."""
    pass


def warn(
    message: str,
    category: type = RezDeprecationWarning,
    pre_formatted: bool = False,
    stacklevel: int = 1,
    filename: str | None = None,
    **kwargs,
) -> None:
    """
    Wrapper around warnings.warn that allows passing a pre-formatted warning message.

    This allows warning about things that aren't coming from Python files,
    like environment variables, etc.
    """
    if not pre_formatted:
        warnings.warn(
            message,
            category=category,
            stacklevel=stacklevel + 1,
            **kwargs,
        )
        return

    original_formatwarning = warnings.formatwarning

    def formatwarning(  # type: ignore
        msg: str,
        category: type,
        filename: str,
        lineno: int,
        line: str | None = None,
    ) -> str:
        return "{0}{1}: {2}\n".format(
            "{0}: ".format(filename) if filename else "",
            category.__name__,
            msg,
        )

    warnings.formatwarning = formatwarning

    warnings.warn(message, category=category, stacklevel=stacklevel + 1, **kwargs)
    warnings.formatwarning = original_formatwarning


# Emulate rez.deprecations behavior
# (module-level imports for compatibility)
warnings  # make sure it's accessible as rez.deprecations.warnings
