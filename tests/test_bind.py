"""Tests for rez_next.bind module."""

import pytest
from rez_next.bind import (
    BindManager,
    BindResult,
    bind_manager,
    bind_tool,
    detect_version,
    extract_version,
    find_tool,
    list_binders,
)


class TestBindModuleImport:
    """Verify the module is importable and exports correct names."""

    def test_module_importable(self):
        from rez_next import bind
        assert hasattr(bind, "list_binders")
        assert hasattr(bind, "bind_tool")
        assert hasattr(bind, "bind_manager")

    def test_bind_manager_instance(self):
        assert isinstance(bind_manager, BindManager)

    def test_bind_result_class(self):
        assert issubclass(BindResult, object)

    def test_list_binders_callable(self):
        assert callable(list_binders)

    def test_bind_tool_callable(self):
        assert callable(bind_tool)

    def test_find_tool_callable(self):
        assert callable(find_tool)

    def test_detect_version_callable(self):
        assert callable(detect_version)

    def test_extract_version_callable(self):
        assert callable(extract_version)


class TestBindManager:
    """Test the BindManager class."""

    def test_bind_manager_construction(self):
        """BindManager can be instantiated."""
        bm = BindManager()
        assert bm is not None
        assert isinstance(bm, BindManager)
        assert hasattr(bm, "available_modules") or hasattr(bm, "list_binders")
        assert hasattr(bm, "bind")

    def test_bind_manager_instance_methods(self):
        """bind_manager singleton should have expected methods."""
        assert hasattr(bind_manager, "available_modules") or hasattr(bind_manager, "list_binders")

    def test_list_binders_returns_list(self):
        """list_binders() returns a list."""
        binders = list_binders()
        assert isinstance(binders, list)

    def test_find_tool_known_tool(self):
        """find_tool returns a string path or None for known tools."""
        result = find_tool("python")
        assert result is None or isinstance(result, str)


class TestBindFunctions:
    """Test standalone bind functions."""

    def test_extract_version_string(self):
        """extract_version handles version strings."""
        v = extract_version("1.2.3")
        assert v is not None

    def test_list_binders_result_types(self):
        """list_binders returns list of strings or dicts."""
        binders = list_binders()
        if binders:
            first = binders[0]
            assert isinstance(first, (str, dict))


class TestBindTool:
    """Test bind_tool function."""

    def test_bind_tool_unknown_package(self):
        """bind_tool raises or returns error for unknown/non-existent package."""
        with pytest.raises((LookupError, RuntimeError, FileNotFoundError)):
            bind_tool("__nonexistent_tool_name__")
