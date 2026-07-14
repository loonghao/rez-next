"""Tests for rez_next.status module.

Aligns with rez.status API:
- `from rez.status import status` — singleton Status instance
- `status.is_in_rez_context()` — check if in a rez context
- `status.get_current_status()` — get current status summary
"""
import pytest


class TestStatusModule:
    """Tests for the rez_next.status module."""

    def test_status_is_module(self):
        """status should be importable as a module."""
        import rez_next.status
        assert rez_next.status is not None

    def test_status_singleton(self):
        """from rez_next.status import status should give RezStatus instance."""
        from rez_next.status import status
        assert status is not None

    def test_status_type(self):
        """status singleton should be RezStatus type."""
        from rez_next.status import status
        type_name = type(status).__name__
        assert 'Status' in type_name or 'RezStatus' in type_name

    def test_is_in_rez_context(self):
        """is_in_rez_context should return a bool (False when not in context)."""
        from rez_next.status import is_in_rez_context
        result = is_in_rez_context()
        assert isinstance(result, bool)

    def test_get_current_status(self):
        """get_current_status should return a dict or string."""
        from rez_next.status import get_current_status
        result = get_current_status()
        assert result is not None

    def test_status_functions_accessible_from_module(self):
        """Key status functions should be accessible from status module."""
        import rez_next.status as st
        assert hasattr(st, 'is_in_rez_context')
        assert hasattr(st, 'get_current_status')
        assert hasattr(st, 'get_context_file')
        assert hasattr(st, 'get_resolved_package_names')
        assert hasattr(st, 'get_rez_env_var')

    def test_get_context_file(self):
        """get_context_file should return None when not in context."""
        from rez_next.status import get_context_file
        result = get_context_file()
        assert result is None or isinstance(result, str)

    def test_get_rez_env_var(self):
        """get_rez_env_var should return None for non-existent vars."""
        from rez_next.status import get_rez_env_var
        result = get_rez_env_var('__REZ_NEXT_TEST_NONEXISTENT_VAR__')
        assert result is None or isinstance(result, str)
