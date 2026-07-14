"""Tests for rez_next.config module — rezconfig.py defaults."""

import os
import sys

import pytest

# The singleton `config = Config()` in __init__.py shadows the submodule.
# Use sys.modules to access the actual module for defaults testing.
import rez_next  # noqa: E402

_cfg_module = sys.modules.get("rez_next.config")
if _cfg_module is None:
    from importlib import import_module

    _cfg_module = import_module("rez_next.config")

# Singleton instance (for API compatibility tests)
rez_config_singleton = rez_next.config


class TestConfigDefaults:
    """Test all Rez config defaults are defined correctly."""

    # -- Paths ---------------------------------------------------------------
    def test_packages_path(self):
        assert _cfg_module.packages_path == [
            "~/packages",
            "~/.rez/packages/int",
            "~/.rez/packages/ext",
        ]

    def test_local_packages_path(self):
        assert _cfg_module.local_packages_path == "~/packages"

    def test_release_packages_path(self):
        assert _cfg_module.release_packages_path == "~/.rez/packages/int"

    def test_tmpdir(self):
        assert _cfg_module.tmpdir is None

    def test_context_tmpdir(self):
        assert _cfg_module.context_tmpdir is None

    def test_plugin_path(self):
        assert _cfg_module.plugin_path == []

    def test_bind_module_path(self):
        assert _cfg_module.bind_module_path == []

    # -- Caching -------------------------------------------------------------
    def test_resolve_caching(self):
        assert _cfg_module.resolve_caching is True

    def test_cache_package_files(self):
        assert _cfg_module.cache_package_files is True

    def test_cache_listdir(self):
        assert _cfg_module.cache_listdir is True

    def test_resource_caching_maxsize(self):
        assert _cfg_module.resource_caching_maxsize == -1

    def test_memcached_uri(self):
        assert _cfg_module.memcached_uri == []

    def test_default_relocatable(self):
        assert _cfg_module.default_relocatable is True

    def test_default_cachable(self):
        assert _cfg_module.default_cachable is False

    # -- Package Resolution --------------------------------------------------
    def test_implicit_packages(self):
        assert len(_cfg_module.implicit_packages) == 3
        assert "~platform=={system.platform}" in _cfg_module.implicit_packages

    def test_variant_select_mode(self):
        assert _cfg_module.variant_select_mode == "version_priority"

    def test_allow_unversioned(self):
        assert _cfg_module.allow_unversioned_packages is True

    # -- Environment Resolution ----------------------------------------------
    def test_default_shell(self):
        assert _cfg_module.default_shell == ""

    def test_suite_visibility(self):
        assert _cfg_module.suite_visibility == "always"

    def test_rez_tools_visibility(self):
        assert _cfg_module.rez_tools_visibility == "append"

    # -- Debugging -----------------------------------------------------------
    def test_catch_rex_errors(self):
        assert _cfg_module.catch_rex_errors is True

    def test_shell_error_truncate_cap(self):
        assert _cfg_module.shell_error_truncate_cap == 750

    # -- Build / Release -----------------------------------------------------
    def test_build_directory(self):
        assert _cfg_module.build_directory == "build"

    def test_build_thread_count(self):
        assert _cfg_module.build_thread_count == "physical_cores"

    def test_release_hooks(self):
        assert _cfg_module.release_hooks == []

    def test_default_build_process(self):
        assert _cfg_module.default_build_process == "local"

    # -- Suites --------------------------------------------------------------
    def test_suite_alias_prefix_char(self):
        assert _cfg_module.suite_alias_prefix_char == "+"

    # -- Appearance ----------------------------------------------------------
    def test_quiet(self):
        assert _cfg_module.quiet is False

    def test_show_progress(self):
        assert _cfg_module.show_progress is True

    def test_dot_image_format(self):
        assert _cfg_module.dot_image_format == "png"

    # -- Misc ----------------------------------------------------------------
    def test_max_package_changelog_chars(self):
        assert _cfg_module.max_package_changelog_chars == 65536

    def test_pip_extra_args(self):
        assert _cfg_module.pip_extra_args == []

    def test_pip_install_remaps(self):
        assert len(_cfg_module.pip_install_remaps) == 2

    # -- Colorization --------------------------------------------------------
    def test_color_enabled(self):
        assert _cfg_module.color_enabled == (os.name == "posix")

    def test_logging_colors(self):
        assert _cfg_module.critical_fore == "red"
        assert _cfg_module.warning_fore == "yellow"
        assert _cfg_module.info_fore == "green"
        assert _cfg_module.debug_fore == "blue"

    def test_context_colors(self):
        assert _cfg_module.local_fore == "green"
        assert _cfg_module.implicit_fore == "cyan"
        assert _cfg_module.alias_fore == "cyan"

    # -- GUI -----------------------------------------------------------------
    def test_gui_threads(self):
        assert _cfg_module.gui_threads is True


class TestConfigInstance:
    """Test the Config singleton instance API compatibility."""

    def test_config_get(self):
        assert rez_config_singleton.get("quiet") is False
        assert rez_config_singleton.get("nonexistent") is None
        assert rez_config_singleton.get("nonexistent", "fallback") == "fallback"

    def test_config_get_packages_path(self):
        result = rez_config_singleton.get("packages_path")
        assert isinstance(result, list)
        assert "~/packages" in result

    def test_config_get_caching(self):
        assert rez_config_singleton.get("resolve_caching") is True
        assert rez_config_singleton.get("cache_package_files") is True

    def test_config_get_nested(self):
        assert rez_config_singleton.get("build_directory") == "build"
        assert rez_config_singleton.get("variant_select_mode") == "version_priority"


class TestModuleLevelGet:
    """Test the module-level convenience function."""

    def test_module_get(self):
        from rez_next.config import get

        assert get("quiet") is False
        assert get("nonexistent") is None
        assert get("nonexistent", "default") == "default"

    def test_module_get_paths(self):
        from rez_next.config import get

        result = get("packages_path")
        assert isinstance(result, list)
        assert "~/packages" in result


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
