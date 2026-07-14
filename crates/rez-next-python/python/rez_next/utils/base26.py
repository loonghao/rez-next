"""
Rez-compatible base26 encoding utilities.

Aligns with ``rez.utils.base26`` API.

Provides Base26 (bijective base-26) encoding utilities used for generating
short unique identifiers (e.g., variant indices, symlink names).

Rez API: ``rez.utils.base26.get_next_base26()``,
``rez.utils.base26.create_unique_base26_symlink()``
"""

from __future__ import annotations

from rez_next.util import get_next_base26, create_unique_base26_symlink

__all__ = [
    "get_next_base26",
    "create_unique_base26_symlink",
]
