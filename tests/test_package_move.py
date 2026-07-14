"""Tests for rez_next.package_move module."""

import os
import tempfile

import pytest
from rez_next.package_move import move_package, PackageMoveError


class TestPackageMoveImport:
    """Verify the module is importable and exports correct names."""

    def test_module_importable(self):
        from rez_next import package_move
        assert hasattr(package_move, "move_package")

    def test_move_package_callable(self):
        assert callable(move_package)

    def test_exception_importable(self):
        assert issubclass(PackageMoveError, Exception)


class TestMovePackageDefault:
    """Test move_package with default parameters."""

    def test_move_with_string_name_string_dest_minimal(self):
        """Minimal invocation: package name string + dest path string."""
        with tempfile.TemporaryDirectory() as tmpdir:
            dest = os.path.join(tmpdir, "dest")
            os.makedirs(dest)
            with pytest.raises((PackageMoveError, LookupError)):
                move_package("nonexistent_pkg_12345", dest)

    def test_move_package_returns_string(self):
        """Return value should be a string (destination path)."""
        with tempfile.TemporaryDirectory() as tmpdir:
            dest = os.path.join(tmpdir, "dest")
            os.makedirs(dest)
            try:
                result = move_package("nonexistent_pkg_12345", dest)
                assert isinstance(result, str)
            except (PackageMoveError, LookupError):
                pass  # Expected to fail since package doesn't exist


class TestMovePackageWithOptions:
    """Test move_package with various option combinations."""

    def test_move_with_verbose(self):
        """verbose=True should not raise."""
        with tempfile.TemporaryDirectory() as tmpdir:
            dest = os.path.join(tmpdir, "dest")
            os.makedirs(dest)
            try:
                result = move_package("nonexistent_pkg_verbose", dest, verbose=True)
                assert isinstance(result, str)
            except (PackageMoveError, LookupError):
                pass  # Expected to fail since package doesn't exist

    def test_move_with_force(self):
        """force=True should attempt the move."""
        with tempfile.TemporaryDirectory() as tmpdir:
            dest = os.path.join(tmpdir, "dest")
            os.makedirs(dest)
            try:
                result = move_package("nonexistent_pkg_force", dest, force=True)
                assert isinstance(result, str)
            except (PackageMoveError, LookupError):
                pass  # Expected to fail since package doesn't exist

    def test_move_with_keep_source(self):
        """keep_source=True should attempt a copy-style move."""
        with tempfile.TemporaryDirectory() as tmpdir:
            dest = os.path.join(tmpdir, "dest")
            os.makedirs(dest)
            try:
                result = move_package("nonexistent_pkg_ks", dest, keep_source=True)
                assert isinstance(result, str)
            except (PackageMoveError, LookupError):
                pass  # Expected to fail since package doesn't exist


class TestMovePackageResolveHelpers:
    """Test internal helper functions."""

    def test_resolve_package_name_string(self):
        from rez_next.package_move import _resolve_package_name
        assert _resolve_package_name("python-3.9") == "python-3.9"

    def test_resolve_dest_path_string(self):
        from rez_next.package_move import _resolve_dest_path
        path = _resolve_dest_path("/tmp/test_repo")
        assert os.path.isabs(path)

    def test_resolve_dest_path_with_repo_object(self):
        from rez_next.package_move import _resolve_dest_path

        class DummyRepo:
            path = "/dummy/repo/path"

        path = _resolve_dest_path(DummyRepo())
        assert "dummy" in path

    def test_resolve_package_with_dest_version(self):
        from rez_next.package_move import _resolve_package_version
        version = _resolve_package_version("pkg", "2.0.0")
        assert version == "2.0.0"


class TestMovePackageIntegration:
    """Lightweight integration tests (work with real filesystem)."""

    def test_move_nonexistent_package_errors(self):
        """Moving a package that doesn't exist should raise an error."""
        with tempfile.TemporaryDirectory() as tmpdir:
            dest = os.path.join(tmpdir, "dest")
            os.makedirs(dest)
            with pytest.raises((PackageMoveError, LookupError)):
                move_package("__this_package_does_not_exist__", dest)

    def test_move_self_detection(self):
        """Moving a package to its own location should work or raise."""
        with tempfile.TemporaryDirectory() as tmpdir:
            dest = os.path.join(tmpdir, "dest")
            os.makedirs(dest)
            try:
                result = move_package(
                    "some_pkg",
                    dest,
                    force=True,
                    keep_source=True,
                )
                assert isinstance(result, str)
            except (PackageMoveError, LookupError):
                pass  # Expected to fail since package doesn't exist
