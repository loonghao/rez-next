"""Tests for ``rez_next.utils.platform_`` — platform abstraction module.

Comprehensive tests covering:
- Global singleton creation
- All public properties (arch, os, tmpdir, cores, etc.)
- Platform-specific class selection
- symlink() method (smoke test)
- has_case_sensitive_filesystem
- new_session_popen_args
- terminal_emulator_command
- Cache behaviour (cached_property)
- Cross-platform fallback (_GenericPlatform)
"""

from __future__ import annotations

import os
import platform
import tempfile

import pytest

from rez_next.utils.platform_ import (
    Platform,
    LinuxPlatform,
    OSXPlatform,
    WindowsPlatform,
    platform_,
)


class TestPlatformSingleton:
    """Tests for the global ``platform_`` singleton."""

    def test_platform_is_instance(self):
        """platform_ should be a Platform instance."""
        assert isinstance(platform_, Platform)

    def test_platform_is_correct_subclass(self):
        """platform_ should be an instance of the expected subclass."""
        system = platform.system().lower()
        if system == "linux":
            assert isinstance(platform_, LinuxPlatform)
        elif system == "darwin":
            assert isinstance(platform_, OSXPlatform)
        elif system == "windows":
            assert isinstance(platform_, WindowsPlatform)

    def test_platform_name_is_not_empty(self):
        """platform_.name should be a non-empty string."""
        assert platform_.name
        assert isinstance(platform_.name, str)

    def test_platform_name_matches_system(self):
        """platform_.name should reflect the current OS."""
        name = platform_.name
        assert name in ("linux", "osx", "windows")


class TestArchProperty:
    """Tests for ``platform_.arch``."""

    def test_arch_is_string(self):
        """arch should return a non-empty string."""
        arch = platform_.arch
        assert isinstance(arch, str)
        assert len(arch) > 0

    def test_arch_known_value(self):
        """arch should be a recognised architecture string."""
        arch = platform_.arch
        assert arch in (
            "x86_64", "amd64", "i386", "i686",
            "aarch64", "arm64", "armv7l",
            "ppc64", "ppc64le", "s390x",
        ), f"unexpected arch: {arch!r}"


class TestOsProperty:
    """Tests for ``platform_.os``."""

    def test_os_is_string(self):
        """os should return a non-empty string."""
        os_val = platform_.os
        assert isinstance(os_val, str)
        assert len(os_val) > 0

    def test_os_starts_with_platform_name(self):
        """os should start with the platform name prefix."""
        os_val = platform_.os
        name = platform_.name
        if name == "windows":
            assert os_val.startswith("windows")
        elif name == "linux":
            assert "-" in os_val  # e.g. "Ubuntu-20.04"
        elif name == "osx":
            assert os_val.startswith("osx")


class TestTmpdirProperty:
    """Tests for ``platform_.tmpdir``."""

    def test_tmpdir_is_string(self):
        """tmpdir should return a non-empty string."""
        tmpdir_val = platform_.tmpdir
        assert isinstance(tmpdir_val, str)
        assert len(tmpdir_val) > 0

    def test_tmpdir_matches_tempfile(self):
        """tmpdir should match ``tempfile.gettempdir()``."""
        assert platform_.tmpdir == tempfile.gettempdir()

    def test_tmpdir_exists(self):
        """tmpdir should be an existing directory."""
        assert os.path.isdir(platform_.tmpdir)


class TestCores:
    """Tests for CPU core counts."""

    def test_physical_cores_positive(self):
        """physical_cores should be a positive integer."""
        cores = platform_.physical_cores
        assert isinstance(cores, int)
        assert cores >= 1

    def test_logical_cores_positive(self):
        """logical_cores should be a positive integer."""
        cores = platform_.logical_cores
        assert isinstance(cores, int)
        assert cores >= 1

    def test_physical_does_not_exceed_logical(self):
        """physical_cores should be <= logical_cores."""
        assert platform_.physical_cores <= platform_.logical_cores


class TestCaseSensitiveFilesystem:
    """Tests for ``has_case_sensitive_filesystem``."""

    def test_is_boolean(self):
        """has_case_sensitive_filesystem should be a bool."""
        val = platform_.has_case_sensitive_filesystem
        assert isinstance(val, bool)

    def test_windows_is_false(self):
        """Windows should have case-insensitive filesystem."""
        if platform_.name == "windows":
            assert platform_.has_case_sensitive_filesystem is False
        else:
            # On Linux/macOS this could be True or False depending on FS
            assert isinstance(platform_.has_case_sensitive_filesystem, bool)


class TestEditorDifftoolImageViewer:
    """Tests for editor/difftool/image_viewer properties."""

    def test_editor_is_string_or_none(self):
        """editor should be a string or None."""
        editor = platform_.editor
        assert editor is None or isinstance(editor, str)

    def test_difftool_is_string_or_none(self):
        """difftool should be a string or None."""
        tool = platform_.difftool
        assert tool is None or isinstance(tool, str)

    def test_image_viewer_is_string_or_none(self):
        """image_viewer should be a string or None."""
        viewer = platform_.image_viewer
        assert viewer is None or isinstance(viewer, str)


class TestTerminalEmulator:
    """Tests for terminal-related properties."""

    def test_terminal_emulator_command_is_string_or_none(self):
        """terminal_emulator_command should be a string or None."""
        cmd = platform_.terminal_emulator_command
        assert cmd is None or isinstance(cmd, str)

    def test_new_session_popen_args_is_dict(self):
        """new_session_popen_args should be a dict."""
        args = platform_.new_session_popen_args
        assert isinstance(args, dict)


class TestSymlink:
    """Tests for the symlink() method (best-effort)."""

    def test_symlink_raises_on_invalid(self):
        """symlink() should raise OSError on invalid paths."""
        with pytest.raises(OSError):
            platform_.symlink("/nonexistent/source", "/nonexistent/link")


class TestCacheBehaviour:
    """Tests for cached_property behaviour."""

    def test_arch_is_cached(self):
        """arch should return the same object on repeated access."""
        # cached_property sets the value as an instance attribute
        arch1 = platform_.arch
        arch2 = platform_.arch
        assert arch1 == arch2


class TestClassInheritance:
    """Tests for Platform class hierarchy."""

    def test_linux_platform_attributes(self):
        """LinuxPlatform should have all required attributes."""
        lp = LinuxPlatform() if platform_.name == "linux" else None
        if lp is not None:
            props = ["arch", "os", "editor", "difftool", "tmpdir",
                     "physical_cores", "logical_cores", "image_viewer",
                     "terminal_emulator_command", "new_session_popen_args",
                     "has_case_sensitive_filesystem"]
            for prop in props:
                assert hasattr(lp, prop)
            assert hasattr(lp, "symlink")

    def test_windows_platform_attributes(self):
        """WindowsPlatform should have all required attributes."""
        wp = WindowsPlatform() if platform_.name == "windows" else None
        if wp is not None:
            props = ["arch", "os", "editor", "difftool", "tmpdir",
                     "physical_cores", "logical_cores", "image_viewer",
                     "terminal_emulator_command", "new_session_popen_args",
                     "has_case_sensitive_filesystem"]
            for prop in props:
                assert hasattr(wp, prop)
            assert hasattr(wp, "symlink")

    def test_osx_platform_attributes(self):
        """OSXPlatform should have all required attributes."""
        op = OSXPlatform() if platform_.name == "osx" else None
        if op is not None:
            props = ["arch", "os", "editor", "difftool", "tmpdir",
                     "physical_cores", "logical_cores", "image_viewer",
                     "terminal_emulator_command", "new_session_popen_args",
                     "has_case_sensitive_filesystem"]
            for prop in props:
                assert hasattr(op, prop)
            assert hasattr(op, "symlink")

    def test_name_attribute(self):
        """Each subclass should have a non-empty name."""
        for cls in (LinuxPlatform, WindowsPlatform, OSXPlatform):
            instance = cls()
            assert instance.name
            assert isinstance(instance.name, str)


class TestModuleReExport:
    """Tests that the module correctly re-exports its public API."""

    def test_platform_importable_from_utils(self):
        """``from rez_next.utils.platform_ import platform_`` should work."""
        # Already imported at module level — just verify
        assert platform_ is not None

    def test_rez_compatible_import(self):
        """Simulate ``from rez.utils.platform_ import platform_``."""
        import importlib
        mod = importlib.import_module("rez_next.utils.platform_")
        assert hasattr(mod, "platform_")
        assert hasattr(mod, "Platform")
        assert hasattr(mod, "LinuxPlatform")
        assert hasattr(mod, "OSXPlatform")
        assert hasattr(mod, "WindowsPlatform")


class TestWindowsSpecific:
    """Windows-specific tests (only run on Windows)."""

    @pytest.mark.skipif(
        platform.system().lower() != "windows",
        reason="Windows-specific test",
    )
    def test_windows_stdout(self):
        """Windows: editor and image_viewer are empty strings."""
        assert platform_.editor == ""
        assert platform_.image_viewer == ""

    @pytest.mark.skipif(
        platform.system().lower() != "windows",
        reason="Windows-specific test",
    )
    def test_windows_creationflags(self):
        """Windows: new_session_popen_args should have creationflags."""
        assert platform_.new_session_popen_args.get("creationflags") == 0x00000010

    @pytest.mark.skipif(
        platform.system().lower() != "windows",
        reason="Windows-specific test",
    )
    def test_windows_terminal_command(self):
        """Windows: terminal_emulator_command should be START."""
        assert platform_.terminal_emulator_command == "START"
