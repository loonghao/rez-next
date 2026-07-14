"""
Tests for rez_next.release_vcs module.
"""

import os
import tempfile

import pytest

from rez_next.release_vcs import (
    ReleaseVCS,
    ReleaseVCSError,
    get_release_vcs_types,
    create_release_vcs,
)


class TestReleaseVCSError:
    """Test ReleaseVCSError exception."""

    def test_is_subclass_of_rez_system_error(self):
        """ReleaseVCSError should be a RezSystemError."""
        from rez_next.exceptions import RezSystemError

        assert issubclass(ReleaseVCSError, RezSystemError)


class TestReleaseVCSAbstract:
    """Test ReleaseVCS ABC cannot be instantiated."""

    def test_cannot_instantiate_abstract(self):
        """ReleaseVCS is abstract and cannot be instantiated."""
        with pytest.raises(TypeError):
            ReleaseVCS("/tmp")  # type: ignore[abstract]


class TestReleaseVCSConcrete:
    """Test a concrete ReleaseVCS subclass."""

    def test_concrete_subclass_registers_automatically(self):
        """A concrete subclass should auto-register."""

        class TestVCS(ReleaseVCS):
            @classmethod
            def name(cls) -> str:
                return "test_vcs"

            @classmethod
            def is_valid_root(cls, path: str) -> bool:
                return path == "/tmp/test_repo"

            @classmethod
            def search_parents_for_root(cls) -> bool:
                return False

            def validate_repostate(self) -> None:
                pass

            def get_current_revision(self) -> object:
                return "abc123"

            def get_changelog(self, previous_revision=None, max_revisions=None) -> str:
                return "test changelog"

            def tag_exists(self, tag_name: str) -> bool:
                return tag_name == "existing"

            def create_release_tag(self, tag_name: str, message=None) -> None:
                pass

            @classmethod
            def export(cls, revision: object, path: str) -> None:
                os.makedirs(path, exist_ok=True)

        try:
            assert TestVCS.name() == "test_vcs"
            assert "test_vcs" in ReleaseVCS._registry
            assert ReleaseVCS._registry["test_vcs"] == TestVCS

            # Test instance creation with explicit vcs_root
            with tempfile.TemporaryDirectory() as tmpdir:
                test_repo = os.path.join(tmpdir, "test_repo")
                os.makedirs(test_repo)

                vcs = TestVCS(pkg_root=test_repo, vcs_root=test_repo)
                assert vcs.pkg_root == test_repo
                assert vcs.vcs_root == test_repo
        finally:
            ReleaseVCS._registry.pop("test_vcs", None)


class TestFactoryFunctions:
    """Test get_release_vcs_types and create_release_vcs."""

    def test_get_release_vcs_types_returns_list(self):
        """get_release_vcs_types should return a list."""
        types = get_release_vcs_types()
        assert isinstance(types, list)

    def test_create_release_vcs_with_unknown_type(self):
        """create_release_vcs should raise on unknown type."""
        with pytest.raises(ReleaseVCSError):
            create_release_vcs("/tmp", vcs_name="nonexistent_vcs")

    def test_create_release_vcs_auto_detect_fails_with_no_registry(self):
        """create_release_vcs should raise when no VCS found."""
        with pytest.raises(ReleaseVCSError):
            create_release_vcs("/nonexistent_path")


class TestFindVCSRoot:
    """Test ReleaseVCS.find_vcs_root classmethod."""

    def test_find_vcs_root_no_search_parents(self):
        """With search_parents_for_root=False, only check the given path."""

        class NoSearchVCS(ReleaseVCS):
            @classmethod
            def name(cls) -> str:
                return "no_search"

            @classmethod
            def is_valid_root(cls, path: str) -> bool:
                return "valid" in path

            @classmethod
            def search_parents_for_root(cls) -> bool:
                return False

            def validate_repostate(self) -> None:
                pass

            def get_current_revision(self) -> object:
                return "r1"

            def get_changelog(self, previous_revision=None, max_revisions=None) -> str:
                return "log"

            def tag_exists(self, tag_name: str) -> bool:
                return False

            def create_release_tag(self, tag_name: str, message=None) -> None:
                pass

            @classmethod
            def export(cls, revision: object, path: str) -> None:
                pass

        try:
            # Should find if path contains "valid"
            with tempfile.TemporaryDirectory() as tmpdir:
                valid_path = os.path.join(tmpdir, "valid_repo")
                os.makedirs(valid_path, exist_ok=True)

                result = NoSearchVCS.find_vcs_root(valid_path)
                assert result is not None
                assert result[1] == 0  # depth = 0

            # Should NOT find without "valid" in path
            with tempfile.TemporaryDirectory() as tmpdir:
                invalid_path = os.path.join(tmpdir, "other_repo")
                os.makedirs(invalid_path, exist_ok=True)

                result = NoSearchVCS.find_vcs_root(invalid_path)
                assert result is None
        finally:
            ReleaseVCS._registry.pop("no_search", None)

    def test_find_vcs_root_searches_parents(self):
        """With search_parents_for_root=True, check parent dirs."""

        class ParentSearchVCS(ReleaseVCS):
            _marker_file = ".parentsearch"

            @classmethod
            def name(cls) -> str:
                return "parent_search"

            @classmethod
            def is_valid_root(cls, path: str) -> bool:
                return os.path.isfile(
                    os.path.join(path, cls._marker_file)
                )

            @classmethod
            def search_parents_for_root(cls) -> bool:
                return True

            def validate_repostate(self) -> None:
                pass

            def get_current_revision(self) -> object:
                return "r1"

            def get_changelog(self, previous_revision=None, max_revisions=None) -> str:
                return "log"

            def tag_exists(self, tag_name: str) -> bool:
                return False

            def create_release_tag(self, tag_name: str, message=None) -> None:
                pass

            @classmethod
            def export(cls, revision: object, path: str) -> None:
                pass

        try:
            with tempfile.TemporaryDirectory() as tmpdir:
                # Create: tmpdir/subdir/deep
                subdir = os.path.join(tmpdir, "subdir")
                deep = os.path.join(subdir, "deep")
                os.makedirs(deep)

                # Place marker in tmpdir (not in subdir)
                marker = os.path.join(tmpdir, ".parentsearch")
                with open(marker, "w") as f:
                    f.write("")

                # Searching from deep should find tmpdir (depth=2)
                result = ParentSearchVCS.find_vcs_root(deep)
                assert result is not None
                assert result[1] == 2  # 2 levels up
                assert result[0] == os.path.abspath(tmpdir)
        finally:
            ReleaseVCS._registry.pop("parent_search", None)
