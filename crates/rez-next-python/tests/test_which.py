"""Tests for which and which_all functions."""

import pytest
import os
import sys
import tempfile

# Add parent directory to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

import rez_next.util as util


class TestWhich:
    """Tests for which function."""

    def test_which_nonexistent(self):
        """Test that non-existent command returns None."""
        result = util.which("this_command_definitely_does_not_exist_12345")
        assert result is None

    def test_which_current_python(self):
        """Test that 'python' can be found in PATH."""
        result = util.which("python")
        # python might not be in PATH on all systems
        if result is not None:
            assert isinstance(result, str)
            assert os.path.isfile(result)

    @pytest.mark.skipif(sys.platform != "win32", reason="Windows only")
    def test_which_windows_exe(self):
        """Test finding .exe file on Windows."""
        result = util.which("cmd")
        assert result is not None
        assert result.endswith(".exe")

    @pytest.mark.skipif(sys.platform == "win32", reason="Unix only")
    def test_which_unix_common(self):
        """Test finding common Unix commands."""
        for cmd in ["ls", "cat", "echo"]:
            result = util.which(cmd)
            if result is not None:
                assert os.path.isfile(result)
                break


class TestWhichAll:
    """Tests for which_all function."""

    def test_which_all_nonexistent(self):
        """Test that non-existent command returns empty list."""
        results = util.which_all("this_command_definitely_does_not_exist_12345")
        assert isinstance(results, list)
        assert len(results) == 0

    def test_which_all_returns_list(self):
        """Test that which_all returns a list."""
        results = util.which_all("python")
        assert isinstance(results, list)


class TestIntegration:
    """Integration tests with temporary directory."""

    def test_which_finds_in_temp_path(self):
        """Test that which finds executable in temp directory added to PATH."""
        with tempfile.TemporaryDirectory() as tmpdir:
            # Create a fake executable
            fake_exe = os.path.join(tmpdir, "fake_cmd")
            if sys.platform == "win32":
                fake_exe += ".exe"
            with open(fake_exe, 'w') as f:
                f.write("# fake executable")

            # On Windows, just check that the file exists
            # (making it truly executable requires more setup)
            if sys.platform != "win32":
                os.chmod(fake_exe, 0o755)
                old_path = os.environ.get("PATH", "")
                os.environ["PATH"] = tmpdir + os.pathsep + old_path

                try:
                    result = util.which("fake_cmd")
                    assert result is not None
                    assert result == fake_exe
                finally:
                    os.environ["PATH"] = old_path


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
