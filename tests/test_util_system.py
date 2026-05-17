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


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
