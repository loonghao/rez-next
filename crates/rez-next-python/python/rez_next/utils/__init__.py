"""
Rez-next utility subpackage.

Mirrors `rez.utils` public API:
- logging_ (conditional Printer, log_duration)
- yaml (dump/load with custom Rez type support)
- colorize (terminal color helpers)
- formatting (string formatting utilities)
- data_utils (dict merge, diff, caching descriptors, etc.)
- filesystem (file/symlink utilities)

Base functions (get_hostname, get_username, which, etc.)
are exposed via the native _native.util module.
"""
from __future__ import annotations

from rez_next.util import *  # noqa: F401, F403 — re-export native util functions

# Submodules
from . import logging_ as logging  # noqa: F401 — rez.utils.logging_ alias
from . import colorize  # noqa: F401 — terminal color helpers
from . import data_utils  # noqa: F401 — dict/data manipulation utilities
from . import formatting  # noqa: F401 — string formatting utilities
from . import filesystem  # noqa: F401 — filesystem utilities
from . import yaml  # noqa: F401 — YAML serialization
