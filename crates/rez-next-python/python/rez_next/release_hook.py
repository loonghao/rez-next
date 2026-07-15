"""
release_hook — aligns with rez.release_hook.

Provides the abstract ``ReleaseHook`` base class, factory functions,
and the ``ReleaseHookEvent`` enum for managing custom behaviour during
package releases.

Key design:
- Single hook failure does not crash the release (graceful fallback)
- Clean ABC with no mutable defaults
- Factory functions with descriptive error messages
"""

from __future__ import annotations

import abc
import logging
from enum import Enum
from typing import Any

logger = logging.getLogger(__name__)


def get_release_hook_types() -> list[str]:
    """Return available release hook implementation names.

    Rez API: ``rez.release_hook.get_release_hook_types()``
    """
    from rez_next.plugin_managers import plugin_manager

    return sorted(plugin_manager.get_plugins("release_hook"))


def create_release_hook(name: str, source_path: str) -> ReleaseHook:
    """Create a single release hook instance of the given type.

    Args:
        name: Hook type name (e.g. ``'email'``).
        source_path: Path to the released source.

    Returns:
        A ``ReleaseHook`` instance.

    Rez API: ``rez.release_hook.create_release_hook()``
    """
    from rez_next.plugin_managers import plugin_manager

    return plugin_manager.create_instance("release_hook", name, source_path=source_path)


def create_release_hooks(
    names: list[str],
    source_path: str,
) -> list[ReleaseHook]:
    """Create release hook instances from a list of names.

    Unavailable hooks are logged as warnings but do not prevent other
    hooks from being created.

    Args:
        names: Hook type names.
        source_path: Path to the released source.

    Returns:
        List of successfully created hooks.

    Rez API: ``rez.release_hook.create_release_hooks()``
    """
    hooks: list[ReleaseHook] = []
    for name in names:
        try:
            hook = create_release_hook(name, source_path)
            hooks.append(hook)
        except Exception as exc:
            logger.warning("Release hook '%s' is not available: %s", name, exc)
    return hooks


class ReleaseHook(abc.ABC):
    """Abstract base class for release hook implementations.

    Subclasses implement one or more of the hook methods to inject custom
    behaviour during the release lifecycle.

    Rez API: ``rez.release_hook.ReleaseHook``
    """

    @classmethod
    def name(cls) -> str:
        """Return the hook name, e.g. ``'email'``."""
        raise NotImplementedError

    def __init__(self, source_path: str) -> None:
        self.source_path: str = source_path
        self.package = None  # Lazy-loaded

    def _get_package(self):
        if self.package is None:
            try:
                from rez_next.developer_package import DeveloperPackage

                self.package = DeveloperPackage.from_path(self.source_path)
            except Exception:
                self.package = None
        return self.package

    def pre_build(
        self,
        user: str = "",
        install_path: str = "",
        variants: Any = None,
        release_message: str | None = None,
        changelog: list[str] | None = None,
        previous_version: Any = None,
        previous_revision: Any = None,
        **kwargs: Any,
    ) -> None:
        """Hook called before the build step.

        Raise ``ReleaseHookCancellingError`` to cancel the release.

        Rez API: ``ReleaseHook.pre_build()``
        """
        pass

    def pre_release(
        self,
        user: str = "",
        install_path: str = "",
        variants: Any = None,
        release_message: str | None = None,
        changelog: list[str] | None = None,
        previous_version: Any = None,
        previous_revision: Any = None,
        **kwargs: Any,
    ) -> None:
        """Hook called before any package variants are released.

        Raise ``ReleaseHookCancellingError`` to cancel the release.

        Rez API: ``ReleaseHook.pre_release()``
        """
        pass

    def post_release(
        self,
        user: str = "",
        install_path: str = "",
        variants: Any = None,
        release_message: str | None = None,
        changelog: list[str] | None = None,
        previous_version: Any = None,
        previous_revision: Any = None,
        **kwargs: Any,
    ) -> None:
        """Hook called after all package variants have been released.

        Rez API: ``ReleaseHook.post_release()``
        """
        pass


class ReleaseHookEvent(Enum):
    """Enum for managing release hook events.

    Each member provides ``label``, ``noun``, and ``func_name`` attributes.

    Rez API: ``rez.release_hook.ReleaseHookEvent``
    """

    pre_build = ("pre-build", "build", "pre_build")
    pre_release = ("pre-release", "release", "pre_release")
    post_release = ("post-release", "release", "post_release")

    def __init__(self, label: str, noun: str, func_name: str) -> None:
        self.label: str = label
        self.noun: str = noun
        self.__name__: str = func_name
