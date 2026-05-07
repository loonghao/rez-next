#!/usr/bin/env python
"""Tests for rez_next.utils module (Cycle 304).

Tests the Python bindings for utility functions implemented in Rust
(rez-next-util crate).
"""

import os
import sys
import pytest

# Add the python directory to path so we can import rez_next
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'python'))

import rez_next.utils as utils


class TestPlatformDetection:
    """Test platform detection functions."""

    def test_get_platform_name(self):
        """Test that get_platform_name returns a valid platform."""
        platform = utils.get_platform_name()
        assert platform in ('windows', 'linux', 'macos', 'unknown')
        assert isinstance(platform, str)

    def test_get_architecture(self):
        """Test that get_architecture returns a valid architecture."""
        arch = utils.get_architecture()
        assert arch in ('x86_64', 'x86', 'aarch64', 'arm', 'unknown')
        assert isinstance(arch, str)

    def test_get_platform_id(self):
        """Test that get_platform_id returns platform-architecture format."""
        platform_id = utils.get_platform_id()
        assert '-' in platform_id
        assert len(platform_id.split('-')) == 2

    def test_is_windows(self):
        """Test is_windows returns a boolean."""
        result = utils.is_windows()
        assert isinstance(result, bool)

    def test_is_linux(self):
        """Test is_linux returns a boolean."""
        result = utils.is_linux()
        assert isinstance(result, bool)

    def test_is_macos(self):
        """Test is_macos returns a boolean."""
        result = utils.is_macos()
        assert isinstance(result, bool)

    def test_is_unix(self):
        """Test is_unix returns a boolean."""
        result = utils.is_unix()
        assert isinstance(result, bool)
        # is_unix should be True if either linux or macos
        if utils.is_linux() or utils.is_macos():
            assert result is True


class TestStringUtilities:
    """Test string utility functions."""

    def test_normalize_name(self):
        """Test normalize_name converts to snake_case."""
        assert utils.normalize_name('Hello World') == 'hello_world'
        assert utils.normalize_name('maya-2024') == 'maya_2024'
        assert utils.normalize_name('My-Package_Name') == 'my_package_name'

    def test_truncate_string(self):
        """Test truncate_string truncates to max_len with ellipsis."""
        # Short string should not be truncated
        assert utils.truncate_string('hello', 10) == 'hello'
        # Long string should be truncated with ...
        result = utils.truncate_string('hello world', 8)
        assert result == 'hello...'
        assert len(result) == 8


class TestVersionAndExecutable:
    """Test version and executable functions."""

    def test_get_rez_next_version(self):
        """Test get_rez_next_version returns a valid version string."""
        version = utils.get_rez_next_version()
        assert isinstance(version, str)
        assert len(version) > 0
        # Version should be in format X.Y.Z
        parts = version.split('.')
        assert len(parts) == 3

    def test_get_executable_name(self):
        """Test get_executable_name returns a string."""
        name = utils.get_executable_name()
        assert isinstance(name, str)
        assert len(name) > 0


class TestWhich:
    """Test which/which_all functions."""

    @pytest.mark.skipif(sys.platform != 'win32', reason='Windows-specific test')
    def test_which_python_windows(self):
        """Test which finds python on Windows."""
        result = utils.which('python.exe')
        if result:
            assert result.endswith('.exe') or 'python' in result.lower()

    @pytest.mark.skipif(sys.platform == 'win32', reason='Unix-specific test')
    def test_which_python_unix(self):
        """Test which finds python on Unix."""
        result = utils.which('python3')
        if result:
            assert 'python' in result

    def test_which_nonexistent(self):
        """Test which returns None for nonexistent command."""
        result = utils.which('nonexistent_command_xyz_12345')
        assert result is None

    def test_which_all(self):
        """Test which_all returns a list."""
        results = utils.which_all('python')
        assert isinstance(results, list)


class TestFileSystem:
    """Test file system utility functions."""

    def test_expand_user_path(self):
        """Test expand_user_path expands ~ to home directory."""
        result = utils.expand_user_path('~/test.txt')
        assert isinstance(result, str)
        assert result != '~/test.txt'
        # Should contain the home directory path
        import os
        home = os.path.expanduser('~')
        assert result.startswith(home)

    def test_ensure_dir_exists(self, tmp_path):
        """Test ensure_dir_exists creates directory."""
        new_dir = tmp_path / 'subdir' / 'nested'
        utils.ensure_dir_exists(str(new_dir))
        assert new_dir.exists()
        assert new_dir.is_dir()

    def test_ensure_parent_dir_exists(self, tmp_path):
        """Test ensure_parent_dir_exists creates parent directory."""
        file_path = tmp_path / 'subdir' / 'test.txt'
        utils.ensure_parent_dir_exists(str(file_path))
        assert file_path.parent.exists()

    def test_is_writable(self, tmp_path):
        """Test is_writable returns True for writable paths."""
        # Temp directory should be writable
        assert utils.is_writable(str(tmp_path)) is True

        # Create a file and check
        file_path = tmp_path / 'test.txt'
        file_path.write_text('test')
        assert utils.is_writable(str(file_path)) is True

    def test_is_writable_nonexistent(self, tmp_path):
        """Test is_writable for nonexistent file in writable dir."""
        file_path = tmp_path / 'new_file.txt'
        # Should return True because parent is writable
        assert utils.is_writable(str(file_path)) is True

    def test_safe_remove_file(self, tmp_path):
        """Test safe_remove removes a file."""
        file_path = tmp_path / 'to_remove.txt'
        file_path.write_text('test')
        assert file_path.exists()

        utils.safe_remove(str(file_path))
        assert not file_path.exists()

    def test_safe_remove_dir(self, tmp_path):
        """Test safe_remove removes a directory recursively."""
        dir_path = tmp_path / 'to_remove'
        dir_path.mkdir()
        (dir_path / 'nested.txt').write_text('test')
        assert dir_path.exists()

        utils.safe_remove(str(dir_path))
        assert not dir_path.exists()

    def test_copy_file(self, tmp_path):
        """Test copy_file copies a file."""
        src = tmp_path / 'src.txt'
        dst = tmp_path / 'subdir' / 'dst.txt'
        src.write_text('hello world')

        utils.copy_file(str(src), str(dst))
        assert dst.exists()
        assert dst.read_text() == 'hello world'


if __name__ == '__main__':
    pytest.main([__file__, '-v'])
