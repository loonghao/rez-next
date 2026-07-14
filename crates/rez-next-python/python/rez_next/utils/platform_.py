"""
Platform abstraction for Rez-next.

Mirrors ``rez.utils.platform_`` API:
- ``Platform`` base class with cached cross-platform properties
- ``LinuxPlatform``, ``OSXPlatform``, ``WindowsPlatform`` concrete subclasses
- Global ``platform_`` singleton instance

Design principles (SOLID):
- Single-responsibility: Each platform subclass owns its detection logic.
- Open/closed: New platforms are added by subclassing ``Platform``, not by
  editing the base class.
- Liskov substitution: All subclasses provide the same public property set.
- Interface segregation: Consumers depend on the ``Platform`` interface
  rather than concrete subclasses.
- Dependency inversion: The singleton selector depends on the abstract
  ``Platform`` interface, not on subclasses.
"""

from __future__ import annotations

import functools
import os
import platform
import subprocess
import tempfile
from abc import ABC, abstractmethod
from typing import TYPE_CHECKING

import rez_next._native  # noqa: F401 — ensure native extension is loaded
from rez_next.util import (
    get_architecture,
    get_hostname,
    get_username,
    is_linux,
    is_macos,
    is_windows,
    which,
)

if TYPE_CHECKING:
    from types import TracebackType


__all__ = [
    "Platform",
    "LinuxPlatform",
    "OSXPlatform",
    "WindowsPlatform",
    "platform_",
]


# ── Helpers ──────────────────────────────────────────────────────────────────

def _physical_cores_linux() -> int:
    """Detect physical CPU cores on Linux via ``/proc/cpuinfo``."""
    try:
        cores: set[tuple[str, str]] = set()
        with open("/proc/cpuinfo") as fh:
            phys_id = None
            core_id = None
            for line in fh:
                line = line.strip()
                if line.startswith("physical id"):
                    phys_id = line.split(":")[-1].strip()
                elif line.startswith("core id"):
                    core_id = line.split(":")[-1].strip()
                elif line == "" and phys_id is not None and core_id is not None:
                    cores.add((phys_id, core_id))
                    phys_id = None
                    core_id = None
        if cores:
            return max(len(cores), 1)
    except (OSError, IOError):
        pass
    # Fallback: logical cores (may overcount on hyperthreading)
    return os.cpu_count() or 1


def _physical_cores_macos() -> int:
    """Detect physical CPU cores on macOS via ``sysctl``."""
    try:
        result = subprocess.run(
            ["sysctl", "-n", "hw.physicalcpu"],
            capture_output=True,
            text=True,
            timeout=5,
        )
        if result.returncode == 0:
            val = result.stdout.strip()
            if val:
                return max(int(val), 1)
    except (OSError, subprocess.SubprocessError, ValueError):
        pass
    return os.cpu_count() or 1


def _physical_cores_windows() -> int:
    """Detect physical CPU cores on Windows via ``wmic``."""
    try:
        result = subprocess.run(
            ["wmic", "cpu", "get", "NumberOfCores", "/value"],
            capture_output=True,
            text=True,
            timeout=5,
        )
        if result.returncode == 0:
            total = 0
            for line in result.stdout.splitlines():
                if line.startswith("NumberOfCores"):
                    total += int(line.split("=")[-1].strip())
            return max(total, 1)
    except (OSError, subprocess.SubprocessError, ValueError):
        pass
    return os.cpu_count() or 1


# ── Platform base class ──────────────────────────────────────────────────────

class Platform(ABC):
    """Abstract base for platform abstraction.

    Each concrete subclass implements the ``_<name>`` private methods;
    public properties are ``@functools.cached_property`` for thread-safe
    one-time evaluation.
    """

    name: str = ""

    # ── Abstract hooks ──────────────────────────────────────────────────

    @abstractmethod
    def _os(self) -> str:
        """Return a human-readable OS identifier (e.g. ``"Ubuntu-20.04"``)."""

    @abstractmethod
    def _terminal_emulator_command(self) -> str | None:
        """Return command to open a new terminal (``None`` for unsupported)."""

    @abstractmethod
    def _new_session_popen_args(self) -> dict[str, object]:
        """Return ``subprocess.Popen`` kwargs for a new-session group."""

    @abstractmethod
    def _image_viewer(self) -> str | None:
        """Return path to an image viewer (``None`` = use browser fallback)."""

    @abstractmethod
    def _editor(self) -> str | None:
        """Return path to a text editor (``None`` = unknown)."""

    @abstractmethod
    def _difftool(self) -> str | None:
        """Return path to a diff tool (e.g. ``meld``, ``diff``)."""

    @abstractmethod
    def _physical_cores(self) -> int:
        """Return the count of physical CPU cores."""

    @abstractmethod
    def _symlink(self, source: str, link_name: str) -> None:
        """Create a symlink from *source* to *link_name*."""

    # ── Public cached properties ────────────────────────────────────────

    @functools.cached_property
    def arch(self) -> str:
        """System architecture (e.g. ``"x86_64"``, ``"aarch64"``)."""
        return get_architecture()

    @functools.cached_property
    def os(self) -> str:  # noqa: A003 — matches rez API naming
        """Human-readable OS identifier."""
        return self._os()

    @functools.cached_property
    def terminal_emulator_command(self) -> str | None:
        """Command to open a new terminal running a given command."""
        return self._terminal_emulator_command()

    @functools.cached_property
    def new_session_popen_args(self) -> dict[str, object]:
        """Kwargs for ``subprocess.Popen`` to start a new process group."""
        return self._new_session_popen_args()

    @functools.cached_property
    def image_viewer(self) -> str | None:
        """Path to an image viewer binary."""
        return self._image_viewer()

    @functools.cached_property
    def editor(self) -> str | None:
        """Path to a text editor binary."""
        return self._editor()

    @functools.cached_property
    def difftool(self) -> str | None:
        """Path to a diff/merge tool binary."""
        return self._difftool()

    @functools.cached_property
    def tmpdir(self) -> str:
        """System temporary directory path."""
        return tempfile.gettempdir()

    @functools.cached_property
    def physical_cores(self) -> int:
        """Number of physical CPU cores."""
        return self._physical_cores()

    @functools.cached_property
    def logical_cores(self) -> int:
        """Number of logical CPU cores (may include hyper-threads)."""
        return os.cpu_count() or 1

    @property
    def has_case_sensitive_filesystem(self) -> bool:
        """Whether the filesystem is case-sensitive (default ``True``)."""
        return True

    # ── Public methods ──────────────────────────────────────────────────

    def symlink(self, source: str, link_name: str) -> None:
        """Create a symbolic link.

        Args:
            source: Target path the link should point to.
            link_name: Path where the link will be created.
        """
        self._symlink(source, link_name)


# ── Concrete implementations ────────────────────────────────────────────────

# NOTE: We keep the legacy name ``OSXPlatform`` (not ``MacOSPlatform``) for
# API compatibility with ``rez.utils.platform_``.  This avoids an unnecessary
# breaking change for code that explicitly references the class.

class LinuxPlatform(Platform):
    """Platform abstraction for Linux."""

    name = "linux"

    def _os(self) -> str:
        return _detect_linux_distro()

    def _terminal_emulator_command(self) -> str | None:
        for cmd in ("x-terminal-emulator", "xterm", "konsole"):
            exe = which(cmd)
            if exe:
                if cmd == "konsole":
                    return f"{exe} --noclose -e"
                return f"{exe} -hold -e"
        return None

    def _new_session_popen_args(self) -> dict[str, object]:
        return {}

    def _image_viewer(self) -> str | None:
        for cmd in ("xdg-open", "eog", "kview"):
            exe = which(cmd)
            if exe:
                return exe
        return None

    def _editor(self) -> str | None:
        editor_env = os.environ.get("EDITOR")
        if editor_env:
            return editor_env
        for cmd in ("vi", "vim", "xdg-open"):
            exe = which(cmd)
            if exe:
                return exe
        return None

    def _difftool(self) -> str | None:
        for cmd in ("kdiff3", "meld", "diff"):
            exe = which(cmd)
            if exe:
                return exe
        return None

    def _physical_cores(self) -> int:
        return _physical_cores_linux()

    def _symlink(self, source: str, link_name: str) -> None:
        os.symlink(source, link_name)


class OSXPlatform(Platform):
    """Platform abstraction for macOS."""

    name = "osx"

    def _os(self) -> str:
        ver = platform.mac_ver()[0]
        return f"osx-{ver}" if ver else "osx"

    def _terminal_emulator_command(self) -> str | None:
        for cmd in ("x-terminal-emulator", "xterm"):
            exe = which(cmd)
            if exe:
                return f"{exe} -hold -e"
        return None

    def _new_session_popen_args(self) -> dict[str, object]:
        return {}

    def _image_viewer(self) -> str | None:
        return which("open")

    def _editor(self) -> str | None:
        return which("open")

    def _difftool(self) -> str | None:
        for cmd in ("meld", "diff"):
            exe = which(cmd)
            if exe:
                return exe
        return None

    def _physical_cores(self) -> int:
        return _physical_cores_macos()

    def _symlink(self, source: str, link_name: str) -> None:
        os.symlink(source, link_name)


class WindowsPlatform(Platform):
    """Platform abstraction for Windows."""

    name = "windows"

    def _os(self) -> str:
        try:
            ver, sp, _, _ = platform.win32_ver()
            if sp:
                return f"windows-{ver}.{sp}"
            return f"windows-{ver}" if ver else "windows"
        except Exception:  # noqa: BLE001 — broad except for platform fallback
            return "windows"

    def _terminal_emulator_command(self) -> str | None:
        return "START"

    def _new_session_popen_args(self) -> dict[str, object]:
        return {"creationflags": 0x00000010}  # CREATE_NEW_PROCESS_GROUP

    def _image_viewer(self) -> str | None:
        return ""

    def _editor(self) -> str | None:
        return ""

    def _difftool(self) -> str | None:
        return which("diff")

    @property
    def has_case_sensitive_filesystem(self) -> bool:
        return False

    def _physical_cores(self) -> int:
        return _physical_cores_windows()

    def _symlink(self, source: str, link_name: str) -> None:
        import ctypes
        from ctypes import wintypes

        kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
        CreateSymbolicLinkW = kernel32.CreateSymbolicLinkW
        CreateSymbolicLinkW.argtypes = (
            wintypes.LPCWSTR,
            wintypes.LPCWSTR,
            wintypes.DWORD,
        )
        CreateSymbolicLinkW.restype = wintypes.BOOLEAN

        flags = 0
        if os.path.isdir(source):
            flags |= 0x1  # SYMBOLIC_LINK_FLAG_DIRECTORY

        if not CreateSymbolicLinkW(link_name, source, flags):
            raise OSError(
                f"Failed to create symlink: {link_name!r} -> {source!r}"
            )


# ── OS version detection helpers ─────────────────────────────────────────────

def _detect_linux_distro() -> str:
    """Detect the Linux distribution name and version.

    Tries (in order):
    1. ``/etc/lsb-release``
    2. ``lsb_release -a``
    3. ``/etc/os-release``
    """
    # Try /etc/lsb-release
    lsb_paths = ("/etc/lsb-release", "/etc/gentoo-release")
    for path in lsb_paths:
        try:
            with open(path) as fh:
                data: dict[str, str] = {}
                for line in fh:
                    if "=" in line:
                        k, v = line.strip().split("=", 1)
                        data[k] = v.strip('"')
            dist = data.get("DISTRIB_ID")
            ver = data.get("DISTRIB_RELEASE")
            if dist and ver:
                return f"{dist}-{ver}"
        except (OSError, IOError):
            continue

    # Try /etc/os-release
    try:
        with open("/etc/os-release") as fh:
            data = {}
            for line in fh:
                if "=" in line:
                    k, v = line.strip().split("=", 1)
                    data[k] = v.strip('"')
        dist = data.get("ID") or data.get("NAME", "").split()[0]
        ver = data.get("VERSION_ID")
        if dist and ver:
            return f"{dist.capitalize()}-{ver}"
        elif dist:
            return dist.capitalize()
    except (OSError, IOError):
        pass

    # Fallback: platform module
    return platform.platform(terse=True)


# ── Global singleton ────────────────────────────────────────────────────────

platform_: Platform
"""Global ``Platform`` singleton matching the running OS."""

if is_linux():
    platform_ = LinuxPlatform()
elif is_macos():
    platform_ = OSXPlatform()
elif is_windows():
    platform_ = WindowsPlatform()
else:
    # Fallback: best-effort generic platform
    class _GenericPlatform(Platform):
        name = platform.system().lower()

        def _os(self) -> str:
            return platform.platform(terse=True)

        def _terminal_emulator_command(self) -> str | None:
            return None

        def _new_session_popen_args(self) -> dict[str, object]:
            return {}

        def _image_viewer(self) -> str | None:
            return None

        def _editor(self) -> str | None:
            return None

        def _difftool(self) -> str | None:
            return which("diff")

        def _physical_cores(self) -> int:
            return os.cpu_count() or 1

        def _symlink(self, source: str, link_name: str) -> None:
            os.symlink(source, link_name)

    platform_ = _GenericPlatform()
