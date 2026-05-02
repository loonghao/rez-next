import rez_next._native  # ensure extension module is initialized  # noqa: F401
from rez_next._native.complete import *  # noqa: F401,F403

# Alias for API compatibility with original rez
get_completion_script = get_completion_script_py


def print_completion_script(shell_type: str) -> None:
    """Print completion script for the given shell type."""
    script = get_completion_script(shell_type)
    print(script)
