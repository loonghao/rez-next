"""Rez-compatible shell module.

Wraps native shell functions under Rez-expected names.
"""
import rez_next._native as _native  # noqa: F811
from rez_next._native.shell import (  # noqa: F401,F403
    Shell,
    create_shell_script,
    get_available_shells,
    get_current_shell,
)


# ── Rez-API-compatible aliases ──────────────────────────────────────


def create_shell(name: str, **kwargs) -> Shell:
    """Create a shell instance.

    Args:
        name: Shell type name (e.g. 'bash', 'tcsh', 'sh').
        **kwargs: Passed to Shell constructor.

    Returns:
        A Shell instance.

    Rez API: ``rez.shell.create_shell()``
    """
    return Shell(name, **kwargs)


def get_shell_types() -> list[str]:
    """Return the list of available shell types.

    Rez API: ``rez.shells.get_shell_types()``
    """
    return get_available_shells()


def get_shell_class(name: str):
    """Return the shell class for the given shell type name.

    Rez API: ``rez.shells.get_shell_class()``
    """
    types = get_available_shells()
    if name not in types:
        msg = "unknown shell type %r; available: %s"
        raise ValueError(msg % (name, ", ".join(types)))
    return Shell
