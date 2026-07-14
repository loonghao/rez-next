"""
plugin_managers — aligns with rez.plugin_managers.

Manages loading of all types of Rez plugins via namespace packages and
entry points. Provides the ``plugin_manager`` singleton used throughout
for plugin discovery, registration, and instantiation.

Key differences from Rez:
- No vendor'd importlib_metadata fallback (Python 3.8+ stdlib)
- LazySingleton is local (no data_utils import chain)
- Failed plugins are logged, not silently swallowed
"""

from __future__ import annotations

import abc
import os
import sys
import types
import importlib
import importlib.metadata as importlib_metadata
import pkgutil
import logging
from typing import Any, ClassVar, TYPE_CHECKING

if TYPE_CHECKING:
    from rez_next.shells import Shell
    from rez_next.release_vcs import ReleaseVCS
    from rez_next.release_hook import ReleaseHook
    from rez_next.build_process import BuildProcess
    from rez_next.build_system import BuildSystem
    from rez_next.package_repository import PackageRepository
    from rez_next.command import Command

logger = logging.getLogger(__name__)


class LazySingleton:
    """Describes a class lazily instantiated as a singleton."""

    def __init__(self, cls: type) -> None:
        self._cls = cls
        self._instance: Any = None

    def __call__(self) -> Any:
        if self._instance is None:
            self._instance = self._cls()
        return self._instance


def extend_path(path: list[str], name: str) -> list[str]:
    """Extend a package's path with directories from the plugin search path.

    Rez API: ``rez.plugin_managers.extend_path()``
    """
    if not isinstance(path, list):
        return path
    pname = os.path.join(*name.split("."))
    init_py = "__init__" + os.extsep + "py"
    path = path[:]
    try:
        from rez_next.config import config as cfg
        plugin_dirs = getattr(cfg, "plugin_path", [])
    except Exception:
        plugin_dirs = []
    for dir_ in plugin_dirs:
        if not os.path.isdir(dir_):
            continue
        subdir = os.path.normcase(os.path.join(dir_, pname))
        initfile = os.path.join(subdir, init_py)
        if subdir not in path and os.path.isfile(initfile):
            path.append(subdir)
    return path


class RezPluginType(abc.ABC):
    """Abstract base representing a single plugin type.

    Subclasses must provide a ``type_name`` class attribute.

    Rez API: ``rez.plugin_managers.RezPluginType``
    """

    type_name: ClassVar[str] = ""

    def __init__(self) -> None:
        if not self.type_name:
            raise TypeError(
                "Subclasses of RezPluginType must provide a 'type_name' attribute"
            )
        self.pretty_type_name: str = self.type_name.replace("_", " ")
        self.plugin_classes: dict[str, type] = {}
        self.failed_plugins: dict[str, str] = {}
        self.plugin_modules: dict[str, types.ModuleType] = {}
        self.config_data: dict[str, Any] = {}
        self.load_plugins()

    def __repr__(self) -> str:
        return f"{self.__class__.__name__}({list(self.plugin_classes)})"

    def register_plugin(
        self,
        plugin_name: str,
        plugin_class: type,
        plugin_module: types.ModuleType,
    ) -> None:
        """Register a single plugin class and module."""
        self.plugin_classes[plugin_name] = plugin_class
        self.plugin_modules[plugin_name] = plugin_module

    def load_plugins(self) -> None:
        """Discover plugins from namespace packages and entry points."""
        self._load_from_namespace()
        self._load_from_entry_points()

    def _load_from_namespace(self) -> None:
        """Load plugins from ``rezplugins.<type_name>`` namespace."""
        type_module_name = f"rezplugins.{self.type_name}"
        try:
            package = importlib.import_module(type_module_name)
        except ImportError:
            return

        paths: list[str] = (
            [package.__path__]
            if isinstance(package.__path__, str)
            else list(package.__path__)
        )

        for path in reversed(paths):
            if not os.path.isdir(path):
                continue
            for _importer, modname, _ispkg in pkgutil.iter_modules(
                [path], f"{type_module_name}."
            ):
                plugin_name = modname.rsplit(".", 1)[-1]
                if plugin_name.startswith("_") or plugin_name == "rezconfig":
                    continue
                if plugin_name in self.plugin_modules:
                    continue
                try:
                    plugin_module = importlib.import_module(modname)
                    self._register_plugin_module(plugin_name, plugin_module, path)
                    self._load_config_from_plugin(plugin_module)
                except Exception as exc:
                    self.failed_plugins[plugin_name] = str(exc)
                    logger.debug("Failed to load plugin %s: %s", modname, exc)

    def _load_from_entry_points(self) -> None:
        """Load plugins registered as ``rez.plugins.<type_name>`` entry points."""
        group = f"rez.plugins.{self.type_name}"
        try:
            discovered = importlib_metadata.entry_points(group=group)
        except TypeError:
            discovered = importlib_metadata.entry_points().get(group, [])

        for ep in discovered:
            if ep.name in self.plugin_modules:
                continue
            try:
                plugin_module = ep.load()
                plugin_path = os.path.dirname(
                    getattr(plugin_module, "__file__", "")
                )
                self._register_plugin_module(ep.name, plugin_module, plugin_path)
                self._load_config_from_plugin(plugin_module)
            except Exception as exc:
                self.failed_plugins[ep.name] = str(exc)
                logger.debug("Failed to load entry point %s: %s", ep.name, exc)

    def _register_plugin_module(
        self,
        plugin_name: str,
        plugin_module: types.ModuleType,
        _plugin_path: str,
    ) -> None:
        if not hasattr(plugin_module, "register_plugin"):
            return
        fn = plugin_module.register_plugin
        if not callable(fn):
            return
        cls = fn()
        if cls is not None:
            self.register_plugin(plugin_name, cls, plugin_module)

    def _load_config_from_plugin(self, plugin_module: types.ModuleType) -> None:
        cfg_path = os.path.join(
            os.path.dirname(getattr(plugin_module, "__file__", "")), "rezconfig"
        )
        if os.path.isfile(cfg_path):
            self.config_data[os.path.basename(plugin_module.__name__)] = {}

    def get_plugin_class(self, plugin_name: str) -> type:
        """Return the class registered under *plugin_name*."""
        try:
            return self.plugin_classes[plugin_name]
        except KeyError:
            from rez_next.exceptions import RezPluginError
            raise RezPluginError(
                f"Unrecognised {self.pretty_type_name} plugin: '{plugin_name}'"
            ) from None

    def get_plugin_module(self, plugin_name: str) -> types.ModuleType:
        """Return the module containing the named plugin."""
        try:
            return self.plugin_modules[plugin_name]
        except KeyError:
            from rez_next.exceptions import RezPluginError
            raise RezPluginError(
                f"Unrecognised {self.pretty_type_name} plugin: '{plugin_name}'"
            ) from None

    def create_instance(self, plugin: str, **kwargs: Any) -> Any:
        """Create and return an instance of the given plugin."""
        return self.get_plugin_class(plugin)(**kwargs)


class RezPluginManager:
    """Primary interface for working with all registered plugins.

    Provides discovery, registration, lookup, and instantiation across
    all plugin types.

    Rez API: ``rez.plugin_managers.RezPluginManager``
    """

    def __init__(self) -> None:
        self._plugin_types: dict[str, LazySingleton] = {}

    def register_plugin_type(self, type_class: type[RezPluginType]) -> None:
        """Register a new plugin type class."""
        if not issubclass(type_class, RezPluginType):
            raise TypeError("type_class must be a RezPluginType subclass")
        if not type_class.type_name:
            raise TypeError("Subclass must provide a 'type_name' attribute")
        self._plugin_types[type_class.type_name] = LazySingleton(type_class)

    def get_plugin_types(self) -> list[str]:
        """Return a list of registered plugin type names."""
        return list(self._plugin_types.keys())

    def get_plugins(self, plugin_type: str) -> list[str]:
        """Return plugin names available for *plugin_type*."""
        return list(self._get_plugin_type(plugin_type).plugin_classes.keys())

    def get_plugin_class(self, plugin_type: str, plugin_name: str) -> type:
        """Return the class registered for *plugin_name* under *plugin_type*."""
        return self._get_plugin_type(plugin_type).get_plugin_class(plugin_name)

    def get_plugin_module(
        self, plugin_type: str, plugin_name: str
    ) -> types.ModuleType:
        """Return the module for the named plugin."""
        return self._get_plugin_type(plugin_type).get_plugin_module(plugin_name)

    def get_failed_plugins(self, plugin_type: str) -> list[tuple[str, str]]:
        """Return ``[(name, reason), ...]`` for plugins that failed to load."""
        return list(
            self._get_plugin_type(plugin_type).failed_plugins.items()
        )

    def create_instance(
        self, plugin_type: str, plugin_name: str, **kwargs: Any
    ) -> Any:
        """Create and return an instance of the named plugin."""
        return self._get_plugin_type(plugin_type).create_instance(
            plugin_name, **kwargs
        )

    def get_summary_string(self) -> str:
        """Return a formatted summary of all loaded plugins."""
        lines: list[str] = []
        for plugin_type in sorted(self.get_plugin_types()):
            pt = self._get_plugin_type(plugin_type)
            for name in sorted(pt.plugin_classes):
                lines.append(
                    f"{pt.pretty_type_name:20s} {name:20s} loaded"
                )
            for name, reason in sorted(pt.failed_plugins.items()):
                lines.append(
                    f"{pt.pretty_type_name:20s} {name:20s} FAILED: {reason}"
                )
        return "\n".join(lines) if lines else "(no plugins registered)"

    def _get_plugin_type(self, plugin_type: str) -> RezPluginType:
        try:
            return self._plugin_types[plugin_type]()
        except KeyError:
            from rez_next.exceptions import RezPluginError
            raise RezPluginError(
                f"Unrecognised plugin type: '{plugin_type}'"
            ) from None


# ── Concrete plugin types ──────────────────────────────────────────────────


class ShellPluginType(RezPluginType):
    """Shell plugin type (bash, tcsh, etc.)."""
    type_name = "shell"


class ReleaseVCSPluginType(RezPluginType):
    """VCS plugin type (git, etc.)."""
    type_name = "release_vcs"


class ReleaseHookPluginType(RezPluginType):
    """Release hook plugin type (email, etc.)."""
    type_name = "release_hook"


class BuildSystemPluginType(RezPluginType):
    """Build system plugin type (cmake, make, etc.)."""
    type_name = "build_system"


class PackageRepositoryPluginType(RezPluginType):
    """Package repository plugin type (filesystem, memory, etc.)."""
    type_name = "package_repository"


class BuildProcessPluginType(RezPluginType):
    """Build process plugin type."""
    type_name = "build_process"


class CommandPluginType(RezPluginType):
    """Command plugin type (custom subcommands)."""
    type_name = "command"


# ── Singleton ──────────────────────────────────────────────────────────────

plugin_manager = RezPluginManager()

plugin_manager.register_plugin_type(ShellPluginType)
plugin_manager.register_plugin_type(ReleaseVCSPluginType)
plugin_manager.register_plugin_type(ReleaseHookPluginType)
plugin_manager.register_plugin_type(BuildSystemPluginType)
plugin_manager.register_plugin_type(PackageRepositoryPluginType)
plugin_manager.register_plugin_type(BuildProcessPluginType)
plugin_manager.register_plugin_type(CommandPluginType)
