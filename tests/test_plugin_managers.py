"""Tests for rez_next.plugin_managers module."""

import pytest
from rez_next.plugin_managers import (
    RezPluginManager,
    RezPluginType,
    LazySingleton,
    extend_path,
    ShellPluginType,
    ReleaseVCSPluginType,
    ReleaseHookPluginType,
    BuildSystemPluginType,
    PackageRepositoryPluginType,
    BuildProcessPluginType,
    CommandPluginType,
    plugin_manager,
)


class TestLazySingleton:
    def test_lazy_instantiation(self):
        calls = []

        class MyClass:
            def __init__(self):
                calls.append("init")

        ls = LazySingleton(MyClass)
        assert len(calls) == 0  # not yet instantiated
        instance = ls()
        assert len(calls) == 1
        assert ls() is instance  # same instance returned
        assert len(calls) == 1  # no new instantiation


class TestExtendPath:
    def test_non_list_returns_unchanged(self):
        result = extend_path("string_path", "name")
        assert result == "string_path"

    def test_list_with_missing_dirs(self):
        result = extend_path(["/nonexistent/path"], "some.package")
        assert result == ["/nonexistent/path"]  # unchanged because path missing


class TestRezPluginType:
    def test_type_name_required(self):
        with pytest.raises(TypeError):
            RezPluginType()

    def test_concrete_type(self):
        pt = ShellPluginType()
        assert pt.type_name == "shell"
        assert pt.pretty_type_name == "shell"
        assert isinstance(pt.plugin_classes, dict)
        assert isinstance(pt.failed_plugins, dict)

    def test_get_summary(self):
        summary = plugin_manager.get_summary_string()
        assert isinstance(summary, str)
        assert "shell" in summary or "(no plugins registered)" in summary


class TestPluginTypeSubclasses:
    def test_shell_plugin_type(self):
        pt = ShellPluginType()
        assert pt.type_name == "shell"

    def test_release_vcs_plugin_type(self):
        pt = ReleaseVCSPluginType()
        assert pt.type_name == "release_vcs"

    def test_release_hook_plugin_type(self):
        pt = ReleaseHookPluginType()
        assert pt.type_name == "release_hook"

    def test_build_system_plugin_type(self):
        pt = BuildSystemPluginType()
        assert pt.type_name == "build_system"

    def test_package_repository_plugin_type(self):
        pt = PackageRepositoryPluginType()
        assert pt.type_name == "package_repository"

    def test_build_process_plugin_type(self):
        pt = BuildProcessPluginType()
        assert pt.type_name == "build_process"

    def test_command_plugin_type(self):
        pt = CommandPluginType()
        assert pt.type_name == "command"


class TestPluginManager:
    def test_singleton(self):
        assert plugin_manager is not None
        assert isinstance(plugin_manager, RezPluginManager)

    def test_get_plugin_types(self):
        types = plugin_manager.get_plugin_types()
        assert isinstance(types, list)
        assert "shell" in types
        assert "build_system" in types
        assert "release_vcs" in types
        assert "package_repository" in types
        assert "release_hook" in types
        assert "build_process" in types
        assert "command" in types

    def test_unknown_plugin_type_raises(self):
        from rez_next.exceptions import RezPluginError
        with pytest.raises(RezPluginError):
            plugin_manager.get_plugins("nonexistent")

    def test_unknown_plugin_class_raises(self):
        from rez_next.exceptions import RezPluginError
        with pytest.raises(RezPluginError):
            plugin_manager.get_plugin_class("shell", "nonexistent")
