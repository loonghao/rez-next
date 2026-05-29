"""
Platform-mapped function decorator for Rez-next.

Mirrors ``rez.utils.platform_mapped`` — provides a decorator that maps
function return values through ``config.platform_map`` using regex
substitution.
"""
from __future__ import annotations

import re


def platform_mapped(func):
    r"""Decorates functions for lookups within a ``config.platform_map`` dict.

    The first level key is mapped to the ``func.__name__``.  Regular expressions
    are used on the second level key values.  Only the first matching regex
    substitution is applied.

    Example config::

        config.platform_map = {
            "os": {
                r"Scientific Linux-(.*)": r"Scientific-\1",
                r"Ubuntu-14.\d": r"Ubuntu-14",
            },
            "arch": {
                "x86_64": "64bit",
                "amd64": "64bit",
            },
        }

    Args:
        func: The function whose return value will be mapped.

    Returns:
        Wrapped function.
    """
    def inner(*args, **kwargs):
        # Lazy import config to avoid circular dependency
        from rez_next.config import config

        result = func(*args, **kwargs)
        entry = config.platform_map.get(func.__name__)
        if entry:
            for key, value in entry.items():
                result, changes = re.subn(key, value, result)
                if changes > 0:
                    break
        return result
    return inner
