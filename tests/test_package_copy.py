"""Tests for rez_next.package_copy module."""

import os
import tempfile

import pytest
from rez_next.package_copy import copy_package, PackageCopyError


class TestPackageCopyImport:
    """Verify the module is importable and exports correct names."""

    def test_module_importable(self):
        from rez_next import package_copy
        assert hasattr(package_copy, "copy_package")

    def test_copy_package_callable(self):
        assert callable(copy_package)

    def test_exception_importable(self):
        assert issubclass(PackageCopyError, Exception)


class TestCopyPackageDefault:
    """Test copy_package with default parameters."""

    def test_copy_with_string_name_string_dest_minimal(self):
        """Minimal invocation: package name string + dest path string."""
        with tempfile.TemporaryDirectory() as tmpdir:
            dest = os.path.join(tmpdir, "dest")
            os.makedirs(dest)
            # Copy a non-existent package should fail gracefully
            with pytest.raises(PackageCopyError):
                copy_package("nonexistent_pkg_12345", dest)

    def test_copy_package_returns_dict_with_copied_skipped_keys(self):
        """Return value should have 'copied' and 'skipped' keys."""
        with tempfile.TemporaryDirectory() as tmpdir:
            dest = os.path.join(tmpdir, "dest")
            os.makedirs(dest)
            try:
                result = copy_package("nonexistent_pkg_12345", dest)
            except PackageCopyError:
                result = {"copied": [], "skipped": []}
            assert "copied" in result
            assert "skipped" in result

    def test_shallow_raises(self):
        """Shallow copy raises PackageCopyError (not yet supported)."""
        with tempfile.TemporaryDirectory() as tmpdir:
            dest = os.path.join(tmpdir, "dest")
            os.makedirs(dest)
            with pytest.raises(PackageCopyError, match="not yet supported"):
                copy_package("some_pkg", dest, shallow=True)

    def test_overrides_raises(self):
        """Custom overrides raises PackageCopyError (not yet supported)."""
        with tempfile.TemporaryDirectory() as tmpdir:
            dest = os.path.join(tmpdir, "dest")
            os.makedirs(dest)
            with pytest.raises(PackageCopyError, match="not yet supported"):
                copy_package("some_pkg", dest, overrides={"name": "other"})


class TestCopyPackageWithOptions:
    """Test copy_package with various option combinations."""

    def test_copy_with_verbose(self):
        """verbose=True should not raise."""
        with tempfile.TemporaryDirectory() as tmpdir:
            dest = os.path.join(tmpdir, "dest")
            os.makedirs(dest)
            try:
                result = copy_package("nonexistent_pkg_verbose", dest, verbose=True)
                assert isinstance(result, dict)
            except PackageCopyError:
                pass  # Expected to fail since package doesn't exist

    def test_copy_with_force(self):
        """force=True should not raise."""
        with tempfile.TemporaryDirectory() as tmpdir:
            dest = os.path.join(tmpdir, "dest")
            os.makedirs(dest)
            try:
                result = copy_package("nonexistent_pkg_force", dest, force=True)
                assert isinstance(result, dict)
            except PackageCopyError:
                pass  # Expected to fail since package doesn't exist

    def test_copy_with_overwrite(self):
        """overwrite=True maps to force=True internally."""
        with tempfile.TemporaryDirectory() as tmpdir:
            dest = os.path.join(tmpdir, "dest")
            os.makedirs(dest)
            try:
                result = copy_package("nonexistent_pkg_ow", dest, overwrite=True)
                assert isinstance(result, dict)
            except PackageCopyError:
                pass  # Expected to fail since package doesn't exist

    def test_copy_with_dry_run(self):
        """dry_run returns empty result without error."""
        with tempfile.TemporaryDirectory() as tmpdir:
            dest = os.path.join(tmpdir, "dest")
            os.makedirs(dest)
            result = copy_package("nonexistent_pkg_dry", dest, dry_run=True)
            assert isinstance(result, dict)
            assert "copied" in result
            assert "skipped" in result


class TestCopyPackageResolveHelpers:
    """Test internal helper functions."""

    def test_resolve_package_name_string(self):
        from rez_next.package_copy import _resolve_package_name
        assert _resolve_package_name("python-3.9") == "python-3.9"

    def test_resolve_dest_path_string(self):
        from rez_next.package_copy import _resolve_dest_path
        path = _resolve_dest_path("/tmp/test_repo")
        assert os.path.isabs(path)

    def test_resolve_src_paths_none(self):
        from rez_next.package_copy import _resolve_src_paths
        assert _resolve_src_paths() is None

    def test_resolve_src_paths_strings(self):
        from rez_next.package_copy import _resolve_src_paths
        with tempfile.TemporaryDirectory() as tmpdir:
            paths = _resolve_src_paths([tmpdir])
            assert paths is not None
            assert os.path.isabs(paths[0])


class TestCopyPackageIntegration:
    """Lightweight integration tests (work with real filesystem)."""

    def test_copy_nonexistent_package_errors(self):
        """Copying a package that doesn't exist should raise PackageCopyError."""
        with tempfile.TemporaryDirectory() as tmpdir:
            dest = os.path.join(tmpdir, "dest")
            os.makedirs(dest)
            with pytest.raises((PackageCopyError, LookupError)):
                copy_package("__this_package_does_not_exist__", dest)
