"""
Bridge to the ``platform_`` implementation — rez-compatible platform detection.

Usage::

    from rez.utils.platform_ import platform_

    print(platform_.os)      # "Ubuntu-20.04" | "windows-10.0.19045" | ...
    print(platform_.arch)    # "x86_64"
    print(platform_.physical_cores)
"""

from __future__ import annotations

from pathlib import Path

_IMPL = (
    Path(__file__).resolve().parents[3]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "utils"
    / "platform_.py"
)
globals().update(__import__("runpy").run_path(str(_IMPL)))
