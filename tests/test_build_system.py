"""Tests for rez_next.build_system module."""

import pytest
from rez_next.build_system import (
    BuildSystem,
    BuildResult,
    get_buildsys_types,
)


class TestBuildResult:
    def test_is_typed_dict(self):
        result: BuildResult = {"success": True}
        assert result["success"] is True

    def test_all_fields_optional(self):
        result: BuildResult = {}
        assert isinstance(result, dict)


class TestBuildSystem:
    def test_name_raises(self):
        with pytest.raises(NotImplementedError):
            BuildSystem.name()

    def test_is_valid_root_raises(self):
        with pytest.raises(NotImplementedError):
            BuildSystem.is_valid_root("/tmp")

    def test_child_build_system_default(self):
        assert BuildSystem.child_build_system() is None

    def test_build_raises(self):
        bs = _create_unimplemented_build_system()
        with pytest.raises(NotImplementedError):
            bs.build(None, None, "", "")

    def test_init_raises_for_invalid_root(self):
        from rez_next.exceptions import BuildSystemError
        with pytest.raises(BuildSystemError):
            _create_concrete_build_system(working_dir="/nonexistent/path")


class TestFactoryFunctions:
    def test_get_buildsys_types(self):
        types = get_buildsys_types()
        assert isinstance(types, list)


# ── Helpers ────────────────────────────────────────────────────────────────


def _create_concrete_build_system(working_dir="/tmp"):
    """Create a minimal BuildSystem subclass for testing."""

    class _TestBuildSystem(BuildSystem):
        @classmethod
        def name(cls):
            return "test"

        @classmethod
        def is_valid_root(cls, path, package=None):
            return path == "/tmp"

        def build(self, context, variant, build_path, install_path,
                  install=False, build_type=None):
            return BuildResult(success=True)

    return _TestBuildSystem(
        working_dir=working_dir,
    )


def _create_unimplemented_build_system(working_dir="/tmp"):
    """Create a BuildSystem subclass without build() override."""

    class _TestBuildSystem(BuildSystem):
        @classmethod
        def name(cls):
            return "test"

        @classmethod
        def is_valid_root(cls, path, package=None):
            return path == "/tmp"

    return _TestBuildSystem(
        working_dir=working_dir,
    )
