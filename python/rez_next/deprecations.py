"""
Deprecation utilities for Rez compatibility.

This module provides the same deprecation warning utilities as `rez.deprecations`,
with the exact same interface so that code can use the same deprecation warnings.
"""

from __future__ import annotations
import warnings
from typing import Any, TYPE_CHECKING

if TYPE_CHECKING:
    from typing import Type


class RezDeprecationWarning(DeprecationWarning):
    """Rez-specific deprecation warning class."""
    pass


def warn(
    message: str,
    category: Type[Warning] = RezDeprecationWarning,
    pre_formatted: bool = False,
    stacklevel: int = 1,
    filename: str = None,
    **kwargs: Any
) -> None:
    """
    Issue a deprecation warning.

    This is a wrapper around `warnings.warn()` that supports non-Python source
    (e.g., environment variables, config items, command-line arguments).

    Args:
        message: Warning message.
        category: Warning category (default: RezDeprecationWarning).
        pre_formatted: If True, message is pre-formatted and custom formatting is used.
        stacklevel: Stack level for warning (incremented by 1 to point to caller).
        filename: Optional filename for pre-formatted warnings.
        **kwargs: Additional arguments passed to `warnings.warn()`.
    """
    if pre_formatted:
        # Save original formatwarning
        original_formatwarning = warnings.formatwarning

        # Define custom formatwarning
        def custom_formatwarning(
            message: Warning,
            category: Type[Warning],
            filename_arg: str,
            lineno: int,
            line: str = None
        ) -> str:
            # Use provided filename or default
            use_filename = filename if filename else filename_arg
            return f"{use_filename}: {category.__name__}: {message}\n"

        # Set custom formatwarning
        warnings.formatwarning = custom_formatwarning

        # Issue warning with incremented stacklevel
        warnings.warn(message, category, stacklevel + 1, **kwargs)

        # Restore original formatwarning
        warnings.formatwarning = original_formatwarning
    else:
        # Issue standard warning
        warnings.warn(message, category, stacklevel + 1, **kwargs)
