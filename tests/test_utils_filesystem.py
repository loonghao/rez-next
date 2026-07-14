"""Tests for rez_next.utils.filesystem module."""

from __future__ import annotations

import os
import tempfile

import pytest
from rez_next.utils.filesystem import (
    find_matching_symlink,
    temp_dir,
    atomic_write,
    hardlink_or_copy,
)


class TestFindMatchingSymlink:
    """Tests for find_matching_symlink function."""

    def test_nonexistent_directory(self):
        result = find_matching_symlink("/nonexistent/path", "/target")
        assert result is None

    def test_empty_directory(self, tmp_path):
        result = find_matching_symlink(str(tmp_path), "/target")
        assert result is None

    def test_no_matching_symlink(self, tmp_path):
        # Create a regular file (not a symlink)
        f = tmp_path / "file.txt"
        f.write_text("hello")
        result = find_matching_symlink(str(tmp_path), "/target")
        assert result is None


class TestTempDir:
    """Tests for temp_dir context manager."""

    def test_temp_dir_created_and_cleaned(self):
        path = None
        with temp_dir(prefix="test_rez_") as p:
            path = p
            assert os.path.isdir(path)
            assert os.path.basename(path).startswith("test_rez_")
        # After context exit, directory should be removed
        assert not os.path.exists(path)

    def test_temp_dir_cleanup_on_error(self):
        path = None
        try:
            with temp_dir() as p:
                path = p
                raise ValueError("boom")
        except ValueError:
            pass
        assert path is not None
        assert not os.path.exists(path)


class TestAtomicWrite:
    """Tests for atomic_write context manager."""

    def test_writes_content(self, tmp_path):
        fpath = str(tmp_path / "test.txt")
        with atomic_write(fpath) as f:
            f.write("hello world")
        with open(fpath) as f:
            assert f.read() == "hello world"

    def test_no_partial_write_on_error(self, tmp_path):
        fpath = str(tmp_path / "test.txt")
        # File doesn't exist yet
        try:
            with atomic_write(fpath) as f:
                f.write("partial")
                raise ValueError("abort")
        except ValueError:
            pass
        # File should NOT exist (write was aborted)
        assert not os.path.exists(fpath)

    def test_overwrites_existing(self, tmp_path):
        fpath = str(tmp_path / "test.txt")
        with atomic_write(fpath) as f:
            f.write("first")
        with atomic_write(fpath) as f:
            f.write("second")
        with open(fpath) as f:
            assert f.read() == "second"


class TestHardlinkOrCopy:
    """Tests for hardlink_or_copy function."""

    def test_copy_fallback(self, tmp_path):
        src = str(tmp_path / "src.txt")
        dst = str(tmp_path / "dst.txt")
        with open(src, "w") as f:
            f.write("hello")

        result = hardlink_or_copy(src, dst)
        # Should have copied (not hardlinked across different platforms)
        assert os.path.exists(dst)
        with open(dst) as f:
            assert f.read() == "hello"

    def test_source_not_found(self, tmp_path):
        src = str(tmp_path / "nonexistent.txt")
        dst = str(tmp_path / "dst.txt")
        with pytest.raises(OSError):
            hardlink_or_copy(src, dst)
