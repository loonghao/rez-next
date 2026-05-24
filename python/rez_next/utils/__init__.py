"""
Rez-next utility subpackage.

Mirrors `rez.utils` public API:
- logging_ (conditional Printer, log_duration)
- yaml (dump/load with custom Rez type support)
- colorize (terminal color helpers)
- formatting (string formatting utilities)

Base functions (get_hostname, get_username, which, etc.)
are exposed via the native _native.util module.
"""
from __future__ import annotations

from rez_next.util import *  # noqa: F401, F403 — re-export native util functions

# Submodules
from . import logging_ as logging  # noqa: F401 — rez.utils.logging_ alias
from . import formatting  # noqa: F401 — string formatting utilities
from . import filesystem  # noqa: F401 — filesystem utilities
from . import yaml  # noqa: F401 — YAML serialization
from . import platform_  # noqa: F401 — platform abstraction
