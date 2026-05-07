"""
Tests for rez_next._native.release_hook module.

Tests the release hook system including:
- Getting available hook types
- Creating hook instances
- Calling hook methods
"""
import pytest
import rez_next._native as native


class TestGetReleaseHookTypes:
    """Tests for py_get_release_hook_types()."""
    
    def test_returns_list(self):
        """py_get_release_hook_types() should return a list."""
        types = native.release_hook.py_get_release_hook_types()
        assert isinstance(types, list)
    
    def test_includes_noop_hook(self):
        """Should include the built-in 'noop' hook."""
        types = native.release_hook.py_get_release_hook_types()
        assert "noop" in types
    
    def test_includes_logging_hook(self):
        """Should include the built-in 'logging' hook."""
        types = native.release_hook.py_get_release_hook_types()
        assert "logging" in types


class TestCreateReleaseHook:
    """Tests for create_release_hook()."""
    
    def test_create_noop_hook(self):
        """Should be able to create a noop hook."""
        hook = native.release_hook.py_create_release_hook(
            "noop", "/tmp/source"
        )
        assert hook is not None
    
    def test_create_logging_hook(self):
        """Should be able to create a logging hook."""
        hook = native.release_hook.py_create_release_hook(
            "logging", "/tmp/source"
        )
        assert hook is not None
    
    def test_create_invalid_hook_raises(self):
        """Should raise an error for invalid hook name."""
        with pytest.raises(Exception):
            native.release_hook.py_create_release_hook(
                "invalid_hook", "/tmp/source"
            )


class TestReleaseHookMethods:
    """Tests for ReleaseHook method calls."""
    
    def test_noop_pre_build(self):
        """NoopHook.pre_build() should not raise."""
        hook = native.release_hook.py_create_release_hook(
            "noop", "/tmp/source"
        )
        # Should not raise
        hook.pre_build("test_user", "/tmp/install", None, None, None, None, None)
    
    def test_noop_pre_release(self):
        """NoopHook.pre_release() should not raise."""
        hook = native.release_hook.py_create_release_hook(
            "noop", "/tmp/source"
        )
        # Should not raise
        hook.pre_release("test_user", "/tmp/install", None, None, None, None, None)
    
    def test_noop_post_release(self):
        """NoopHook.post_release() should not raise."""
        hook = native.release_hook.py_create_release_hook(
            "noop", "/tmp/source"
        )
        # Should not raise
        hook.post_release("test_user", "/tmp/install", [], None, None, None, None)
    
    def test_logging_pre_build(self):
        """LoggingHook.pre_build() should not raise."""
        hook = native.release_hook.py_create_release_hook(
            "logging", "/tmp/source"
        )
        # Should not raise
        hook.pre_build("test_user", "/tmp/install", None, None, None, None, None)


class TestReleaseHookModule:
    """Tests for release_hook module attributes."""
    
    def test_module_has_release_hook_class(self):
        """release_hook module should have ReleaseHook class."""
        assert hasattr(native.release_hook, "ReleaseHook")
    
    def test_module_has_get_types_function(self):
        """release_hook module should have py_get_release_hook_types function."""
        assert hasattr(native.release_hook, "py_get_release_hook_types")
    
    def test_module_has_create_function(self):
        """release_hook module should have py_create_release_hook function."""
        assert hasattr(native.release_hook, "py_create_release_hook")
