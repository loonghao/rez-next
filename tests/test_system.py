"""Tests for rez_next.system module.

Aligns with rez.system API:
- `from rez.system import system` — singleton System instance
- `system.platform` — current platform
- `system.arch` — current architecture
- `system.os` — current operating system
- `system.hostname` — machine hostname
"""
import pytest


class TestSystemModule:
    """Tests for the rez_next.system module."""

    def test_system_is_module(self):
        """system should be importable as a module."""
        import rez_next.system
        assert rez_next.system is not None

    def test_system_singleton(self):
        """from rez_next.system import system should give System instance."""
        from rez_next.system import system
        assert system is not None

    def test_system_platform(self):
        """system.platform should return a string."""
        from rez_next.system import system
        assert isinstance(system.platform, str)

    def test_system_arch(self):
        """system.arch should return a string."""
        from rez_next.system import system
        assert isinstance(system.arch, str)

    def test_system_os(self):
        """system.os should return a string."""
        from rez_next.system import system
        assert isinstance(system.os, str)

    def test_system_hostname(self):
        """system.hostname should return a string."""
        from rez_next.system import system
        assert isinstance(system.hostname, str)

    def test_system_rez_version(self):
        """system.rez_version should return a string."""
        from rez_next.system import system
        assert isinstance(system.rez_version, str)

    def test_module_level_platform(self):
        """Module-level platform attribute should be accessible."""
        import rez_next.system
        if hasattr(rez_next.system, 'platform'):
            assert isinstance(rez_next.system.platform, str)

    def test_multiple_imports_give_same_singleton(self):
        """Multiple imports of system should give the same instance."""
        from rez_next.system import system as s1
        from rez_next.system import system as s2
        assert s1 is s2
