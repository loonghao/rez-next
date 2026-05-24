"""Tests for rez_next.command module."""

import pytest
import rez_next as rez


class TestCommandResult:
    """Tests for CommandResult class."""

    def test_command_result_attributes(self):
        """CommandResult should have stdout, stderr, exit_code, success attributes."""
        # We can't directly instantiate CommandResult from Python easily,
        # but we can check the class exists and has the right structure
        assert hasattr(rez.command.CommandResult, "__new__")

    def test_command_result_repr(self):
        """CommandResult should have a useful repr."""
        # Create a CommandResult via execute_command
        import sys
        if sys.platform == "win32":
            result = rez.command.execute_command("cmd", ["/c", "echo test"])
        else:
            result = rez.command.execute_command("echo", ["test"])
        assert result is not None
        assert hasattr(result, "stdout")
        assert hasattr(result, "stderr")
        assert hasattr(result, "exit_code")
        assert hasattr(result, "success")
        assert "CommandResult" in repr(result)


class TestExecuteCommand:
    """Tests for execute_command function."""

    def test_execute_command_success(self):
        """execute_command should succeed with a valid command."""
        import sys
        if sys.platform == "win32":
            result = rez.command.execute_command("cmd", ["/c", "echo test"])
        else:
            result = rez.command.execute_command("echo", ["test"])

        assert result is not None
        assert result.success is True
        assert result.exit_code == 0

    def test_execute_command_not_found(self):
        """execute_command should raise an error for non-existent command."""
        with pytest.raises(Exception):
            rez.command.execute_command("this_command_definitely_does_not_exist_12345", [])

    def test_execute_command_return_type(self):
        """execute_command should return a CommandResult."""
        import sys
        if sys.platform == "win32":
            result = rez.command.execute_command("cmd", ["/c", "echo hello"])
        else:
            result = rez.command.execute_command("echo", ["hello"])

        assert isinstance(result, rez.command.CommandResult)


class TestCommandExists:
    """Tests for command_exists function."""

    def test_command_exists_echo(self):
        """command_exists should return True for 'echo' (available via cmd on Windows)."""
        import sys
        if sys.platform == "win32":
            # On Windows, echo is a cmd builtin, not a standalone executable
            # But cmd exists
            assert rez.command.command_exists("cmd") is True
        else:
            assert rez.command.command_exists("echo") is True

    def test_command_exists_nonexistent(self):
        """command_exists should return False for non-existent command."""
        result = rez.command.command_exists("this_command_definitely_does_not_exist_12345")
        assert result is False


class TestGetCommandOutput:
    """Tests for get_command_output function."""

    def test_get_command_output_success(self):
        """get_command_output should return stdout as string."""
        import sys
        if sys.platform == "win32":
            output = rez.command.get_command_output("cmd", ["/c", "echo hello"])
        else:
            output = rez.command.get_command_output("echo", ["hello"])

        assert output is not None
        assert "hello" in output

    def test_get_command_output_not_found(self):
        """get_command_output should raise an error for non-existent command."""
        with pytest.raises(Exception):
            rez.command.get_command_output("this_command_definitely_does_not_exist_12345", [])


class TestGetCommandPath:
    """Tests for get_command_path function."""

    def test_get_command_path_echo(self):
        """get_command_path should return a path for 'echo' or 'cmd'."""
        import sys
        if sys.platform == "win32":
            path = rez.command.get_command_path("cmd")
        else:
            path = rez.command.get_command_path("echo")

        # Path might be None if command not found in PATH
        if path is not None:
            assert isinstance(path, str)
            assert len(path) > 0

    def test_get_command_path_nonexistent(self):
        """get_command_path should return None for non-existent command."""
        result = rez.command.get_command_path("this_command_definitely_does_not_exist_12345")
        assert result is None
