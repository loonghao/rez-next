"""
Module-level tests for rez_next config and system API.
"""

import pytest

rez = pytest.importorskip(
    "rez_next",
    reason="rez_next not built — run: maturin develop --features extension-module",
)

VALID_PLATFORMS = ("linux", "windows", "osx")


class TestConfigModule:
    """rez_next.config submodule."""

    def test_import_Config(self):
        from rez_next.config import Config

        assert Config is not None

    def test_Config_instantiate(self):
        from rez_next.config import Config

        cfg = Config()
        assert cfg is not None

    def test_packages_path_is_list(self):
        cfg = rez.Config()
        assert isinstance(cfg.packages_path, list)

    def test_local_packages_path_is_str(self):
        cfg = rez.Config()
        assert isinstance(cfg.local_packages_path, str)
        assert len(cfg.local_packages_path) > 0

    def test_release_packages_path_is_str(self):
        cfg = rez.Config()
        assert isinstance(cfg.release_packages_path, str)

    def test_default_shell_is_str(self):
        cfg = rez.Config()
        assert isinstance(cfg.default_shell, str)

    def test_get_packages_path(self):
        cfg = rez.Config()
        result = cfg.get("packages_path", None)
        # May return list or None — both valid
        assert result is None or isinstance(result, list)

    def test_get_unknown_field_returns_default(self):
        cfg = rez.Config()
        default = "MY_DEFAULT"
        result = cfg.get("_nonexistent_field_xyz", default)
        assert result == default

    def test_config_singleton_via_module(self):
        """rez_next.config module has a config instance."""
        import rez_next.config as config_mod

        assert hasattr(config_mod, "Config")

    def test_top_level_config_not_none(self):
        """rez.config (top-level attribute) must not be None."""
        assert rez.config is not None


class TestSystemModule:
    """rez_next.system submodule."""

    def test_import_system_singleton(self):
        from rez_next.system import system

        assert system is not None

    def test_system_platform(self):
        from rez_next.system import system

        assert system.platform in VALID_PLATFORMS

    def test_system_arch_nonempty(self):
        from rez_next.system import system

        assert isinstance(system.arch, str)
        assert len(system.arch) > 0

    def test_system_os_nonempty(self):
        from rez_next.system import system

        assert isinstance(system.os, str)
        assert len(system.os) > 0

    def test_top_level_system_module_has_platform(self):
        """rez.system.platform accessible when system is a module."""
        import rez_next.system as sys_mod

        assert hasattr(sys_mod, "platform")
        assert sys_mod.platform in VALID_PLATFORMS

    def test_top_level_system_module_has_arch(self):
        import rez_next.system as sys_mod

        assert hasattr(sys_mod, "arch")
        assert isinstance(sys_mod.arch, str)

    def test_system_num_cpus(self):
        from rez_next.system import system

        assert isinstance(system.num_cpus, int)
        assert system.num_cpus >= 1

    def test_system_hostname(self):
        from rez_next.system import system

        assert isinstance(system.hostname, str)
