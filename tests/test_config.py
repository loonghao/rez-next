"""Tests for rez_next.config module."""

import pytest
import sys
from rez_next._native import config


class TestConfig:
    """Test Config class bindings."""

    def test_config_creation(self):
        """Test creating a new Config object."""
        cfg = config.Config()
        assert cfg is not None

    def test_config_repr(self):
        """Test Config string representation."""
        cfg = config.Config()
        result = repr(cfg)
        assert "Config" in result

    def test_config_contains_key(self):
        """Test checking if a key exists."""
        cfg = config.Config()
        # New config should not contain any keys
        assert not cfg.contains_key("nonexistent")

    def test_config_get_string_nonexistent(self):
        """Test getting a nonexistent string key."""
        cfg = config.Config()
        result = cfg.get_string("nonexistent")
        assert result is None


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
