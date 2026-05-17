"""
Tests for Package.to_dict() and to_package_py() methods, and setters.
"""
import rez_next as rez
import pytest


class TestToDict:
    """Test PyPackage.to_dict() method."""

    def test_empty_package(self):
        """Test to_dict() for package with only name."""
        pkg = rez.Package("test_package")
        d = pkg.to_dict()
        assert d["name"] == "test_package"
        assert len(d) == 1

    def test_package_with_version(self):
        """Test to_dict() includes version."""
        pkg = rez.Package("test")
        pkg.set_version("1.2.3")
        d = pkg.to_dict()
        assert d["name"] == "test"
        assert d["version"] == "1.2.3"

    def test_package_with_all_fields(self):
        """Test to_dict() with all common fields."""
        pkg = rez.Package("my_package")
        pkg.set_version("2.0.0")
        pkg.description = "Test package"
        pkg.authors = ["Author 1", "Author 2"]
        pkg.requires = ["python-3.9", "numpy"]
        pkg.build_requires = ["cmake"]
        pkg.tools = ["my_tool"]

        d = pkg.to_dict()
        assert d["name"] == "my_package"
        assert d["version"] == "2.0.0"
        assert d["description"] == "Test package"
        assert len(d["authors"]) == 2
        assert len(d["requires"]) == 2
        assert len(d["build_requires"]) == 1
        assert len(d["tools"]) == 1


class TestToPackagePy:
    """Test PyPackage.to_package_py() method."""

    def test_empty_package(self):
        """Test to_package_py() for package with only name."""
        pkg = rez.Package("test_package")
        result = pkg.to_package_py()
        assert "# -*- coding: utf-8 -*-" in result
        assert 'name = "test_package"' in result

    def test_package_with_version(self):
        """Test to_package_py() includes version."""
        pkg = rez.Package("test")
        pkg.set_version("1.2.3")
        result = pkg.to_package_py()
        assert 'version = "1.2.3"' in result

    def test_description_short(self):
        """Test short description (single line)."""
        pkg = rez.Package("test")
        pkg.description = "Short desc"
        result = pkg.to_package_py()
        assert 'description = "Short desc"' in result

    def test_description_long(self):
        """Test long description (triple quotes)."""
        pkg = rez.Package("test")
        pkg.description = "A" * 50  # > 40 chars
        result = pkg.to_package_py()
        assert 'description = """' in result
        assert 'A' * 50 in result

    def test_authors_single(self):
        """Test single author formatting."""
        pkg = rez.Package("test")
        pkg.authors = ["Author 1"]
        result = pkg.to_package_py()
        assert 'authors = ["Author 1"]' in result

    def test_authors_multiple(self):
        """Test multiple authors formatting."""
        pkg = rez.Package("test")
        pkg.authors = ["Author 1", "Author 2"]
        result = pkg.to_package_py()
        assert "authors = [" in result
        assert '"Author 1",' in result
        assert '"Author 2"' in result

    def test_requires(self):
        """Test requires formatting."""
        pkg = rez.Package("test")
        pkg.requires = ["python-3.9", "numpy"]
        result = pkg.to_package_py()
        assert "requires = [" in result
        assert '"python-3.9",' in result

    def test_variants(self):
        """Test variants formatting."""
        pkg = rez.Package("test")
        pkg.variants = [["os-Windows", "os-Linux"], ["maya-2024"]]
        result = pkg.to_package_py()
        assert "variants = [" in result
        assert '"os-Windows",' in result
        assert '"maya-2024"' in result

    def test_tools(self):
        """Test tools formatting."""
        pkg = rez.Package("test")
        pkg.tools = ["tool1", "tool2"]
        result = pkg.to_package_py()
        assert "tools = [" in result

    def test_uuid(self):
        """Test uuid in output."""
        pkg = rez.Package("test")
        pkg.uuid = "12345678-1234-1234-1234-123456789012"
        result = pkg.to_package_py()
        assert 'uuid = "12345678-1234-1234-1234-123456789012"' in result
        # Verify setter works
        pkg.uuid = "new-uuid-1234"
        assert pkg.uuid == "new-uuid-1234"

    def test_complete_package(self):
        """Test a complete package.py output."""
        pkg = rez.Package("my_package")
        pkg.set_version("1.2.3")
        pkg.description = "A test package"
        pkg.authors = ["Test Author"]
        pkg.requires = ["python-3.9+"]
        pkg.build_requires = ["cmake-3.20"]
        pkg.tools = ["my_tool"]
        pkg.uuid = "test-uuid-1234"

        result = pkg.to_package_py()

        # Check all fields are present
        assert "# -*- coding: utf-8 -*-" in result
        assert 'name = "my_package"' in result
        assert 'version = "1.2.3"' in result
        assert 'description = "A test package"' in result
        assert "authors = [" in result
        assert "requires = [" in result
        assert "build_requires = [" in result
        assert "tools = [" in result
        assert 'uuid = "test-uuid-1234"' in result


class TestSetters:
    """Test PyPackage property setters."""

    def test_set_description(self):
        """Test description setter."""
        pkg = rez.Package("test")
        pkg.description = "New description"
        assert pkg.description == "New description"

    def test_set_authors(self):
        """Test authors setter."""
        pkg = rez.Package("test")
        pkg.authors = ["Author 1", "Author 2"]
        assert len(pkg.authors) == 2
        assert pkg.authors[0] == "Author 1"

    def test_set_requires(self):
        """Test requires setter."""
        pkg = rez.Package("test")
        pkg.requires = ["python-3.9", "numpy"]
        assert len(pkg.requires) == 2
        assert pkg.requires[0] == "python-3.9"

    def test_set_build_requires(self):
        """Test build_requires setter."""
        pkg = rez.Package("test")
        pkg.build_requires = ["cmake"]
        assert len(pkg.build_requires) == 1

    def test_set_private_build_requires(self):
        """Test private_build_requires setter."""
        pkg = rez.Package("test")
        pkg.private_build_requires = ["internal_tool"]
        assert len(pkg.private_build_requires) == 1

    def test_set_variants(self):
        """Test variants setter."""
        pkg = rez.Package("test")
        pkg.variants = [["os-Windows"], ["os-Linux"]]
        assert len(pkg.variants) == 2

    def test_set_tools(self):
        """Test tools setter."""
        pkg = rez.Package("test")
        pkg.tools = ["tool1", "tool2"]
        assert len(pkg.tools) == 2

    def test_set_commands(self):
        """Test commands setter."""
        pkg = rez.Package("test")
        pkg.commands = "def commands():\n    pass"
        assert pkg.commands is not None

    def test_set_uuid(self):
        """Test uuid setter."""
        pkg = rez.Package("test")
        pkg.uuid = "test-uuid"
        assert pkg.uuid == "test-uuid"

    def test_set_cachable(self):
        """Test cachable setter."""
        pkg = rez.Package("test")
        pkg.cachable = True
        assert pkg.cachable is True

    def test_set_relocatable(self):
        """Test relocatable setter."""
        pkg = rez.Package("test")
        pkg.relocatable = False
        assert pkg.relocatable is False

    def test_set_is_dev_package(self):
        """Test is_dev_package setter."""
        pkg = rez.Package("test")
        pkg.is_dev_package = True
        assert pkg.is_dev_package is True


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
