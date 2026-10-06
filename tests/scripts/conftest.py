"""Make the vx extension entrypoint importable as `release`.

These tests exercise the script's own logic (argument parsing, source-directory
resolution, exit-code mapping). They deliberately do not import `rez_next`, so
they run in CI without a built wheel.
"""

import sys
from pathlib import Path

SCRIPTS_DIR = str(Path(__file__).resolve().parents[2] / "scripts")

if SCRIPTS_DIR not in sys.path:
    sys.path.insert(0, SCRIPTS_DIR)
