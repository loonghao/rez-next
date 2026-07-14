"""Tests for rez_next.command module."""

import pytest
from rez_next.command import Command


class TestCommand:
    def test_name_raises(self):
        with pytest.raises(NotImplementedError):
            Command.name()

    def test_concrete_command(self):
        class MyCommand(Command):
            @classmethod
            def name(cls):
                return "my_cmd"

        cmd = MyCommand()
        assert cmd.name() == "my_cmd"
        assert hasattr(cmd, "settings")

    def test_settings_default(self):
        class MyCommand(Command):
            @classmethod
            def name(cls):
                return "my_cmd"

        cmd = MyCommand()
        assert cmd.settings is None or isinstance(cmd.settings, dict)
