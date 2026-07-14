"""
Tests for the rez_next.exceptions module.

Verifies that the exception hierarchy matches rez.exceptions.
"""
from __future__ import annotations

import pytest


class TestExceptionHierarchy:
    """Verify the rez_next exception hierarchy matches rez.exceptions."""

    def test_import_all_exceptions(self) -> None:
        """All exception classes should be importable from rez_next.exceptions."""
        from rez_next.exceptions import (
            RezError,
            RezSystemError,
            RezBindError,
            RezPluginError,
            ConfigurationError,
            ResolveError,
            PackageFamilyNotFoundError,
            PackageNotFoundError,
            ResourceError,
            ResourceNotFoundError,
            ResourceContentError,
            PackageMetadataError,
            PackageCommandError,
            PackageRequestError,
            PackageCopyError,
            PackageMoveError,
            ContextBundleError,
            PackageCacheError,
            PackageTestError,
            InvalidPackageError,
            ResolvedContextError,
            RexError,
            RexUndefinedVariableError,
            RexStopError,
            BuildError,
            BuildSystemError,
            BuildContextResolveError,
            BuildProcessError,
            ReleaseError,
            ReleaseVCSError,
            ReleaseHookError,
            ReleaseHookCancellingError,
            SuiteError,
            PackageRepositoryError,
            RezGuiQTImportError,
        )

    def test_exception_inheritance(self) -> None:
        """Verify inheritance hierarchy matches rez.exceptions."""
        from rez_next.exceptions import (
            RezError,
            RezSystemError,
            ResourceError,
            ResourceContentError,
            PackageMetadataError,
            RexError,
            RexUndefinedVariableError,
            RexStopError,
            BuildError,
            BuildSystemError,
            BuildContextResolveError,
            ReleaseError,
            ReleaseVCSError,
            RezGuiQTImportError,
        )

        # Base
        assert issubclass(RezError, Exception)

        # RezError subclasses
        assert issubclass(RezSystemError, RezError)

        # Resource hierarchy
        assert issubclass(ResourceError, RezError)
        assert issubclass(ResourceContentError, ResourceError)
        assert issubclass(PackageMetadataError, ResourceContentError)

        # Rex hierarchy
        assert issubclass(RexError, RezError)
        assert issubclass(RexUndefinedVariableError, RexError)
        assert issubclass(RexStopError, RexError)

        # Build hierarchy
        assert issubclass(BuildError, RezError)
        assert issubclass(BuildSystemError, BuildError)
        assert issubclass(BuildContextResolveError, BuildError)

        # Release hierarchy
        assert issubclass(ReleaseError, RezError)
        assert issubclass(ReleaseVCSError, ReleaseError)

        # RezGuiQTImportError extends ImportError, not RezError
        assert issubclass(RezGuiQTImportError, ImportError)
        assert not issubclass(RezGuiQTImportError, RezError)

    def test_resource_content_error_with_path(self) -> None:
        """ResourceContentError should format with path correctly."""
        from rez_next.exceptions import ResourceContentError

        exc = ResourceContentError("bad value", path="/path/to/file.py")
        assert "/path/to/file.py" in str(exc)
        assert "bad value" in str(exc)

    def test_package_metadata_error_type_name(self) -> None:
        """PackageMetadataError should have correct type_name."""
        from rez_next.exceptions import PackageMetadataError

        assert PackageMetadataError.type_name == "package definition file"

    def test_build_context_resolve_error(self) -> None:
        """BuildContextResolveError should format with context status."""
        from rez_next.exceptions import BuildContextResolveError

        class MockContext:
            status = "unsolved"
            failure_description = "Could not resolve python-3"

        ctx = MockContext()
        exc = BuildContextResolveError(ctx)
        assert "Could not resolve python-3" in str(exc)
        assert exc.context is ctx

    def test_convert_errors_context_manager(self) -> None:
        """convert_errors context manager should convert exception types."""
        from rez_next.exceptions import convert_errors, RezError, RezSystemError

        with pytest.raises(RezSystemError):
            with convert_errors(ValueError, RezSystemError):
                raise ValueError("test error")

    def test_convert_errors_no_exception(self) -> None:
        """convert_errors should not raise when no exception occurs."""
        from rez_next.exceptions import convert_errors, RezError, RezSystemError

        result = None
        with convert_errors(ValueError, RezSystemError):
            result = 42
        assert result == 42

    def test_convert_errors_other_exception(self) -> None:
        """convert_errors should let unrelated exceptions propagate."""
        from rez_next.exceptions import convert_errors, RezError, RezSystemError

        with pytest.raises(TypeError):
            with convert_errors(ValueError, RezSystemError):
                raise TypeError("unrelated")


class TestDeprecations:
    """Tests for rez_next.deprecations module."""

    def test_rez_deprecation_warning_class(self) -> None:
        """RezDeprecationWarning should be a DeprecationWarning subclass."""
        from rez_next.deprecations import RezDeprecationWarning

        assert issubclass(RezDeprecationWarning, DeprecationWarning)

    def test_warn_basic(self) -> None:
        """warn() should issue a deprecation warning."""
        import warnings
        from rez_next.deprecations import warn, RezDeprecationWarning

        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            warn("test deprecation")
            assert len(w) == 1
            assert "test deprecation" in str(w[0].message)
            assert w[0].category is RezDeprecationWarning

    def test_warn_custom_category(self) -> None:
        """warn() should accept a custom warning category."""
        import warnings
        from rez_next.deprecations import warn

        class CustomWarning(UserWarning):
            pass

        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            warn("custom warning", category=CustomWarning)
            assert len(w) == 1
            assert w[0].category is CustomWarning
