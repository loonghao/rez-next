"""
Test module imports and error handling for rez-core.

This module tests the import mechanisms and error handling fallbacks
in the rez-core package.
"""

import sys
from unittest.mock import MagicMock, patch

import pytest


class TestModuleImports:
    """Test module import mechanisms and fallbacks."""

    def test_main_module_imports(self):
        """Test that main module imports work correctly."""
        import rez_core

        # Test that all expected attributes are available
        expected_attrs = [
            "Version",
            "VersionRange",
            "parse_version",
            "parse_version_range",
            "VersionToken",
            "NumericToken",
            "AlphanumericVersionToken",
            "Config",
            "RezCoreConfig",
            "__version__",
            "__author__",
            "__email__",
        ]

        for attr in expected_attrs:
            assert hasattr(rez_core, attr), f"Missing attribute: {attr}"

    def test_error_import_fallbacks(self):
        """Test error import fallbacks when Rust module is not available."""
        # Test PyVersionParseError fallback
        with patch.dict("sys.modules", {"rez_core._rez_core": None}):
            # Remove the module from cache to force re-import
            if "rez_core" in sys.modules:
                del sys.modules["rez_core"]

            # Mock the _rez_core module to raise ImportError for specific imports
            mock_module = MagicMock()
            del mock_module.PyVersionParseError  # This will cause AttributeError
            del mock_module.RezCoreError  # This will cause AttributeError

            with patch.dict("sys.modules", {"rez_core._rez_core": mock_module}):
                # This should trigger the fallback imports
                import rez_core

                # Test that fallbacks are used
                assert rez_core.VersionParseError == ValueError
                assert rez_core.RezCoreError == Exception

    def test_submodule_imports(self):
        """Test that submodules are properly imported."""
        import rez_core

        # Test submodule availability
        assert hasattr(rez_core, "version_module")
        assert hasattr(rez_core, "tokens_module")
        assert hasattr(rez_core, "errors_module")

        # Test that submodules have expected content
        assert hasattr(rez_core.version_module, "create_version")
        assert hasattr(rez_core.tokens_module, "create_token")
        assert hasattr(rez_core.errors_module, "VersionError")

    def test_config_alias(self):
        """Test that Config alias works correctly."""
        import rez_core

        # Test that Config and RezCoreConfig refer to the same thing
        assert rez_core.Config is rez_core.RezCoreConfig

    def test_all_exports(self):
        """Test that __all__ contains all expected exports."""
        import rez_core

        # Test that all items in __all__ are actually available
        for item in rez_core.__all__:
            assert hasattr(
                rez_core, item
            ), f"__all__ contains {item} but it's not available"

    def test_metadata_attributes(self):
        """Test module metadata attributes."""
        import rez_core

        assert isinstance(rez_core.__version__, str)
        assert isinstance(rez_core.__author__, str)
        assert isinstance(rez_core.__email__, str)

        # Test that metadata has reasonable values
        assert len(rez_core.__version__) > 0
        assert len(rez_core.__author__) > 0
        assert "@" in rez_core.__email__


class TestErrorsModule:
    """Test the errors module functionality."""

    def test_error_classes_inheritance(self):
        """Test that error classes have correct inheritance."""
        import rez_core
        from rez_core.errors import RangeParseError, TokenError, VersionError

        # Test inheritance chain
        assert issubclass(VersionError, rez_core.VersionParseError)
        assert issubclass(RangeParseError, rez_core.VersionParseError)
        assert issubclass(TokenError, rez_core.VersionParseError)

    def test_handle_version_error_decorator(self):
        """Test the handle_version_error decorator."""
        import rez_core
        from rez_core.errors import (
            RangeParseError,
            TokenError,
            VersionError,
            handle_version_error,
        )

        @handle_version_error
        def test_func_version_error():
            raise ValueError("Invalid version format")

        @handle_version_error
        def test_func_range_error():
            raise ValueError("Invalid range specification")

        @handle_version_error
        def test_func_token_error():
            raise ValueError("Invalid token value")

        @handle_version_error
        def test_func_generic_error():
            raise ValueError("Some other error")

        @handle_version_error
        def test_func_version_parse_error():
            raise rez_core.VersionParseError("Direct version parse error")

        @handle_version_error
        def test_func_unexpected_error():
            raise RuntimeError("Unexpected error")

        # Test version error handling
        with pytest.raises(VersionError):
            test_func_version_error()

        # Test range error handling
        with pytest.raises(RangeParseError):
            test_func_range_error()

        # Test token error handling
        with pytest.raises(TokenError):
            test_func_token_error()

        # Test generic ValueError handling
        with pytest.raises(VersionError):
            test_func_generic_error()

        # Test that VersionParseError is re-raised as-is
        with pytest.raises(rez_core.VersionParseError):
            test_func_version_parse_error()

        # Test unexpected error handling
        with pytest.raises(rez_core.RezCoreError):
            test_func_unexpected_error()

    def test_error_module_exports(self):
        """Test that error module exports are correct."""
        import rez_core.errors as errors

        # Test that all items in __all__ are available
        for item in errors.__all__:
            assert hasattr(
                errors, item
            ), f"errors.__all__ contains {item} but it's not available"


class TestTokensModule:
    """Test the tokens module functionality."""

    def test_create_token_numeric(self):
        """Test create_token with numeric strings."""
        import rez_core
        from rez_core.tokens import create_token

        # Test numeric token creation
        token = create_token("123")
        assert isinstance(token, rez_core.NumericToken)

        token = create_token("0")
        assert isinstance(token, rez_core.NumericToken)

    def test_create_token_alphanumeric(self):
        """Test create_token with alphanumeric strings."""
        import rez_core
        from rez_core.tokens import create_token

        # Test alphanumeric token creation
        token = create_token("abc")
        assert isinstance(token, rez_core.AlphanumericVersionToken)

        token = create_token("alpha1")
        assert isinstance(token, rez_core.AlphanumericVersionToken)

        token = create_token("beta")
        assert isinstance(token, rez_core.AlphanumericVersionToken)

    def test_create_token_empty_string(self):
        """Test create_token with empty string."""
        from rez_core.tokens import create_token

        with pytest.raises(ValueError, match="Token string cannot be empty"):
            create_token("")

    def test_parse_token_string_empty(self):
        """Test parse_token_string with empty string."""
        from rez_core.tokens import parse_token_string

        result = parse_token_string("")
        expected = {
            "type": "empty",
            "is_numeric": False,
            "is_alphanumeric": False,
            "length": 0,
        }
        assert result == expected

    def test_parse_token_string_numeric(self):
        """Test parse_token_string with numeric strings."""
        import rez_core
        from rez_core.tokens import parse_token_string

        # Test simple numeric
        result = parse_token_string("123")
        assert result["type"] == "numeric"
        assert result["is_numeric"] is True
        assert result["is_alphanumeric"] is True
        assert result["length"] == 3
        assert result["has_leading_zeros"] is False
        assert result["token_class"] == rez_core.NumericToken

        # Test with leading zeros
        result = parse_token_string("0123")
        assert result["has_leading_zeros"] is True

    def test_parse_token_string_alphanumeric(self):
        """Test parse_token_string with alphanumeric strings."""
        import rez_core
        from rez_core.tokens import parse_token_string

        result = parse_token_string("abc123")
        assert result["type"] == "alphanumeric"
        assert result["is_numeric"] is False
        assert result["is_alphanumeric"] is True
        assert result["length"] == 6
        assert result["has_leading_zeros"] is False
        assert result["token_class"] == rez_core.AlphanumericVersionToken

    def test_tokens_module_exports(self):
        """Test that tokens module exports are correct."""
        import rez_core.tokens as tokens

        # Test that all items in __all__ are available
        for item in tokens.__all__:
            assert hasattr(
                tokens, item
            ), f"tokens.__all__ contains {item} but it's not available"


class TestVersionModule:
    """Test the version module functionality."""

    def test_create_version(self):
        """Test create_version function."""
        import rez_core
        from rez_core.version import create_version

        version = create_version("1.2.3")
        assert isinstance(version, rez_core.Version)
        assert str(version) == "1.2.3"

    def test_create_range(self):
        """Test create_range function."""
        import rez_core
        from rez_core.version import create_range

        version_range = create_range(">=1.0.0")
        assert isinstance(version_range, rez_core.VersionRange)

    def test_compare_versions_strings(self):
        """Test compare_versions with string inputs."""
        from rez_core.version import compare_versions

        # Test less than
        assert compare_versions("1.0.0", "2.0.0") == -1

        # Test greater than
        assert compare_versions("2.0.0", "1.0.0") == 1

        # Test equal
        assert compare_versions("1.0.0", "1.0.0") == 0

    def test_compare_versions_objects(self):
        """Test compare_versions with Version objects."""
        import rez_core
        from rez_core.version import compare_versions

        v1 = rez_core.Version("1.0.0")
        v2 = rez_core.Version("2.0.0")

        assert compare_versions(v1, v2) == -1
        assert compare_versions(v2, v1) == 1
        assert compare_versions(v1, v1) == 0

    def test_sort_versions_strings(self):
        """Test sort_versions with string inputs."""
        from rez_core.version import sort_versions

        versions = ["2.0.0", "1.0.0", "1.5.0"]
        sorted_versions = sort_versions(versions)

        # Should be sorted in ascending order
        assert str(sorted_versions[0]) == "1.0.0"
        assert str(sorted_versions[1]) == "1.5.0"
        assert str(sorted_versions[2]) == "2.0.0"

        # Test reverse sorting
        sorted_versions_desc = sort_versions(versions, reverse=True)
        assert str(sorted_versions_desc[0]) == "2.0.0"
        assert str(sorted_versions_desc[2]) == "1.0.0"

    def test_sort_versions_objects(self):
        """Test sort_versions with Version objects."""
        import rez_core
        from rez_core.version import sort_versions

        versions = [
            rez_core.Version("2.0.0"),
            rez_core.Version("1.0.0"),
            rez_core.Version("1.5.0"),
        ]
        sorted_versions = sort_versions(versions)

        assert str(sorted_versions[0]) == "1.0.0"
        assert str(sorted_versions[1]) == "1.5.0"
        assert str(sorted_versions[2]) == "2.0.0"

    def test_version_module_exports(self):
        """Test that version module exports are correct."""
        import rez_core.version as version

        # Test that all items in __all__ are available
        for item in version.__all__:
            assert hasattr(
                version, item
            ), f"version.__all__ contains {item} but it's not available"
