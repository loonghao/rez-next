"""
Tests for rez_next.bundle_context module.
"""

import os
import tempfile

import pytest

from rez_next.bundle_context import bundle_context


class TestBundleContext:
    """Test bundle_context function."""

    def test_bundle_context_creates_directory(self):
        """bundle_context should create destination directory."""
        with tempfile.TemporaryDirectory() as tmpdir:
            dest = os.path.join(tmpdir, "my_bundle")

            # bundle_context with minimal args
            bundle_context(
                context=None,
                dest_dir=dest,
                quiet=True,
            )

            assert os.path.isdir(dest)

    def test_bundle_context_creates_bundle_yaml(self):
        """bundle_context should create a bundle.yaml manifest."""
        with tempfile.TemporaryDirectory() as tmpdir:
            dest = os.path.join(tmpdir, "my_bundle")

            bundle_context(
                context=None,
                dest_dir=dest,
                quiet=True,
            )

            bundle_yaml = os.path.join(dest, "bundle.yaml")
            assert os.path.isfile(bundle_yaml)

    def test_bundle_context_raises_if_dest_exists_without_force(self):
        """bundle_context should raise FileExistsError if dest exists without force."""
        with tempfile.TemporaryDirectory() as tmpdir:
            # Create the directory first
            os.makedirs(tmpdir, exist_ok=True)

            with pytest.raises(FileExistsError):
                bundle_context(
                    context=None,
                    dest_dir=tmpdir,
                    quiet=True,
                )

    def test_bundle_context_force_overwrites(self):
        """bundle_context with force=True should overwrite existing dir."""
        with tempfile.TemporaryDirectory() as tmpdir:
            dest = os.path.join(tmpdir, "force_bundle")
            os.makedirs(dest, exist_ok=True)

            # Should not raise with force=True
            bundle_context(
                context=None,
                dest_dir=dest,
                force=True,
                quiet=True,
            )

            assert os.path.isdir(dest)

    def test_bundle_context_accepts_skip_non_relocatable(self):
        """bundle_context should accept skip_non_relocatable flag."""
        with tempfile.TemporaryDirectory() as tmpdir:
            dest = os.path.join(tmpdir, "skip_bundle")

            bundle_context(
                context=None,
                dest_dir=dest,
                skip_non_relocatable=True,
                quiet=True,
            )

            assert os.path.isdir(dest)

    def test_bundle_context_accepts_patch_libs(self):
        """bundle_context should accept patch_libs flag."""
        with tempfile.TemporaryDirectory() as tmpdir:
            dest = os.path.join(tmpdir, "patch_bundle")

            bundle_context(
                context=None,
                dest_dir=dest,
                patch_libs=True,
                quiet=True,
            )

            assert os.path.isdir(dest)
