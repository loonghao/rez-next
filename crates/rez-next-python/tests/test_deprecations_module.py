"""Tests for rez_next.deprecations module."""

import warnings

import pytest
import rez_next.deprecations as deprecations


class TestRezDeprecationWarning:
    """Tests for RezDeprecationWarning class."""

    def test_is_deprecation_warning(self):
        """RezDeprecationWarning should be a subclass of DeprecationWarning."""
        assert issubclass(deprecations.RezDeprecationWarning, DeprecationWarning)

    def test_can_be_raised(self):
        """RezDeprecationWarning can be raised and caught."""
        with pytest.raises(deprecations.RezDeprecationWarning):
            raise deprecations.RezDeprecationWarning("test warning")

    def test_can_be_caught_as_deprecation_warning(self):
        """RezDeprecationWarning can be caught as DeprecationWarning."""
        with warnings.catch_warnings(record=True):
            warnings.simplefilter("always")
            deprecations.warn("test", category=deprecations.RezDeprecationWarning)
            # Should not raise - just warned


class TestWarnFunction:
    """Tests for the warn() function."""

    def test_simple_warning(self):
        """warn() should issue a warning."""
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            deprecations.warn("test message", category=DeprecationWarning)
            assert len(w) == 1
            assert "test message" in str(w[0].message)

    def test_preformatted_warning(self):
        """warn() with pre_formatted=True should use custom format."""
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            deprecations.warn(
                "custom message",
                category=DeprecationWarning,
                pre_formatted=True,
                filename="test.py",
            )
            assert len(w) == 1

    def test_stacklevel(self):
        """warn() should respect stacklevel."""
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")

            def nested_warn():
                deprecations.warn(
                    "stacklevel test",
                    category=DeprecationWarning,
                    stacklevel=2,
                )

            nested_warn()
            assert len(w) == 1

    def test_rez_deprecation_warning(self):
        """warn() should work with RezDeprecationWarning."""
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            deprecations.warn(
                "rez deprecation",
                category=deprecations.RezDeprecationWarning,
            )
            assert len(w) == 1
            assert issubclass(w[0].category, deprecations.RezDeprecationWarning)


class TestModuleExports:
    """Tests for module-level exports."""

    def test_warnings_accessible(self):
        """warnings module should be accessible."""
        assert hasattr(deprecations, "warnings")
        import warnings

        assert deprecations.warnings is warnings

    def test_RezDeprecationWarning_accessible(self):
        """RezDeprecationWarning should be accessible."""
        assert hasattr(deprecations, "RezDeprecationWarning")

    def test_warn_accessible(self):
        """warn function should be accessible."""
        assert hasattr(deprecations, "warn")
        assert callable(deprecations.warn)
