"""
build_system — aligns with rez.build_system.

Provides the abstract ``BuildSystem`` base class, the ``BuildResult``
type alias, and factory functions for discovering and creating build
system instances from source directories.

Designed with:
- Clean ABC hierarchy (no mutable default args)
- Clear error messages showing available build systems
- No hidden state or side effects in class methods
- Rez API compatibility without deprecated parameters
"""

from __future__ import annotations

import abc
import os
from typing import Any, Optional, Sequence, TypedDict, TYPE_CHECKING

if TYPE_CHECKING:
    import argparse
    from rez_next.resolved_context import ResolvedContext
    from rez_next.rex import RexExecutor
    from rez_next.build_process import BuildType
    from rez_next.developer_package import DeveloperPackage


class BuildResult(TypedDict, total=False):
    """Result dictionary returned by ``BuildSystem.build()``.

    Rez API: ``rez.build_system.BuildResult``

    Attributes:
        success: Whether the build succeeded.
        extra_files: Additional files produced by the build.
        build_env_script: Path to a generated build environment script.
    """
    success: bool
    extra_files: list[str]
    build_env_script: str


# ── Factory functions ─────────────────────────────────────────────────────


def get_buildsys_types() -> list[str]:
    """Return available build system implementation names (e.g. ``'cmake'``, ``'make'``).

    Rez API: ``rez.build_system.get_buildsys_types()``
    """
    from rez_next.plugin_managers import plugin_manager
    return sorted(plugin_manager.get_plugins("build_system"))


def get_valid_build_systems(
    working_dir: str,
    package: Any = None,
) -> list[type[BuildSystem]]:
    """Return BuildSystem subclasses that could build the source in *working_dir*.

    Detection logic:
    1. If the package defines ``build_command`` → ``custom`` build system.
    2. If the package defines ``build_system`` → use that specific system.
    3. Otherwise, iterate all registered build systems and check
       ``is_valid_root()``, pruning child build systems from the list.

    Rez API: ``rez.build_system.get_valid_build_systems()``
    """
    from rez_next.plugin_managers import plugin_manager
    from rez_next.developer_package import DeveloperPackage
    from rez_next.exceptions import ResourceContentError

    if package is None:
        try:
            package = DeveloperPackage.from_path(working_dir)
        except (ResourceContentError, FileNotFoundError):
            pass

    if package is not None:
        build_command = getattr(package, "build_command", None)
        if build_command is not None:
            buildsys_name: Optional[str] = "custom"
        else:
            buildsys_name = getattr(package, "build_system", None)

        if buildsys_name:
            cls = plugin_manager.get_plugin_class("build_system", buildsys_name)
            return [cls]

    clss: list[type[BuildSystem]] = []
    for name in get_buildsys_types():
        cls = plugin_manager.get_plugin_class("build_system", name)
        if cls.is_valid_root(working_dir, package=package):
            clss.append(cls)

    # Remove child build systems (e.g. if cmake generates make, remove make)
    child_cls: set = {x.child_build_system() for x in clss if x.child_build_system()}
    clss = [c for c in clss if c not in child_cls]
    return clss


def create_build_system(
    working_dir: str,
    buildsys_type: Optional[str] = None,
    package: Any = None,
    opts: Optional[argparse.Namespace] = None,
    write_build_scripts: bool = False,
    verbose: bool = False,
    build_args: Optional[Sequence[str]] = None,
    child_build_args: Optional[list[str]] = None,
) -> BuildSystem:
    """Create a BuildSystem instance for the source in *working_dir*.

    If *buildsys_type* is not specified, auto-detect via
    ``get_valid_build_systems()``. Raises ``BuildSystemError`` when
    no single matching build system can be determined.

    Rez API: ``rez.build_system.create_build_system()``
    """
    from rez_next.plugin_managers import plugin_manager
    from rez_next.exceptions import BuildSystemError

    if buildsys_type is None:
        clss = get_valid_build_systems(working_dir, package=package)
        if not clss:
            raise BuildSystemError(
                f"No build system is associated with the path: {working_dir}"
            )
        if len(clss) > 1:
            available = ", ".join(c.name() for c in clss)
            raise BuildSystemError(
                f"Source could be built with one of: {available}. "
                "Please specify a build system."
            )
        buildsys_type = clss[0].name()

    cls = plugin_manager.get_plugin_class("build_system", buildsys_type)
    return cls(
        working_dir,
        opts=opts,
        package=package,
        write_build_scripts=write_build_scripts,
        verbose=verbose,
        build_args=build_args or [],
        child_build_args=child_build_args or [],
    )


# ── Abstract base class ───────────────────────────────────────────────────


class BuildSystem(abc.ABC):
    """Abstract base class for build system integrations.

    Subclasses represent concrete build systems such as cmake, make, scons,
    etc. Each subclass must implement ``name()``, ``is_valid_root()``, and
    ``build()``.

    Rez API: ``rez.build_system.BuildSystem``
    """

    # ── Class-level interface ─────────────────────────────────────────

    @classmethod
    def name(cls) -> str:
        """Return the build system name, e.g. ``'cmake'`` or ``'make'``."""
        raise NotImplementedError

    @classmethod
    def is_valid_root(cls, path: str, package: Any = None) -> bool:
        """Return True if this build system can build the source at *path*."""
        raise NotImplementedError

    @classmethod
    def child_build_system(cls) -> Optional[str]:
        """Return the name of the child build system, if any.

        For example, cmake generates makefiles, so ``cmake`` would return
        ``'make'`` here. This prevents both cmake and make from appearing
        in auto-detection results.
        """
        return None

    @classmethod
    def bind_cli(
        cls,
        parser: argparse.ArgumentParser,
        group: argparse._ArgumentGroup,  # noqa: SLF001
    ) -> None:
        """Expose build-system-specific CLI arguments.

        Arguments should be added to *group* rather than *parser* directly.
        """
        pass

    # ── Instance interface ───────────────────────────────────────────

    def __init__(
        self,
        working_dir: str,
        opts: Optional[argparse.Namespace] = None,
        package: Any = None,
        write_build_scripts: bool = False,
        verbose: bool = False,
        build_args: Sequence[str] = (),
        child_build_args: list[str] = (),
    ) -> None:
        from rez_next.exceptions import BuildSystemError
        if not self.is_valid_root(working_dir):
            raise BuildSystemError(
                f"Not a valid working directory for build system "
                f"{self.name()!r}: {working_dir}"
            )
        self.working_dir: str = working_dir
        self.package = package
        self.write_build_scripts = write_build_scripts
        self.verbose = verbose
        self.build_args = list(build_args)
        self.child_build_args = list(child_build_args)
        self.opts = opts

    def build(
        self,
        context: ResolvedContext,
        variant: Any,
        build_path: str,
        install_path: str,
        install: bool = False,
        build_type: BuildType = None,  # noqa: ANN401
    ) -> BuildResult:
        """Execute the build.

        Args:
            context: The resolved context for the build environment.
            variant: The variant being built.
            build_path: Directory for build outputs.
            install_path: Directory for installed files.
            install: If True, install after building.
            build_type: ``BuildType.local`` or ``BuildType.central``.

        Returns:
            A ``BuildResult`` dictionary.
        """
        raise NotImplementedError

    # ── Standard environment variables ────────────────────────────────

    @classmethod
    def set_standard_vars(
        cls,
        executor: RexExecutor,
        context: ResolvedContext,
        variant: Any,
        build_type: BuildType,
        install: bool,
        build_path: str,
        install_path: Optional[str] = None,
    ) -> None:
        """Set standard environment variables that all build systems can rely on.

        Sets variables like ``REZ_BUILD_ENV``, ``REZ_BUILD_PATH``,
        ``REZ_BUILD_THREAD_COUNT``, ``REZ_BUILD_PROJECT_VERSION``, etc.
        """
        from rez_next.rex import literal
        from rez_next.config import config as rez_config

        package = variant.parent if hasattr(variant, "parent") else variant
        description = getattr(package, "description", None) or ""
        variant_requires = [
            str(r) for r in getattr(variant, "variant_requires", [])
        ]
        subpath = getattr(variant, "_non_shortlinked_subpath", "")

        build_type_name = (
            build_type.name if hasattr(build_type, "name") else str(build_type)
        )

        vars_: dict[str, Any] = {
            "REZ_BUILD_ENV": literal("1"),
            "REZ_BUILD_PATH": literal(executor.normalize_path(build_path)),
            "REZ_BUILD_THREAD_COUNT": literal(
                str(getattr(rez_config, "build_thread_count", 4))
            ),
            "REZ_BUILD_VARIANT_INDEX": literal(
                str(getattr(variant, "index", 0) or 0)
            ),
            "REZ_BUILD_VARIANT_REQUIRES": literal(" ".join(variant_requires)),
            "REZ_BUILD_VARIANT_SUBPATH": literal(
                executor.normalize_path(subpath)
            ),
            "REZ_BUILD_PROJECT_VERSION": literal(str(package.version)),
            "REZ_BUILD_PROJECT_NAME": literal(package.name),
            "REZ_BUILD_PROJECT_DESCRIPTION": literal(description.strip()),
            "REZ_BUILD_PROJECT_FILE": literal(
                getattr(package, "filepath", "")
            ),
            "REZ_BUILD_SOURCE_PATH": literal(
                executor.normalize_path(
                    os.path.dirname(getattr(package, "filepath", ""))
                )
            ),
            "REZ_BUILD_REQUIRES": literal(" ".join(
                str(x) for x in context.requested_packages(True)
            )),
            "REZ_BUILD_REQUIRES_UNVERSIONED": literal(" ".join(
                x.name for x in context.requested_packages(True)
            )),
            "REZ_BUILD_TYPE": literal(build_type_name),
            "REZ_BUILD_INSTALL": literal("1" if install else "0"),
        }
        if install_path:
            vars_["REZ_BUILD_INSTALL_PATH"] = literal(
                executor.normalize_path(install_path)
            )

        for key, value in vars_.items():
            executor.env[key] = value

    @classmethod
    def add_pre_build_commands(
        cls,
        executor: RexExecutor,
        variant: Any,
        build_type: BuildType,
        install: bool,
        build_path: str,
        install_path: Optional[str] = None,
    ) -> None:
        """Execute ``pre_build_commands`` from the package if present."""
        pre_build_commands = getattr(variant, "pre_build_commands", None)
        if not pre_build_commands:
            return

        from rez_next.utils.data_utils import RO_AttrDictWrapper as ROA
        from rez_next.rex_bindings import VariantBinding

        build_ns = {
            "build_type": build_type.name if hasattr(build_type, "name") else str(build_type),
            "install": install,
            "build_path": executor.normalize_path(build_path),
            "install_path": (
                executor.normalize_path(install_path) if install_path else None
            ),
        }

        bound_variant = VariantBinding(
            variant,
            interpreter=executor.interpreter,
        )
        with executor.reset_globals():
            executor.bind("this", bound_variant)
            executor.bind("build", ROA(build_ns))
            executor.execute_code(pre_build_commands)

    @classmethod
    def add_standard_build_actions(
        cls,
        executor: RexExecutor,
        context: ResolvedContext,
        variant: Any,
        build_type: BuildType,
        install: bool,
        build_path: str,
        install_path: Optional[str] = None,
    ) -> None:
        """Execute standard build actions common to every build system.

        Currently calls ``set_standard_vars()`` and ``add_pre_build_commands()``.
        """
        cls.set_standard_vars(
            executor=executor,
            context=context,
            variant=variant,
            build_type=build_type,
            install=install,
            build_path=build_path,
            install_path=install_path,
        )
        cls.add_pre_build_commands(
            executor=executor,
            variant=variant,
            build_type=build_type,
            install=install,
            build_path=build_path,
            install_path=install_path,
        )
