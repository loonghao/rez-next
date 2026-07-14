"""
command — aligns with rez.command.

Provides the ``Command`` base class for registering custom Rez subcommands
via the plugin system. Each subcommand plugin is a module under the
``command`` plugin type that provides ``setup_parser()``, ``command()``,
and ``register_plugin()``.

Rez API: ``rez.command.Command``
"""

from __future__ import annotations

import abc
from typing import Any

# Re-export native command utilities for drop-in compatibility
from rez_next._native.command import (  # type: ignore[import]  # noqa: F401
    CommandResult,
    execute_command,
    command_exists,
    get_command_output,
    get_command_path,
)


class Command(abc.ABC):
    """Base class for custom Rez subcommand plugins.

    Plugin modules under the ``command`` plugin type should:

    * HAVE a module-level docstring (used as command help text).
    * PROVIDE a ``setup_parser(parser, completions=False)`` function.
    * PROVIDE a ``command(opts, parser=None, extra_arg_groups=None)`` function.
    * PROVIDE a ``register_plugin()`` function returning a ``Command`` subclass.
    * MAY have a module-level ``command_behavior`` dict.

    Rez API: ``rez.command.Command``
    """

    def __init__(self) -> None:
        self.type_settings: dict[str, Any] = {}
        try:
            from rez_next.config import config as cfg
            self.type_settings = getattr(cfg, "plugins", {}).get(
                "extension", {}
            )
        except Exception:
            pass
        self.settings: Any = self.type_settings.get(self.name())

    @classmethod
    def name(cls) -> str:
        """Return the command name (also the rez subcommand name)."""
        raise NotImplementedError
