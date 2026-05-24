"""Bridge to rez_next cli module (command-line interface).

Aligns with rez.cli API:
- ``main()`` — CLI entry point
- ``setup_parser()`` — argument parser setup
- Subcommand modules for build, env, search, etc.
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "cli.py"
)
globals().update(runpy.run_path(str(_IMPL)))
