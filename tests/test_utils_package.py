"""Tests for rez_next.utils package.

Aligned with Rez's ``rez.utils`` API (with_noop, reraise).
"""

import pytest
from rez_next.utils import with_noop, reraise


class TestWithNoop:
    """Test rez_next.utils.with_noop()."""

    def test_with_noop_context_manager(self):
        """Test with_noop() can be used as a context manager."""
        with with_noop():
            pass  # should not raise

    def test_with_noop_yields_none(self):
        """Test with_noop() yields no value."""
        with with_noop() as value:
            assert value is None


class TestReraise:
    """Test rez_next.utils.reraise()."""

    def test_reraise_changes_exception_type(self):
        """Test reraise() changes the exception type."""
        with pytest.raises(RuntimeError) as exc_info:
            try:
                raise ValueError("original message")
            except ValueError as e:
                reraise(e, RuntimeError)

        assert isinstance(exc_info.value, RuntimeError)
        assert "original message" in str(exc_info.value)

    def test_reraise_preserves_traceback(self):
        """Test reraise() preserves the original traceback."""
        import traceback

        try:
            try:
                raise ValueError("original message")
            except ValueError as e:
                reraise(e, RuntimeError)
        except RuntimeError:
            tb = traceback.format_exc()
            assert "ValueError" in tb or "original message" in tb

    def test_reraise_raises_always(self):
        """Test reraise() always raises an exception (NoReturn)."""
        with pytest.raises(LookupError):
            try:
                raise KeyError("missing key")
            except KeyError as e:
                reraise(e, LookupError)

    def test_reraise_no_suppressed_context(self):
        """Test reraise() does not chain exceptions (from None)."""
        try:
            try:
                raise ValueError("inner")
            except ValueError as e:
                reraise(e, RuntimeError)
        except RuntimeError as e:
            # Should NOT have __cause__ since we use from None
            assert e.__cause__ is None


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
