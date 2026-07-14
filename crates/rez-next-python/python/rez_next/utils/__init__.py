"""Rez-compatible utils sub-package.

Aligns with ``rez.utils`` API by providing utility context managers
and exception re-raising helpers.

Functions:
    with_noop: No-op context manager (placeholder).
    reraise: Re-raise an exception with a different type but preserving
        the original traceback.
"""
from __future__ import annotations

import sys
from contextlib import contextmanager
from typing import NoReturn


@contextmanager
def with_noop():
    """No-op context manager.

    Usage::

        with with_noop():
            pass  # nothing happens

    Rez API: ``rez.utils.with_noop()``
    """
    yield


def reraise(exc: BaseException, new_exc_cls: type[BaseException]) -> NoReturn:
    """Re-raise an exception with a different type.

    Preserves the original traceback while changing the exception class.

    Args:
        exc: Original exception instance.
        new_exc_cls: Target exception class (e.g., ValueError).

    Raises:
        NoReturn: Always raises an exception of type ``new_exc_cls``.

    Rez API: ``rez.utils.reraise()``
    """
    def _reraise(exc, new_exc_cls):
        raise new_exc_cls(exc).with_traceback(sys.exc_info()[2]) from None

    _reraise(exc, new_exc_cls)
