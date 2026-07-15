"""Tests for rez_next.module_root_path (API compatibility with rez)."""

import os

import rez_next as rez


class TestModuleRootPath:
    """Test module_root_path attribute for API compatibility."""

    def test_module_root_path_exists(self) -> None:
        """Test that module_root_path attribute exists."""
        assert hasattr(rez, "module_root_path"), "rez_next should have 'module_root_path' attribute"

    def test_module_root_path_is_string(self) -> None:
        """Test that module_root_path is a string."""
        assert isinstance(rez.module_root_path, str), "module_root_path should be a string"

    def test_module_root_path_is_valid_directory(self) -> None:
        """Test that module_root_path points to a valid directory."""
        assert os.path.isdir(rez.module_root_path), (
            f"module_root_path should be a valid directory: {rez.module_root_path}"
        )

    def test_module_root_path_ends_with_rez_next(self) -> None:
        """Test that module_root_path ends with 'rez_next' (package directory)."""
        assert rez.module_root_path.endswith("rez_next"), (
            "module_root_path should end with 'rez_next'"
        )
