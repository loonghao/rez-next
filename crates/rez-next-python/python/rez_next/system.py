"""
Rez-compatible system abstraction.

Mirrors ``rez.system`` public API:

- ``system`` singleton (``System`` instance)
- ``System`` class with cached properties for platform, arch, OS, variant,
  shell, user, hostname, etc.
- ``create_system()`` factory function
- ``get_system()`` factory function

Design decisions:
- Wraps the native Rust ``System`` struct for core properties (platform,
  arch, os) and adds Python-level methods that are hard to implement in Rust
  (shell detection, summary string, cache clearing).
- Uses ``functools.cached_property`` for thread-safe lazy evaluation.
- ``get_system()`` / ``create_system()`` allow testing with custom instances.

Lessons from upstream rez issues:
- #436 (MAX_PATH): Path operations use ``os.path`` with long-path support
- #446 (cache staleness): ``clear_caches()`` exposes both soft and hard clear
"""

from __future__ import annotations

import os as _os
import platform as _stdlib_platform
import re
from functools import cached_property

import rez_next._native  # noqa: F401 — ensure extension module
from rez_next._native.system import System as _NativeSystem  # noqa: F401
from rez_next._native.system import system as _native_system
from rez_next.util import get_architecture, get_hostname, get_username
from rez_next.util import which as _which

__all__ = [
    "System",
    "create_system",
    "get_system",
    "system",
]


class System:
    """Access to underlying system data.

    Wraps the native Rust ``System`` struct and extends it with
    Python-level properties and methods matching the upstream ``rez.system``
    public API.

    Args:
        native: Optional native ``System`` instance. If not provided,
            uses the global native singleton.
    """

    def __init__(self, native: _NativeSystem | None = None) -> None:
        self._native = native or _native_system

    # ── Core Properties ──────────────────────────────────────────────────

    @property
    def rez_version(self) -> str:
        """Returns the current version of rez-next."""
        return getattr(self._native, "rez_version", "0.0.0")

    @cached_property
    def platform(self) -> str:
        """Get the current platform name.

        Returns:
            The current platform (``"windows"``, ``"linux"``, ``"osx"``, etc).
        """
        return self._native.platform or _stdlib_platform.system().lower()

    @cached_property
    def arch(self) -> str:
        """Get the current architecture.

        Returns:
            The current architecture (``"x86_64"``, ``"AMD64"``, ``"aarch64"``, etc).
        """
        return self._make_safe_version_string(
            self._native.arch or get_architecture() or _stdlib_platform.machine()
        )

    @cached_property
    def os(self) -> str:
        """Get the current operating system.

        Returns:
            OS identifier such as ``"windows-10.0.26100.SP0"``,
            ``"Ubuntu-22.04"``, or ``"osx-14.5"``.
        """
        return self._make_safe_version_string(self._native.os or self._detect_os())

    @cached_property
    def variant(self) -> list[str]:
        """Returns a list of the form ``["platform-X", "arch-X", "os-X"]``
        suitable for use as a variant in a system-dependent package."""
        return [
            f"platform-{self.platform}",
            f"arch-{self.arch}",
            f"os-{self.os}",
        ]

    @cached_property
    def shell(self) -> str:
        """Get the current shell.

        Returns:
            The current shell this process is running in (``"powershell"``,
            ``"bash"``, ``"tcsh"``, etc). On Windows, always returns
            ``"powershell"``.
        """
        if self.platform == "windows":
            return "powershell"
        parent_pid = _os.getppid()
        if parent_pid == 0:
            return "bash"
        # Infer shell from /proc/self/comm or /proc/<ppid>/cmdline
        try:
            import subprocess as sp

            result = sp.run(
                ["ps", "-o", "args=", "-p", str(parent_pid)],
                capture_output=True,
                text=True,
                timeout=5,
            )
            if result.returncode == 0:
                cmdline = result.stdout.strip()
                for shell_candidate in ("bash", "zsh", "tcsh", "csh", "ksh", "sh"):
                    if shell_candidate in cmdline:
                        return shell_candidate
        except Exception:  # noqa: BLE001 — broad except for platform fallback
            pass
        return "bash"

    @cached_property
    def user(self) -> str:
        """Get the current username."""
        return get_username() or _os.environ.get("USER", _os.environ.get("USERNAME", "unknown"))

    @cached_property
    def hostname(self) -> str:
        """Get the current hostname.

        Returns:
            The machine hostname, eg ``somesvr``.
        """
        return self._native.hostname or get_hostname() or _stdlib_platform.node()

    @cached_property
    def home(self) -> str:
        """Get the home directory for the current user.

        Returns:
            The home directory path (e.g. ``/home/user`` or ``C:\\Users\\user``).
        """
        return _os.path.expanduser("~")

    @cached_property
    def fqdn(self) -> str:
        """Get the fully qualified domain name.

        Returns:
            The FQDN of the current machine,
            eg ``somesvr.somestudio.com``.
        """
        import socket as _socket

        return _socket.getfqdn()

    @cached_property
    def domain(self) -> str:
        """Get the domain.

        Returns:
            The domain, eg ``somestudio.com``.
        """
        try:
            return self.fqdn.split(".", 1)[1]
        except IndexError:
            return ""

    @cached_property
    def num_cpus(self) -> int:
        """Number of logical CPUs (may include hyper-threads)."""
        return self._native.num_cpus or _os.cpu_count() or 1

    @cached_property
    def python_version(self) -> str:
        """Full Python version string (e.g. 3.12.10 (...))."""
        return self._native.python_version or _stdlib_platform.python_version()

    # ── Rez Environment Properties ───────────────────────────────────────

    @property
    def env(self) -> str | None:
        """Get the current Rez environment name.

        Returns the value of ``REZ_ENV`` environment variable, or ``None``.
        """
        return _os.environ.get("REZ_ENV")

    @property
    def env_key(self) -> str:
        """Get the environment variable key used for the current env.

        Returns:
            ``"REZ_ENV"`` (always, for API compatibility).
        """
        return "REZ_ENV"

    @cached_property
    def rez_bin_path(self) -> str | None:
        """Get the path to the rez binary, if in a production install.

        Returns:
            The path to the ``rez`` binary directory, or ``None``.
        """
        import rez_next  # noqa: F811 — lazy import to avoid circular import

        bin_dir = _os.path.join(
            _os.path.dirname(_os.path.abspath(rez_next.__file__)),
            "..",
            "..",
            "Scripts" if self.platform == "windows" else "bin",
        )
        bin_dir = _os.path.realpath(_os.path.normpath(bin_dir))
        # Check if rez binary exists
        exe_name = "rez" + (".exe" if self.platform == "windows" else "")
        if _os.path.isfile(_os.path.join(bin_dir, exe_name)):
            return bin_dir
        # Also check if rez can be found via which
        exe = _which("rez")
        if exe:
            return _os.path.dirname(exe)
        return None

    @property
    def is_production_rez_install(self) -> bool:
        """Return ``True`` if this is a production rez install."""
        return bool(self.rez_bin_path)

    @property
    def selftest_is_running(self) -> bool:
        """Return ``True`` if tests are running via rez-selftest."""
        return _os.environ.get("__REZ_SELFTEST_RUNNING") == "1"

    # ── Public Methods ───────────────────────────────────────────────────

    def get_summary_string(self) -> str:
        """Get a string summarising the state of Rez as a whole.

        Includes the plugin manager summary.
        """
        from rez_next.plugin_managers import plugin_manager

        txt = f"Rez-Next {self.rez_version}"
        txt += f"\n\n{plugin_manager.get_summary_string()}"
        return txt

    def clear_caches(self, hard: bool = False) -> None:
        """Clear package repository caches."""
        del hard
        from rez_next.package_repository import package_repository_manager

        package_repository_manager.clear_caches()

    def which(self, arg: str) -> str | None:
        """Find an executable in the system PATH.

        Args:
            arg: Name of the executable to find.

        Returns:
            Full path to the executable, or ``None`` if not found.
        """
        return _which(arg)

    # ── Internal Helpers ─────────────────────────────────────────────────

    @classmethod
    def _make_safe_version_string(cls, s: str) -> str:
        """Convert a raw platform/arch string to a Rez-safe version string.

        Rez-safe strings only contain alphanumeric characters, underscores,
        dots, and hyphens.
        """
        sep_regex = re.compile(r"[.\-]")
        char_regex = re.compile(r"[a-zA-Z0-9_]")
        s = s.strip(".").strip("-")
        toks = sep_regex.split(s)
        seps = sep_regex.findall(s)
        valid_toks: list[str] = []
        expects_tok = True
        while toks or seps:
            if expects_tok:
                tok = toks[0]
                toks = toks[1:]
                if tok:
                    valid_tok = "".join(ch if char_regex.match(ch) else "_" for ch in tok)
                    valid_toks.append(valid_tok)
                else:
                    seps = seps[1:]  # skip empty
                expects_tok = False
            else:
                sep = seps[0]
                seps = seps[1:]
                valid_toks.append(sep)
                expects_tok = True
        return "".join(valid_toks)

    @staticmethod
    def _detect_os() -> str:
        """Fallback OS detection when native provides no value."""
        system_name = _stdlib_platform.system().lower()
        if system_name == "windows":
            try:
                ver, sp, _, _ = _stdlib_platform.win32_ver()
                if sp:
                    return f"windows-{ver}.{sp}"
                return f"windows-{ver}" if ver else "windows"
            except Exception:  # noqa: BLE001
                return "windows"
        elif system_name == "linux":
            return _stdlib_platform.platform(terse=True)
        elif system_name == "darwin":
            ver = _stdlib_platform.mac_ver()[0]
            return f"osx-{ver}" if ver else "osx"
        return system_name


# ── Factory functions ────────────────────────────────────────────────────────

_system_instance: System | None = None


def create_system(native: _NativeSystem | None = None) -> System:
    """Create a new ``System`` instance.

    Args:
        native: Optional native ``System`` instance for testing.

    Returns:
        A new ``System`` instance.
    """
    return System(native=native)


def get_system(native: _NativeSystem | None = None) -> System:
    """Get or create a ``System`` singleton.

    This function returns a cached singleton instance. If called with
    a *native* argument, a new instance is created.

    Args:
        native: Optional native ``System`` instance for testing.

    Returns:
        A ``System`` instance (singleton).
    """
    global _system_instance
    if _system_instance is None or native is not None:
        _system_instance = System(native=native)
    return _system_instance


# ── Global singleton ─────────────────────────────────────────────────────────

system: System = get_system()
"""Global ``System`` singleton matching the running OS & platform."""

# Module-level convenience exports (matching ``rez.system`` API)
platform: str = system.platform  # noqa: A001 — API compat with upstream
arch: str = system.arch
os: str = system.os  # noqa: A001 — API compat
user: str = system.user
hostname: str = system.hostname
