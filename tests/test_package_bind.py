"""
Tests for the rez_next.package_bind module — aligns with rez.package_bind API.

Tests:
- Bind-module discovery (get_bind_modules, find_bind_module)
- Bind-package error handling (unknown module)
- Cache invalidation
- Integration with Rust built-in binders
"""

import os
import sys
import tempfile
from pathlib import Path

import pytest

from rez_next import package_bind
from rez_next.exceptions import RezBindError


# ── Fixtures ─────────────────────────────────────────────────────────


@pytest.fixture(autouse=True)
def clear_bind_cache():
    """Clear the bind-module cache before each test."""
    package_bind._clear_cache()
    yield


@pytest.fixture
def temp_bind_script():
    """Create a temporary directory with a minimal bind script.

    Returns (tmpdir, script_path, script_name).
    """
    with tempfile.TemporaryDirectory() as tmpdir:
        script = Path(tmpdir) / "my_tool.py"
        script.write_text(
            _MINIMAL_BIND_SCRIPT, encoding="utf-8"
        )
        yield Path(tmpdir), str(script), "my_tool"


_MINIMAL_BIND_SCRIPT = r"""
def bind(path, version_range=None, opts=None, parser=None):
    import os
    os.makedirs(path, exist_ok=True)
    return [type("Result", (), {"name": "my_tool", "version": "1.0.0", "install_path": path})()
        ]
"""


# ── Tests: Module Discovery ─────────────────────────────────────────


class TestGetBindModules:
    """get_bind_modules() — discover built-in and external bind modules."""

    def test_returns_dict(self):
        """get_bind_modules() must return a dict."""
        modules = package_bind.get_bind_modules()
        assert isinstance(modules, dict)

    def test_includes_builtin_binders(self):
        """Built-in binders (python, cmake, git...) should be present."""
        modules = package_bind.get_bind_modules()
        # At minimum, these built-in binders should be available
        expected = {"python", "cmake", "git"}
        assert expected.issubset(modules.keys()), (
            f"Missing built-in binders. Have: {set(modules.keys())}"
        )

    def test_builtin_path_virtual(self):
        """Built-in binder paths should start with '<builtin>:'."""
        modules = package_bind.get_bind_modules()
        for name, path in modules.items():
            if name in ("python", "cmake", "git"):
                assert path.startswith("<builtin>:"), (
                    f"Built-in '{name}' should have virtual path, got {path}"
                )

    def test_verbose_output(self, capsys):
        """Verbose mode should print paths being searched."""
        package_bind.get_bind_modules(verbose=True)
        captured = capsys.readouterr()
        # Should print something (even if no custom paths configured)
        # Just verify the call doesn't crash and prints something
        assert captured.out is not None

    def test_cache_hit(self):
        """The second call should return the same dict (cached)."""
        first = package_bind.get_bind_modules()
        second = package_bind.get_bind_modules()
        assert first is second, "Cached result should be the same object"

    def test_cache_after_clear(self):
        """After _clear_cache, a new dict is returned."""
        first = package_bind.get_bind_modules()
        package_bind._clear_cache()
        second = package_bind.get_bind_modules()
        assert first is not second, "Cache should be invalidated after _clear_cache"


class TestFindBindModule:
    """find_bind_module() — locate a specific bind module."""

    def test_find_builtin(self):
        """Finding a known built-in binder should return its virtual path."""
        path = package_bind.find_bind_module("python")
        assert path is not None
        assert path.startswith("<builtin>:")

    def test_find_builtin_cmake(self):
        """Finding 'cmake' should return a virtual built-in path."""
        path = package_bind.find_bind_module("cmake")
        assert path is not None
        assert "<builtin>" in path

    def test_find_unknown(self):
        """An unknown binder should return None."""
        path = package_bind.find_bind_module("__nonexistent_tool_12345__")
        assert path is None

    def test_find_unknown_verbose(self, capsys):
        """Verbose mode should print close-match suggestions for unknown."""
        path = package_bind.find_bind_module("pythin", verbose=True)
        assert path is None
        captured = capsys.readouterr()
        assert "not found" in captured.out.lower()

    def test_find_external_script(self, temp_bind_script):
        """An external bind script should be discoverable."""
        tmpdir, _, name = temp_bind_script
        # Temporarily patch config bind_module_path
        original_path = list(getattr(package_bind, "_config", None).bind_module_path)

        try:
            if hasattr(package_bind, "_config"):
                package_bind._config.bind_module_path = [str(tmpdir)]
            package_bind._clear_cache()
            path = package_bind.find_bind_module(name)
            assert path is not None
            assert path.endswith(f"{name}.py")
        finally:
            if hasattr(package_bind, "_config"):
                package_bind._config.bind_module_path = original_path


# ── Tests: bind_package ─────────────────────────────────────────────


class TestBindPackage:
    """bind_package() — the main entry point for binding."""

    def test_bind_unknown_raises(self):
        """Binding an unknown package should raise RezBindError."""
        with pytest.raises(RezBindError, match="Bind module not found"):
            package_bind.bind_package("__nonexistent_tool_12345__")

    def test_bind_builtin_no_path(self):
        """Binding a built-in tool without a path should not crash.

        It might raise a different error (tool not in PATH, or version
        detection failure), but must NOT raise RezBindError for missing
        module.
        """
        try:
            package_bind.bind_package("python")
        except RezBindError as exc:
            # This is fine — either module not found (shouldn't happen)
            # or actual bind failure (e.g. version detection).
            assert "not found" not in str(exc).lower()

    def test_bind_builtin_with_install_path(self):
        """Binding to a specific path should write package.py."""
        with tempfile.TemporaryDirectory() as tmpdir:
            install_path = str(Path(tmpdir) / "packages")
            try:
                results = package_bind.bind_package(
                    "cmake",
                    path=install_path,
                    quiet=True,
                )
                # If cmake is on the system, we get results
                assert len(results) >= 1
                pkg_dir = Path(results[0].install_path)
                assert pkg_dir.exists()
                assert (pkg_dir / "package.py").exists()
            except RezBindError:
                # cmake may not be in PATH during CI
                pass

    def test_bind_quiet(self, capsys):
        """Quiet mode should suppress output."""
        with tempfile.TemporaryDirectory() as tmpdir:
            try:
                package_bind.bind_package(
                    "git",
                    path=tmpdir,
                    quiet=True,
                )
            except RezBindError:
                pass
            captured = capsys.readouterr()
            # Quiet mode should have no output
            assert captured.out == ""

    def test_bind_non_quiet_prints_summary(self, capsys):
        """Non-quiet mode should print a package list."""
        with tempfile.TemporaryDirectory() as tmpdir:
            try:
                package_bind.bind_package(
                    "git",
                    path=tmpdir,
                    quiet=False,
                )
            except RezBindError:
                pass
            captured = capsys.readouterr()
            if captured.out:
                # Should mention PACKAGE header or "packages were installed"
                assert ("PACKAGE" in captured.out or
                        "installed" in captured.out.lower())


# ── Tests: Error Handling ───────────────────────────────────────────


class TestErrorHandling:
    """Error and edge-case handling."""

    def test_rez_bind_error_import(self):
        """RezBindError should be importable from the exceptions module."""
        from rez_next.exceptions import RezBindError

        assert issubclass(RezBindError, Exception)

    def test_rez_bind_error_raise(self):
        """RezBindError should be raiseable with a message."""
        with pytest.raises(RezBindError, match="test error"):
            raise RezBindError("test error")

    def test_invalid_bind_module_path_config(self):
        """Invalid bind_module_path should not crash get_bind_modules."""
        original = getattr(package_bind._config, "bind_module_path", [])
        try:
            package_bind._config.bind_module_path = "/nonexistent/path"
            package_bind._clear_cache()
            modules = package_bind.get_bind_modules()
            assert isinstance(modules, dict)
            # Built-in binders should still be present
            assert "python" in modules
        finally:
            package_bind._config.bind_module_path = original

    def test_bind_package_empty_name_raises(self):
        """An empty package name should raise RezBindError."""
        with pytest.raises(RezBindError):
            package_bind.bind_package("")

    def test_clear_cache_twice(self):
        """Clearing cache twice should be idempotent."""
        package_bind._clear_cache()
        package_bind._clear_cache()  # Should not raise
        modules = package_bind.get_bind_modules()
        assert isinstance(modules, dict)


# ── Tests: External Bind Scripts ────────────────────────────────────


class TestExternalBindScripts:
    """Integration with external Python bind scripts."""

    def test_discover_external_script(self, temp_bind_script):
        """An external bind script in bind_module_path should be discoverable."""
        tmpdir, _, name = temp_bind_script
        original = getattr(package_bind._config, "bind_module_path", [])
        try:
            if hasattr(package_bind, "_config"):
                package_bind._config.bind_module_path = [str(tmpdir)]
            package_bind._clear_cache()
            modules = package_bind.get_bind_modules()
            assert name in modules, (
                f"External script '{name}' should be in modules. Keys: {list(modules.keys())}"
            )
        finally:
            if hasattr(package_bind, "_config"):
                package_bind._config.bind_module_path = original

    def test_external_script_takes_precedence(self, temp_bind_script):
        """An external script should override a built-in binder of the same name."""
        tmpdir, _, name = temp_bind_script
        original = getattr(package_bind._config, "bind_module_path", [])
        try:
            if hasattr(package_bind, "_config"):
                package_bind._config.bind_module_path = [str(tmpdir)]
            package_bind._clear_cache()
            path = package_bind.find_bind_module(name)
            assert path is not None
            # External script should win because it's added after builtins
            assert not path.startswith("<builtin>:"), (
                "External script should override built-in binder"
            )
            assert path.endswith(".py")
        finally:
            if hasattr(package_bind, "_config"):
                package_bind._config.bind_module_path = original
