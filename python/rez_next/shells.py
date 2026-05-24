"""Shells module — aligns with rez.shells API.

Rez exposes shell plugins via ``rez.shells`` (plural). This module
provides the same interface by importing from the native ``shell``
submodule, ensuring ``from rez_next.shells import create_shell``
works as expected.
"""
from rez_next.shell import (  # noqa: F401
    Shell,
    create_shell,
    get_shell_types,
    get_shell_class,
)
