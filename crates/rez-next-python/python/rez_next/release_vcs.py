"""
Release VCS module — aligns with rez.release_vcs.

Provides an abstract version control system (VCS) interface for releasing
Rez packages, along with factory functions for discovering and creating
VCS instances.
"""

from __future__ import annotations

import abc
import os
import subprocess
from typing import Any, ClassVar, Optional

from rez_next.exceptions import RezSystemError
from rez_next.packages_ import get_developer_package


class ReleaseVCSError(RezSystemError):
    """Error raised for VCS-related failures in the release process."""


class ReleaseVCS(abc.ABC):
    """Abstract base class for version control system integrations.

    Subclasses must implement all abstract methods to provide VCS support
    for releasing Rez packages (e.g., Git, Mercurial).

    Rez API: ``rez.release_vcs.ReleaseVCS``
    """

    # Registry: maps VCS name → subclass
    _registry: ClassVar[dict[str, type[ReleaseVCS]]] = {}

    def __init__(
        self, pkg_root: str, vcs_root: Optional[str] = None
    ) -> None:
        """Initialize a VCS instance.

        Args:
            pkg_root: Root directory of the package source.
            vcs_root: Root directory of the VCS repository. If None,
                the VCS implementation should auto-detect it.
        """
        self.pkg_root: str = pkg_root
        self._package = None
        if vcs_root:
            self.vcs_root = vcs_root
        else:
            found = self.find_vcs_root(pkg_root)
            if found is None:
                raise ReleaseVCSError(
                    f"No VCS root found for: {pkg_root}"
                )
            self.vcs_root = found[0]

    @property
    def package(self):
        """The developer package at ``pkg_root`` (lazy-loaded)."""
        if self._package is None:
            try:
                self._package = get_developer_package(self.pkg_root)
            except Exception:
                self._package = None
        return self._package

    # ── Abstract methods ──────────────────────────────────────────────

    @classmethod
    @abc.abstractmethod
    def name(cls) -> str:
        """Return the VCS type name, e.g. ``'git'``."""

    @classmethod
    @abc.abstractmethod
    def is_valid_root(cls, path: str) -> bool:
        """Return ``True`` if *path* is a valid VCS root directory."""

    @classmethod
    @abc.abstractmethod
    def search_parents_for_root(cls) -> bool:
        """Return ``True`` if parent directories should be searched."""

    @classmethod
    def find_vcs_root(cls, path: str) -> Optional[tuple[str, int]]:
        """Find the VCS root directory by walking up parents.

        Returns:
            ``(vcs_root, depth)`` tuple where *depth* is the number of
            parent directories traversed, or ``None`` if not found.
        """
        if not cls.search_parents_for_root():
            if cls.is_valid_root(path):
                return (os.path.abspath(path), 0)
            return None

        current = os.path.abspath(path)
        depth = 0
        while True:
            if cls.is_valid_root(current):
                return (current, depth)
            parent = os.path.dirname(current)
            if parent == current:
                return None
            current = parent
            depth += 1

    @abc.abstractmethod
    def validate_repostate(self) -> None:
        """Ensure the working copy is up-to-date and clean."""

    @abc.abstractmethod
    def get_current_revision(self) -> object:
        """Get the current revision identifier (str, dict, etc.)."""

    @abc.abstractmethod
    def get_changelog(
        self,
        previous_revision: Any = None,
        max_revisions: Optional[int] = None,
    ) -> str:
        """Get changelog text since the given revision."""

    @abc.abstractmethod
    def tag_exists(self, tag_name: str) -> bool:
        """Check if a tag exists in the repository."""

    @abc.abstractmethod
    def create_release_tag(
        self, tag_name: str, message: Optional[str] = None
    ) -> None:
        """Create a release tag in the repository."""

    @classmethod
    @abc.abstractmethod
    def export(cls, revision: object, path: str) -> None:
        """Export the repository at the given revision to a directory.

        The directory at *path* must not exist (but its parent must).
        """

    # ── Optional helper methods ───────────────────────────────────────

    def get_current_branch(self) -> Optional[str]:
        """Get the current branch name, or ``None`` if in detached HEAD."""
        return None

    def find_executable(self, name: str) -> str:
        """Find a VCS executable by name.

        Raises ``ReleaseVCSError`` if not found.
        """
        exe = ReleaseVCS._which(name)
        if exe is None:
            raise ReleaseVCSError(f"VCS executable not found: {name}")
        return exe

    @staticmethod
    def _which(name: str) -> Optional[str]:
        """Find an executable in PATH."""
        for path_dir in os.environ.get("PATH", "").split(os.pathsep):
            candidate = os.path.join(path_dir, name)
            if os.path.isfile(candidate) and os.access(candidate, os.X_OK):
                return candidate
            # Windows: try with .exe extension
            candidate_exe = candidate + ".exe"
            if os.path.isfile(candidate_exe):
                return candidate_exe
        return None

    def _cmd(self, *args: str) -> list[str]:
        """Run an external command and return output lines.

        Raises ``ReleaseVCSError`` on failure.
        """
        try:
            result = subprocess.run(
                args,
                capture_output=True,
                text=True,
                check=False,
            )
            if result.returncode != 0:
                raise ReleaseVCSError(
                    f"Command failed (exit {result.returncode}): "
                    f"{' '.join(args)}\n{result.stderr.strip()}"
                )
            return [line for line in result.stdout.splitlines() if line]
        except FileNotFoundError:
            raise ReleaseVCSError(f"Executable not found: {args[0]}")
        except OSError as e:
            raise ReleaseVCSError(f"Command error: {e}")

    def __init_subclass__(cls, **kwargs: Any) -> None:
        """Auto-register subclasses in the VCS registry."""
        super().__init_subclass__(**kwargs)
        # Only register concrete subclasses (skip abstract base)
        if not getattr(cls, '__abstractmethods__', None):
            try:
                name = cls.name()
                ReleaseVCS._registry[name] = cls
            except NotImplementedError:
                pass


# ── Factory functions ──────────────────────────────────────────────


def get_release_vcs_types() -> list[str]:
    """Return all registered VCS type names.

    Returns:
        List of VCS names (e.g., ``['git', 'hg']``).

    Rez API: ``rez.release_vcs.get_release_vcs_types()``
    """
    return sorted(ReleaseVCS._registry.keys())


def create_release_vcs(
    path: str, vcs_name: Optional[str] = None
) -> ReleaseVCS:
    """Create a ``ReleaseVCS`` instance from a path.

    If *vcs_name* is given, use that specific VCS type. Otherwise,
    auto-detect by checking each registered VCS type.

    Args:
        path: Source package root path.
        vcs_name: Optional specific VCS type name.

    Returns:
        A ``ReleaseVCS`` instance.

    Raises:
        ReleaseVCSError: If no suitable VCS is found.

    Rez API: ``rez.release_vcs.create_release_vcs()``
    """
    if vcs_name:
        cls = ReleaseVCS._registry.get(vcs_name)
        if cls is None:
            raise ReleaseVCSError(f"Unknown VCS type: {vcs_name}")
        return cls(path)

    # Auto-detect: find the closest VCS root
    candidates: list[tuple[str, type[ReleaseVCS], int]] = []
    for name, cls in ReleaseVCS._registry.items():
        try:
            result = cls.find_vcs_root(path)
            if result is not None:
                candidates.append((name, cls, result[1]))
        except Exception:
            continue

    if not candidates:
        raise ReleaseVCSError(
            f"No VCS root found for path: {path}. "
            f"Available types: {list(ReleaseVCS._registry.keys())}"
        )

    # Pick the VCS with the shallowest depth (closest to path)
    candidates.sort(key=lambda x: x[2])
    return candidates[0][1](path)
