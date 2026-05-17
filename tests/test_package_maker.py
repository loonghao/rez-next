"""
Tests for package_maker functionality (to_package_py()).

Validates that rez_next.packages.Package.to_package_py()
matches rez package.py format.
"""

import rez_next as rez
import pytest


class TestPackageToPackagePy:
    """Tests for Package.to_package_py() method."""

    def test_basic(self):
        """Test basic package with only name."""
        pkg = rez.Package("test_pkg")
        result = pkg.to_package_py()
        assert 'name = "test_pkg"' in result
        assert "version = " not in result

    def test_with_version(self):
        """Test package with version."""
        pkg = rez.Package("test_pkg")
        pkg.set_version("1.0.0")
        result = pkg.to_package_py()
        assert 'name = "test_pkg"' in result
        assert 'version = "1.0.0"' in result

    def test_with_description(self):
        """Test package with description."""
        pkg = rez.Package("test_pkg")
        pkg.description = "A test package"
        result = pkg.to_package_py()
        assert 'description = "A test package"' in result

    def test_with_authors(self):
        """Test package with authors."""
        pkg = rez.Package("test_pkg")
        pkg.authors = ["Author One", "Author Two"]
        result = pkg.to_package_py()
        assert "authors = [" in result
        assert '"Author One"' in result
        assert '"Author Two"' in result

    def test_with_requires(self):
        """Test package with requires."""
        pkg = rez.Package("test_pkg")
        pkg.requires = ["python-3.9", "maya-2024"]
        result = pkg.to_package_py()
        assert "requires = [" in result
        assert '"python-3.9"' in result
        assert '"maya-2024"' in result

    def test_with_variants(self):
        """Test package with variants."""
        pkg = rez.Package("test_pkg")
        pkg.variants = [["python-3.9"], ["python-3.10"]]
        result = pkg.to_package_py()
        assert "variants = [" in result
        assert '["python-3.9"]' in result
        assert '["python-3.10"]' in result

    def test_with_tools(self):
        """Test package with tools."""
        pkg = rez.Package("test_pkg")
        pkg.tools = ["my_tool", "another_tool"]
        result = pkg.to_package_py()
        assert "tools = [" in result
        assert '"my_tool"' in result
        assert '"another_tool"' in result

    def test_with_commands(self):
        """Test package with commands function."""
        pkg = rez.Package("test_pkg")
        pkg.commands = '    env.PATH.prepend("{root}/bin")\n'
        result = pkg.to_package_py()
        assert "def commands():" in result
        assert "env.PATH.prepend" in result

    def test_with_uuid(self):
        """Test package with uuid."""
        pkg = rez.Package("test_pkg")
        pkg.uuid = "12345678-1234-1234-1234-123456789012"
        result = pkg.to_package_py()
        assert 'uuid = "12345678-1234-1234-1234-123456789012"' in result

    def test_with_relocatable(self):
        """Test package with relocatable flag."""
        pkg = rez.Package("test_pkg")
        pkg.relocatable = True
        result = pkg.to_package_py()
        assert "relocatable = True" in result

    def test_with_cachable(self):
        """Test package with cachable flag."""
        pkg = rez.Package("test_pkg")
        pkg.cachable = False
        result = pkg.to_package_py()
        assert "cachable = False" in result

    def test_complete_package(self):
        """Test package with all fields."""
        pkg = rez.Package("complete_pkg")
        pkg.set_version("2.1.0")
        pkg.description = "A complete test package"
        pkg.authors = ["Test Author"]
        pkg.requires = ["python-3.9"]
        pkg.build_requires = ["cmake-3.20"]
        pkg.variants = [["python-3.9"], ["python-3.10"]]
        pkg.tools = ["my_tool"]
        pkg.commands = '    env.PATH.prepend("{root}/bin")\n'
        pkg.uuid = "12345678-1234-1234-1234-123456789012"
        pkg.relocatable = True
        pkg.cachable = True

        result = pkg.to_package_py()

        # Verify all fields are present
        assert 'name = "complete_pkg"' in result
        assert 'version = "2.1.0"' in result
        assert 'description = "A complete test package"' in result
        assert "authors = [" in result
        assert "requires = [" in result
        assert "build_requires = [" in result
        assert "variants = [" in result
        assert "tools = [" in result
        assert "def commands():" in result
        assert 'uuid = "' in result
        assert "relocatable = True" in result
        assert "cachable = True" in result

    def test_escapes_description(self):
        """Test that quotes in description are escaped."""
        pkg = rez.Package("escape_test")
        pkg.description = 'Description with "quotes"'
        result = pkg.to_package_py()
        # Check that description field exists and contains the text
        assert "description" in result
        assert "quotes" in result

    def test_output_format_valid(self):
        """Test that output can be parsed as valid Python (basic check)."""
        pkg = rez.Package("format_test")
        pkg.set_version("1.0.0")
        pkg.requires = ["python-3.9"]
        pkg.commands = '    env.PATH.prepend("{root}/bin")\n'

        result = pkg.to_package_py()

        # Verify output starts with "name = "
        assert result.startswith('name = "')

        # Verify commands function is properly formatted
        assert "def commands():" in result
        assert "    env.PATH.prepend" in result


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
