"""Tests for the build_process module."""

import os
import sys
import abc
import pytest
from unittest.mock import MagicMock, patch, PropertyMock

# Ensure rez_next is importable
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "crates", "rez-next-python", "python"))

from rez_next.build_process import (
    BuildType,
    BuildProcessError,
    BuildContextResolveError,
    ReleaseError,
    ReleaseHookCancellingError,
    ReleaseVCSError,
    BuildProcess,
    BuildProcessHelper,
    get_build_process_types,
    create_build_process,
    _remove_readonly,
    _retry_rmtree,
)


# ==============================================================================
# TestBuildType
# ==============================================================================

class TestBuildType:
    def test_local_value(self):
        assert BuildType.local == 0
        assert BuildType.local.name == "local"

    def test_central_value(self):
        assert BuildType.central == 1
        assert BuildType.central.name == "central"

    def test_int_enum(self):
        assert int(BuildType.local) == 0
        assert isinstance(BuildType.local, int)


# ==============================================================================
# TestErrorHierarchy
# ==============================================================================

class TestErrorHierarchy:
    def test_build_process_error(self):
        err = BuildProcessError("test")
        assert isinstance(err, Exception)
        assert str(err) == "test"

    def test_build_context_resolve_error(self):
        err = BuildContextResolveError("resolve failed")
        assert isinstance(err, BuildProcessError)
        assert str(err) == "resolve failed"

    def test_release_error(self):
        err = ReleaseError("release failed")
        assert isinstance(err, BuildProcessError)
        assert str(err) == "release failed"

    def test_release_hook_cancelling_error(self):
        err = ReleaseHookCancellingError("cancelled")
        assert isinstance(err, BuildProcessError)
        assert str(err) == "cancelled"

    def test_release_vcs_error_with_cause(self):
        cause = ValueError("underlying")
        err = ReleaseVCSError("vcs failed", cause=cause)
        assert isinstance(err, BuildProcessError)
        assert str(err) == "vcs failed"
        assert err.cause is cause

    def test_release_vcs_error_without_cause(self):
        err = ReleaseVCSError("vcs failed")
        assert err.cause is None


# ==============================================================================
# TestBuildProcessAbstract
# ==============================================================================

class TestBuildProcessAbstract:
    def test_cannot_instantiate_abc(self):
        """BuildProcess has abstract methods and cannot be instantiated."""
        with pytest.raises(TypeError):
            BuildProcess()

    def test_concrete_subclass_registers(self):
        """Concrete subclasses auto-register in the registry."""

        class MyBuildProcess(BuildProcess):
            def build(self, **kwargs):
                return 1

            def release(self, **kwargs):
                return 1

        assert "MyBuildProcess" in BuildProcess._registry
        assert BuildProcess._registry["MyBuildProcess"] is MyBuildProcess

    def test_abstract_subclass_cannot_be_instantiated(self):
        """Abstract subclass cannot be instantiated."""

        class MidLevel(BuildProcess):
            @abc.abstractmethod
            def extra(self):
                pass

            def build(self, **kwargs):
                return 1

            def release(self, **kwargs):
                return 1

        with pytest.raises(TypeError):
            MidLevel()

    def test_concrete_subclass_working_dir_property(self):
        """working_dir property borrows from build_system."""

        class MyProc(BuildProcess):
            def build(self, **kwargs):
                return 1
            def release(self, **kwargs):
                return 1

        proc = MyProc()
        assert proc.working_dir is None
        assert proc.package is None

    def test_named_subclass(self):
        """Subclass with name attribute uses it as key."""
        class MyProc(BuildProcess):
            name = "custom_name"
            def build(self, **kwargs):
                return 1
            def release(self, **kwargs):
                return 1
        assert MyProc.name in BuildProcess._registry


# ==============================================================================
# TestBuildProcessConcrete
# ==============================================================================

class TestBuildProcessConcrete:
    """Test with a concrete subclass (not BuildProcessHelper)."""

    @pytest.fixture
    def proc(self):
        class SimpleBuildProcess(BuildProcess):
            def build(self, install_path=None, clean=False, install=False, variants=None):
                return 2
            def release(self, release_message=None, variants=None):
                return 1
        return SimpleBuildProcess()

    def test_print_quiet(self, proc):
        proc._quiet = True
        proc._print("should not print")  # no crash

    def test_print_not_quiet(self, proc, capsys):
        proc._quiet = False
        proc._print("hello")
        captured = capsys.readouterr()
        assert "hello" in captured.out

    def test_print_header(self, proc, capsys):
        proc._quiet = False
        proc._print_header("Test Title")
        captured = capsys.readouterr()
        assert "Test Title" in captured.out
        assert "====" in captured.out

    def test_n_of_m(self, proc):
        assert proc._n_of_m(1, 4) == "[1/4]"

    def test_get_changelog_no_vcs(self, proc):
        assert proc.get_changelog() is None

    def test_get_changelog_with_vcs(self, proc):
        vcs = MagicMock()
        vcs.get_changelog.return_value = "changelog content"
        proc._vcs = vcs
        assert proc.get_changelog() == "changelog content"
        vcs.get_changelog.assert_called_once()

    def test_get_changelog_with_max_revisions(self, proc):
        vcs = MagicMock()
        proc._vcs = vcs
        proc.get_changelog(max_revisions=50)
        vcs.get_changelog.assert_called_once_with(max_revisions=50)

    def test_package_property_from_build_system(self, proc):
        mock_pkg = MagicMock()
        mock_pkg.name = "test_pkg"
        mock_bs = MagicMock()
        mock_bs.package = mock_pkg
        proc._build_system = mock_bs
        assert proc.package is mock_pkg
        assert proc.package.name == "test_pkg"


# ==============================================================================
# TestBuildProcessHelper
# ==============================================================================

class TestBuildProcessHelper:
    @pytest.fixture
    def helper(self):
        """Create a BuildProcessHelper with mocks."""
        pkg = MagicMock()
        pkg.name = "test_pkg"
        pkg.version = "1.0.0"
        pkg.variants = [
            ["python-3.9", "maya-2024"],
            ["python-3.10", "maya-2025"],
        ]
        pkg.build_requires = ["cmake-3"]
        bs = MagicMock()
        bs.package = pkg
        bs.working_dir = "/tmp/work"
        vcs = MagicMock()
        vcs.status.return_value = "clean"
        vcs.tag_exists.return_value = False
        return BuildProcessHelper(build_system=bs, vcs=vcs)

    def test_package_property(self, helper):
        assert helper.package is not None
        assert helper.package.name == "test_pkg"

    def test_working_dir_property(self, helper):
        assert helper.working_dir == "/tmp/work"

    def test_repo_operation_no_error(self, helper):
        with helper.repo_operation("test"):
            pass  # No error should be raised

    def test_repo_operation_skip_error(self, helper):
        helper._skip_repo_errors = True
        with helper.repo_operation("test"):
            raise ReleaseVCSError("some vcs error")
        # Should not raise

    def test_repo_operation_raise_error(self, helper):
        helper._skip_repo_errors = False
        with pytest.raises(ReleaseVCSError):
            with helper.repo_operation("test"):
                raise ReleaseVCSError("some vcs error")

    def test_visit_variants_no_package(self):
        helper = BuildProcessHelper()
        count, results = helper.visit_variants(lambda i, r: i)
        assert count == 0
        assert results == []

    def test_visit_variants_no_variants(self):
        pkg = MagicMock()
        pkg.variants = None
        bs = MagicMock()
        bs.package = pkg
        helper = BuildProcessHelper(build_system=bs)
        count, results = helper.visit_variants(lambda i, r: f"res={i}")
        assert count == 1
        assert results == ["res=0"]

    def test_visit_variants_with_variants(self, helper):
        count, results = helper.visit_variants(lambda i, r: f"idx={i}")
        assert count == 2
        assert results == ["idx=0", "idx=1"]

    def test_visit_variants_subset(self, helper):
        count, results = helper.visit_variants(lambda i, r: f"idx={i}", variants=[1])
        assert count == 1
        assert results == ["idx=1"]

    def test_get_package_install_path(self, helper):
        result = helper.get_package_install_path("/repo")
        expected = os.path.join("/repo", "test_pkg", "1.0.0")
        assert result == expected

    def test_get_package_install_path_no_package(self):
        helper = BuildProcessHelper()
        assert helper.get_package_install_path("/repo") == "/repo"

    def test_pre_release_no_vcs(self):
        pkg = MagicMock()
        pkg.name = "test"
        pkg.version = "1.0"
        bs = MagicMock()
        bs.package = pkg
        helper = BuildProcessHelper(build_system=bs, vcs=None)
        with pytest.raises(ReleaseError, match="VCS is required"):
            helper.pre_release()

    def test_pre_release_tag_exists_raises(self, helper):
        helper._vcs.tag_exists.return_value = True
        helper._ignore_existing_tag = False
        with pytest.raises(ReleaseError, match="already exists"):
            helper.pre_release()

    def test_pre_release_tag_exists_ignored(self, helper):
        helper._vcs.tag_exists.return_value = True
        helper._ignore_existing_tag = True
        assert helper.pre_release() is True

    def test_pre_release_passes(self, helper):
        assert helper.pre_release() is True

    def test_post_release_creates_tag(self, helper):
        tag = helper.post_release(release_message="v1.0")
        assert tag == "v1.0.0"
        helper._vcs.create_tag.assert_called_once_with(
            "v1.0.0", message="v1.0"
        )

    def test_get_current_tag_name(self, helper):
        tag = helper.get_current_tag_name()
        assert tag == "v1.0.0"

    def test_get_changelog_delegates(self, helper):
        helper._vcs.get_changelog.return_value = "changelog"
        assert helper.get_changelog() == "changelog"

    def test_get_release_data(self, helper):
        helper._vcs.get_changelog.return_value = "log"
        data = helper.get_release_data()
        assert data["package_name"] == "test_pkg"
        assert data["package_version"] == "1.0.0"
        assert data["vcs_name"] is not None
        assert "changelog" in data

    def test_get_previous_release(self, helper):
        with patch("rez_next.packages_.get_latest_package") as mock_glp:
            mock_glp.return_value = MagicMock(version="0.9.0")
            prev = helper.get_previous_release()
            assert prev is not None

    def test_build_no_build_system(self):
        helper = BuildProcessHelper()
        with pytest.raises(BuildProcessError, match="build system"):
            helper.build()

    def test_release_no_build_system(self):
        helper = BuildProcessHelper()
        with pytest.raises(BuildProcessError, match="build system"):
            helper.release()

    def test_run_hooks_import_error(self, helper):
        """run_hooks gracefully handles missing release_hook module."""
        helper.run_hooks("pre_release")  # no crash

    def test_build_with_system(self, helper):
        helper._build_system.build.return_value = True
        count = helper.build()
        assert count == 2


# ==============================================================================
# TestFactoryFunctions
# ==============================================================================

class TestFactoryFunctions:
    def test_get_build_process_types(self):
        types = get_build_process_types()
        assert isinstance(types, dict)

    def test_create_unknown_type(self):
        with pytest.raises(BuildProcessError, match="Unknown"):
            create_build_process("nonexistent")

    def test_create_known_type(self):
        class MyProc(BuildProcess):
            name = "test_proc"
            def build(self, **kwargs):
                return 1
            def release(self, **kwargs):
                return 1
        proc = create_build_process("test_proc")
        assert isinstance(proc, BuildProcess)


# ==============================================================================
# TestPlatformHelpers
# ==============================================================================

class TestPlatformHelpers:
    def test_remove_readonly_normal(self, tmp_path):
        f = tmp_path / "test.txt"
        f.write_text("hello")
        _remove_readonly(os.unlink, str(f), None)
        assert not f.exists()

    def test_retry_rmtree_exists(self, tmp_path):
        d = tmp_path / "subdir"
        d.mkdir()
        (d / "file.txt").write_text("hello")
        _retry_rmtree(str(d))
        assert not d.exists()

    def test_retry_rmtree_nonexistent(self, tmp_path):
        p = str(tmp_path / "nonexistent")
        _retry_rmtree(p)  # no crash

    def test_retry_rmtree_with_error(self, tmp_path):
        d = tmp_path / "locked"
        d.mkdir()
        (d / "a.txt").write_text("data")
        _retry_rmtree(str(d))
        assert not d.exists()
