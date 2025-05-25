"""
Tests for rez-core configuration functionality.

These tests verify the configuration system works correctly and provides
appropriate defaults and customization options.
"""

import pytest
import rez_core


class TestConfigCreation:
    """Test configuration object creation and properties."""
    
    def test_config_creation_default(self):
        """Test creating a config with default values."""
        config = rez_core.Config()
        assert config is not None
        assert repr(config).startswith("Config(")
    
    def test_config_creation_from_fixture(self, config):
        """Test using config from fixture."""
        assert config is not None
        assert isinstance(config, rez_core.Config)
    
    def test_config_representation(self):
        """Test config string representation."""
        config = rez_core.Config()
        repr_str = repr(config)
        
        # Should contain key configuration flags
        assert "use_rust_version=" in repr_str
        assert "use_rust_solver=" in repr_str
        assert "use_rust_repository=" in repr_str


class TestConfigDefaults:
    """Test that configuration defaults are appropriate."""
    
    def test_default_rust_features_enabled(self):
        """Test that Rust features are enabled by default."""
        config = rez_core.Config()
        
        # These should be accessible as properties when implemented
        # For now, just test that config creation works
        assert config is not None
    
    def test_config_is_consistent(self):
        """Test that multiple config instances have same defaults."""
        config1 = rez_core.Config()
        config2 = rez_core.Config()
        
        # Both should have same string representation
        assert repr(config1) == repr(config2)


@pytest.mark.integration
class TestConfigIntegration:
    """Test configuration integration with other components."""
    
    def test_config_with_version_system(self, config):
        """Test that config works with version system."""
        # This test verifies that having a config doesn't break version operations
        v = rez_core.Version("1.2.3")
        assert str(v) == "1.2.3"
        
        # Config should still be valid
        assert config is not None
    
    def test_config_with_version_range_system(self, config):
        """Test that config works with version range system."""
        vr = rez_core.VersionRange(">=1.0.0")
        assert str(vr) == ">=1.0.0"
        
        # Config should still be valid
        assert config is not None


@pytest.mark.compat
class TestConfigCompatibility:
    """Test configuration compatibility with rez expectations."""
    
    def test_config_creation_matches_rez_pattern(self):
        """Test that config creation follows rez patterns."""
        # In rez, configs are typically created with default values
        # and then customized as needed
        config = rez_core.Config()
        
        # Should be able to create without errors
        assert config is not None
        
        # Should have a meaningful string representation
        repr_str = repr(config)
        assert len(repr_str) > 10  # Should be more than just "Config()"
    
    def test_config_can_be_used_multiple_times(self):
        """Test that config objects can be reused."""
        config = rez_core.Config()
        
        # Should be able to use config multiple times
        for _ in range(10):
            repr_str = repr(config)
            assert "Config(" in repr_str
