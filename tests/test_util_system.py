"""
Tests for rez_next.util system functions.

Aligned with Rez's system.py interface.
"""

import pytest
import rez_next as rez
from rez_next.util import (
    get_hostname,
    get_username,
    get_home_directory,
    get_fqdn,
    get_domain,
    which,
)


class TestSystemFunctions:
    """Test system information functions."""

    def test_get_hostname(self):
        """Test get_hostname() returns a non-empty string."""
        hostname = get_hostname()
        assert isinstance(hostname, str)
        assert len(hostname) > 0

    def test_get_username(self):
        """Test get_username() returns a non-empty string."""
        username = get_username()
        assert isinstance(username, str)
        assert len(username) > 0

    def test_get_home_directory(self):
        """Test get_home_directory() returns a path string or None."""
        home = get_home_directory()
        # Home directory should be a string or None
        assert home is None or isinstance(home, str)
        if home is not None:
            assert len(home) > 0

    def test_get_fqdn(self):
        """Test get_fqdn() returns a non-empty string."""
        fqdn = get_fqdn()
        assert isinstance(fqdn, str)
        assert len(fqdn) > 0

    def test_get_domain(self):
        """Test get_domain() returns a string."""
        domain = get_domain()
        assert isinstance(domain, str)
        # Domain might be empty if FQDN doesn't contain a dot
        # Just ensure it doesn't raise an exception


class TestWhichFunction:
    """Test rez_next.util.which() — upstream-compatible wrapper."""

    def test_which_finds_existing_command(self):
        """Test which() returns a path for an existing command."""
        result = which("python")
        assert result is not None
        assert "python" in result.lower()

    def test_which_returns_none_for_missing(self):
        """Test which() returns None for a non-existent command."""
        result = which("nonexistent_cmd_xyz_123")
        assert result is None

    def test_which_multi_args_returns_first_found(self):
        """Test which(*programs) accepts multiple args and returns the first match."""
        result = which("nonexistent_cmd_xyz_123", "python")
        assert result is not None
        assert "python" in result.lower()

    def test_which_all_missing_returns_none(self):
        """Test which() returns None when none of the programs exist."""
        result = which(
            "nonexistent_cmd_xyz_123",
            "another_missing_prog_456",
        )
        assert result is None

    def test_which_signature_matches_upstream(self):
        """Test which() has the upstream-compatible signature."""
        import inspect
        sig = inspect.signature(which)
        # Should accept *args style
        assert any(
            p.kind == inspect.Parameter.VAR_POSITIONAL
            for p in sig.parameters.values()
        )


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
