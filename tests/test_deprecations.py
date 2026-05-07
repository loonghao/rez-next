"""Tests for rez_next.deprecations module.

This module tests the deprecation utilities that provide
compatibility with rez.deprecations.
"""

import warnings
import pytest
import rez_next as rez
from rez_next.deprecations import RezDeprecationWarning, warn


class TestRezDeprecationWarning:
    """Tests for RezDeprecationWarning class."""

    def test_is_subclass_of_deprecation_warning(self):
        """RezDeprecationWarning should be a subclass of DeprecationWarning."""
        assert issubclass(RezDeprecationWarning, DeprecationWarning)

    def test_can_be_raised_as_warning(self):
        """RezDeprecationWarning can be raised and caught."""
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            warnings.warn("test message", RezDeprecationWarning)
            assert len(w) == 1
            assert issubclass(w[0].category, RezDeprecationWarning)

    def test_can_be_instantiated(self):
        """RezDeprecationWarning can be instantiated."""
        warning = RezDeprecationWarning("test")
        assert str(warning) == "test"


class TestWarnFunction:
    """Tests for the warn() function."""

    def test_basic_warning(self):
        """warn() should emit a warning."""
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            warn("test message")
            assert len(w) == 1
            assert "test message" in str(w[0].message)

    def test_custom_category(self):
        """warn() should accept a custom category."""
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            warn("custom warning", category=UserWarning)
            assert len(w) == 1
            assert issubclass(w[0].category, UserWarning)

    def test_pre_formatted_false(self):
        """warn() with pre_formatted=False should use standard formatting."""
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            warn("test", pre_formatted=False)
            assert len(w) == 1

    def test_pre_formatted_true(self):
        """warn() with pre_formatted=True should emit a warning."""
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            warn("pre-formatted message", pre_formatted=True)
            assert len(w) == 1
            # Just check that the warning is emitted with correct category
            assert issubclass(w[0].category, RezDeprecationWarning)

    def test_stacklevel(self):
        """warn() should respect stacklevel parameter."""
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")

            def nested_function():
                warn("stacklevel test", stacklevel=2)

            nested_function()
            assert len(w) == 1

    def test_filename_parameter(self):
        """warn() should accept filename parameter."""
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            warn(
                "test with filename",
                pre_formatted=True,
                filename="/fake/path.py",
            )
            assert len(w) == 1
            # Just check that the warning is emitted
            assert issubclass(w[0].category, RezDeprecationWarning)


class TestModuleCompatibility:
    """Tests for module-level compatibility."""

    def test_warnings_module_accessible(self):
        """warnings module should be accessible via rez.deprecations.warnings."""
        assert hasattr(rez.deprecations, "warnings")
        import warnings as _warnings

        # Should be the same module object
        assert rez.deprecations.warnings is _warnings

    def test_module_has_warn_function(self):
        """Module should have warn function accessible."""
        assert callable(warn)
        assert hasattr(rez.deprecations, "warn")

    def test_module_has_RezDeprecationWarning(self):
        """Module should have RezDeprecationWarning class accessible."""
        assert hasattr(rez.deprecations, "RezDeprecationWarning")
        assert RezDeprecationWarning is not None


class TestRezCompatibility:
    """Tests to ensure compatibility with original rez.deprecations."""

    def test_import_works(self):
        """Importing rez_next should provide deprecations module."""
        assert hasattr(rez, "deprecations")

    def test_warn_signature_compatible(self):
        """warn() signature should be compatible with rez.deprecations.warn."""
        import inspect

        sig = inspect.signature(warn)
        params = list(sig.parameters.keys())
        assert "message" in params
        assert "category" in params
        assert "pre_formatted" in params
