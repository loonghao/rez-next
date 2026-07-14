"""
Rez-compatible lint helper module.

Aligns with ``rez.utils.lint_helper`` API.

This module lets you import anything from it, and the result is a
variable set to ``None``. It is only here to keep linters such as
PyFlakes happy. It is used in cases where code looks like it references
an uninitialised variable, but does not.

Rez API: ``rez.utils.lint_helper``
"""

from __future__ import annotations

from types import ModuleType
import sys


class NoneModule(ModuleType):
    """Module subclass that returns ``None`` for any attribute access.

    This allows ``from rez_next.utils.lint_helper import some_variable``
    to succeed and return ``None``, satisfying linters that would otherwise
    flag the variable as undefined.
    """

    def __getattr__(self, name):
        return None

    def used(self, object_) -> None:
        """Use this to stop 'variable/module not used' linting errors.

        Args:
            object_: The object to mark as used.
        """
        pass


noner = NoneModule(__name__)

sys.modules[__name__] = noner
