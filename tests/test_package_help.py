"""
Tests for PackageHelp and HelpSection classes.

These tests verify that the PackageHelp functionality works correctly.
"""

import pytest
import rez_next as rez
from rez_next._native import package_help


class TestHelpSection:
    """Tests for HelpSection class."""
    
    def test_create_help_section(self):
        """Test creating a HelpSection object."""
        section = package_help.HelpSection("Documentation", "https://example.com")
        assert section.name == "Documentation"
        assert section.uri == "https://example.com"
    
    def test_set_name(self):
        """Test setting the name of a HelpSection."""
        section = package_help.HelpSection("Documentation", "https://example.com")
        section.name = "Help"
        assert section.name == "Help"
    
    def test_set_uri(self):
        """Test setting the URI of a HelpSection."""
        section = package_help.HelpSection("Documentation", "https://example.com")
        section.uri = "https://newexample.com"
        assert section.uri == "https://newexample.com"
    
    def test_repr(self):
        """Test string representation of HelpSection."""
        section = package_help.HelpSection("Documentation", "https://example.com")
        repr_str = repr(section)
        assert "Documentation" in repr_str
        assert "https://example.com" in repr_str


class TestPackageHelp:
    """Tests for PackageHelp class."""
    
    def test_create_package_help(self):
        """Test creating a PackageHelp object."""
        ph = package_help.PackageHelp("mypackage", None, [])
        assert ph.success is False  # No help found
        assert len(ph.sections) == 0
    
    def test_success_property(self):
        """Test success property."""
        ph = package_help.PackageHelp("mypackage", None, [])
        assert ph.success is False
    
    def test_sections_property(self):
        """Test sections property."""
        ph = package_help.PackageHelp("mypackage", None, [])
        assert isinstance(ph.sections, list)
        assert len(ph.sections) == 0
    
    def test_print_info(self, capsys):
        """Test print_info method."""
        ph = package_help.PackageHelp("mypackage", None, [])
        ph.print_info()
        captured = capsys.readouterr()
        # No output when no sections
        assert captured.out == ""


class TestPackageHelpIntegration:
    """Integration tests for PackageHelp with real packages."""
    
    @pytest.fixture
    def sample_packages(self):
        """Create sample packages for testing."""
        # This is a mock - in real usage, you would pass actual Package objects
        return []
    
    def test_no_help_found(self, sample_packages):
        """Test when no help is found."""
        ph = package_help.PackageHelp("nonexistent", None, sample_packages)
        assert ph.success is False
        assert len(ph.sections) == 0
    
    def test_with_version_range(self, sample_packages):
        """Test with version range."""
        ph = package_help.PackageHelp("mypackage", ">=1.0", sample_packages)
        assert ph.success is False  # No packages provided


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
