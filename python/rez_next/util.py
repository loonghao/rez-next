"""Bridge to rez_next util module (native utility functions).

Aligns with rez.util API:
- ``which()`` — find an executable in PATH
- ``which_all()`` — find all matching executables
- ``get_hostname()`` — get system hostname
- ``get_username()`` — get current username
- ``copy_file()`` — copy a file
- ``safe_remove()`` — safely remove a file
- ``ensure_dir_exists()`` — create directory if missing
- ``ensure_parent_dir_exists()`` — create parent directory
- ``expand_user_path()`` — expand ~ in paths
- ``is_writable()`` — check if directory/file is writable
- ``truncate_string()`` — truncate string to max length
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "util.py"
)
globals().update(runpy.run_path(str(_IMPL)))
