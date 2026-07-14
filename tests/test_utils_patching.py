"""
Tests for rez_next.utils.patching module.

Aligns with rez's ``utils.patching`` API.
"""

import pytest
from rez_next.utils.patching import get_patched_request
from rez_next._native import PackageRequirement as Requirement


class TestGetPatchedRequest:
    """Tests for get_patched_request function."""

    def test_patch_replaces_existing(self):
        """Basic patch: replace foo-5 with foo-6."""
        result = get_patched_request(["foo-5", "bah-8.1"], ["foo-6"])
        assert len(result) == 2
        assert str(result[0]) == "foo-6"
        assert str(result[1]) == "bah-8.1"

    def test_patch_caret_removes(self):
        """^ prefix removes matching request entirely."""
        result = get_patched_request(["foo-5", "bah-8.1"], ["^bah"])
        assert len(result) == 1
        assert str(result[0]).startswith("foo")

    def test_patch_appends_new(self):
        """Unknown patch request is appended."""
        result = get_patched_request(["foo-5"], ["new_pkg-1"])
        assert len(result) == 2
        assert str(result[1]) == "new_pkg-1"

    def test_patch_bang_none_conflict(self):
        """! prefix: does NOT override normal foo (rules['!'] = (F,F,F))."""
        result = get_patched_request(["foo-5"], ["!foo"])
        # !foo only replaces an existing !foo, not normal foo
        assert len(result) == 2
        assert str(result[0]) == "foo-5"
        assert str(result[1]) == "!foo"

    def test_patch_bang_always_appends(self):
        """! prefix NEVER replaces existing (rules['!'] = (F,F,F))."""
        result = get_patched_request(["!foo"], ["!foo"])
        # !foo never replaces, always appends
        assert len(result) == 2
        assert str(result[0]) == "!foo"
        assert str(result[1]) == "!foo"

    def test_patch_tilde_does_not_override_weak(self):
        """~ prefix does NOT override ~foo in practice (rez code bug: ~foo has conflict=False)."""
        result = get_patched_request(["~foo-5"], ["~foo-6"])
        # Per rez code: ~foo-5 has conflict=False -> rule[0]=False -> NOT replaced
        assert len(result) == 2
        assert str(result[0]) == "~foo-5"

    def test_patch_tilde_does_not_override_normal(self):
        """~ prefix does NOT override normal foo."""
        result = get_patched_request(["foo-5"], ["~foo-6"])
        assert len(result) == 2
        assert str(result[0]) == "foo-5"

    def test_patch_requirement_objects(self):
        """Input can be Requirement objects instead of strings."""
        result = get_patched_request(
            [Requirement("foo-5"), Requirement("bah-8.1")],
            ["foo-6"],
        )
        assert isinstance(result[0], Requirement)
        assert str(result[0]) == "foo-6"

    def test_empty_patchlist(self):
        """Empty patchlist returns unchanged request."""
        result = get_patched_request(["foo-5", "bah-8.1"], [])
        assert len(result) == 2

    def test_multiple_patches(self):
        """Multiple patches applied in order."""
        result = get_patched_request(
            ["foo-5", "bah-8.1", "dep-1"],
            ["foo-6", "^dep"],
        )
        assert len(result) == 2
        assert str(result[0]) == "foo-6"

    def test_patch_empty_requires(self):
        """Empty requires with patches appends all."""
        result = get_patched_request([], ["foo-5", "bah-1"])
        assert len(result) == 2

    def test_patch_same_name_different_version(self):
        """Patch with same name but different version."""
        result = get_patched_request(["python-3.9"], ["python-3.11"])
        assert str(result[0]) == "python-3.11"
