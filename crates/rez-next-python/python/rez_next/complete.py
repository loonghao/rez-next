import rez_next._native  # ensure extension module is initialized  # noqa: F401
from rez_next._native.complete import (
    get_completion_install_path,
    get_completion_script_py,
    supported_completion_shells,
)

# Alias for API compatibility with original rez
get_completion_script = get_completion_script_py

__all__ = [
    "get_completion_install_path",
    "get_completion_script",
    "get_completion_script_py",
    "print_completion_script",
    "supported_completion_shells",
]


def print_completion_script(shell_type: str) -> None:
    """Print completion script for the given shell type."""
    script = get_completion_script(shell_type)
    print(script)
