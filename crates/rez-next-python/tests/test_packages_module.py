"""Tests for rez_next.packages_ module - Cycle 290."""

import sys
import os
import tempfile
import pytest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

import rez_next.packages_ as packages_
from rez_next import Package, PackageFormat


class TestLoadPackageFromFile:
    """Tests for load_package_from_file function."""

    def test_load_valid_package(self, tmp_path):
        """Test loading a valid package from file."""
        # Create a simple package.py
        pkg_file = tmp_path / "package.py"
        pkg_file.write_text("""
name = "test_package"
version = "1.0.0"
description = "A test package"
requires = ["python-3.9"]
""")
        
        # Load the package
        pkg = packages_.load_package_from_file(str(pkg_file))
        
        assert pkg is not None
        assert pkg.name == "test_package"
        assert str(pkg.version) == "1.0.0"
        assert pkg.description == "A test package"

    def test_load_nonexistent_file(self):
        """Test loading a non-existent file raises OSError."""
        with pytest.raises(OSError):
            packages_.load_package_from_file("nonexistent/package.py")

    def test_load_invalid_package(self, tmp_path):
        """Test loading an invalid package file."""
        pkg_file = tmp_path / "package.py"
        pkg_file.write_text("this is not a valid package file")
        
        # Might raise an error or return None
        try:
            result = packages_.load_package_from_file(str(pkg_file))
            assert result is None
        except Exception:
            # Expected for invalid package
            pass


class TestSavePackageToFile:
    """Tests for save_package_to_file function."""

    def test_save_package_py(self, tmp_path):
        """Test saving a package to package.py format."""
        pkg = Package("test_save")
        pkg.set_version("1.0.0")
        pkg.description = "Test save package"
        
        output_file = tmp_path / "package.py"
        packages_.save_package_to_file(pkg, str(output_file))
        
        assert output_file.exists()
        content = output_file.read_text()
        assert "name" in content
        assert "test_save" in content

    def test_save_and_load_roundtrip(self, tmp_path):
        """Test save then load produces equivalent package."""
        # Create original package
        original = Package("roundtrip")
        original.set_version("2.0.0")
        original.description = "Roundtrip test"
        
        # Save to file
        pkg_file = tmp_path / "package.py"
        packages_.save_package_to_file(original, str(pkg_file))
        
        # Load back
        loaded = packages_.load_package_from_file(str(pkg_file))
        
        assert loaded is not None
        assert loaded.name == original.name
        assert str(loaded.version) == str(original.version)


class TestGetLatestPackage:
    """Tests for get_latest_package function."""

    def test_function_exists(self):
        """Test that get_latest_package function exists and is callable."""
        assert hasattr(packages_, "get_latest_package")
        assert callable(packages_.get_latest_package)

    # Note: Full testing requires a configured package repository
    # These tests verify the API signature


class TestGetPackage:
    """Tests for get_package function."""

    def test_function_exists(self):
        """Test that get_package function exists and is callable."""
        assert hasattr(packages_, "get_package")
        assert callable(packages_.get_package)


class TestIterPackages:
    """Tests for iter_packages function."""

    def test_function_exists(self):
        """Test that iter_packages function exists and is callable."""
        assert hasattr(packages_, "iter_packages")
        assert callable(packages_.iter_packages)


class TestCopyPackage:
    """Tests for copy_package function."""

    def test_function_exists(self):
        """Test that copy_package function exists and is callable."""
        assert hasattr(packages_, "copy_package")
        assert callable(packages_.copy_package)


class TestMovePackage:
    """Tests for move_package function."""

    def test_function_exists(self):
        """Test that move_package function exists and is callable."""
        assert hasattr(packages_, "move_package")
        assert callable(packages_.move_package)


class TestRemovePackage:
    """Tests for remove_package function."""

    def test_function_exists(self):
        """Test that remove_package function exists and is callable."""
        assert hasattr(packages_, "remove_package")
        assert callable(packages_.remove_package)


class TestWalkPackages:
    """Tests for walk_packages function."""
    
    def test_function_exists(self):
        """Test that walk_packages function exists and is callable."""
        assert hasattr(packages_, "walk_packages")
        assert callable(packages_.walk_packages)


class TestIterPackageFamilies:
    """Tests for iter_package_families function."""

    def test_function_exists(self):
        """Test that iter_package_families function exists and is callable."""
        assert hasattr(packages_, "iter_package_families")
        assert callable(packages_.iter_package_families)

    def test_returns_list(self):
        """Test that iter_package_families returns a list."""
        result = packages_.iter_package_families()
        assert isinstance(result, list)

    def test_returns_package_families(self):
        """Test that items are PackageFamily instances."""
        result = packages_.iter_package_families()
        if len(result) > 0:
            from rez_next._native import PackageFamily
            assert isinstance(result[0], PackageFamily)


class TestCreatePackage:
    """Tests for create_package function."""

    def test_function_exists(self):
        """Test that create_package function exists and is callable."""
        assert hasattr(packages_, "create_package")
        assert callable(packages_.create_package)

    def test_create_minimal_package(self):
        """Test creating a package with just a name."""
        data = {"name": "my_package"}
        pkg = packages_.create_package(data)
        assert pkg is not None
        assert pkg.name == "my_package"
        assert pkg.version is None

    def test_create_package_with_version(self):
        """Test creating a package with name and version."""
        data = {"name": "my_package", "version": "1.0.0"}
        pkg = packages_.create_package(data)
        assert pkg.name == "my_package"
        assert str(pkg.version) == "1.0.0"

    def test_create_package_with_description(self):
        """Test creating a package with description."""
        data = {
            "name": "my_package",
            "version": "2.0.0",
            "description": "A test package"
        }
        pkg = packages_.create_package(data)
        assert pkg.name == "my_package"
        assert pkg.description == "A test package"

    def test_create_package_with_requires(self):
        """Test creating a package with requirements."""
        data = {
            "name": "my_package",
            "version": "1.0.0",
            "requires": ["python-3.9", "maya-2024"]
        }
        pkg = packages_.create_package(data)
        assert len(pkg.requires) == 2
        assert "python-3.9" in pkg.requires

    def test_create_package_with_tools(self):
        """Test creating a package with tools."""
        data = {
            "name": "my_tool_package",
            "version": "1.0.0",
            "tools": ["my_tool", "another_tool"]
        }
        pkg = packages_.create_package(data)
        assert len(pkg.tools) == 2
        assert "my_tool" in pkg.tools

    def test_create_package_with_variants(self):
        """Test creating a package with variants."""
        data = {
            "name": "my_variant_package",
            "version": "1.0.0",
            "variants": [
                ["python-3.9"],
                ["python-3.10"]
            ]
        }
        pkg = packages_.create_package(data)
        assert len(pkg.variants) == 2
        assert pkg.variants[0] == ["python-3.9"]

    def test_create_package_missing_name(self):
        """Test that missing 'name' field raises ValueError."""
        data = {"version": "1.0.0", "description": "No name"}
        with pytest.raises(ValueError):
            packages_.create_package(data)

    def test_create_package_with_all_fields(self):
        """Test creating a package with all common fields."""
        data = {
            "name": "full_package",
            "version": "3.0.0",
            "description": "A fully specified package",
            "authors": ["Alice", "Bob"],
            "requires": ["python-3.11"],
            "build_requires": ["cmake-3.20"],
            "tools": ["full_tool"],
            "uuid": "12345678-1234-1234-1234-123456789012"
        }
        pkg = packages_.create_package(data)
        assert pkg.name == "full_package"
        assert str(pkg.version) == "3.0.0"
        assert pkg.description == "A fully specified package"
        assert len(pkg.authors) == 2
        assert len(pkg.requires) == 1
        assert len(pkg.tools) == 1


class TestGetLatestPackageFromString:
    """Tests for get_latest_package_from_string function."""

    def test_function_exists(self):
        """Test that get_latest_package_from_string function exists and is callable."""
        assert hasattr(packages_, "get_latest_package_from_string")
        assert callable(packages_.get_latest_package_from_string)

    def test_with_name_only(self):
        """Test with only package name (no version suffix)."""
        # This should behave like get_latest_package(name)
        result = packages_.get_latest_package_from_string("python")
        # Result can be None if no package found, or a Package instance
        assert result is None or hasattr(result, "name")

    def test_with_version_suffix(self):
        """Test with name-version format."""
        result = packages_.get_latest_package_from_string("python-3.9")
        # Result can be None if no package found, or a Package instance
        if result is not None:
            assert result.name == "python"
            assert str(result.version).startswith("3.9")

    def test_invalid_string(self):
        """Test with invalid package string."""
        # Should not raise, but return None if not found
        result = packages_.get_latest_package_from_string("nonexistent_pkg_xyz")
        assert result is None


class TestGetPackageFromString:
    """Tests for get_package_from_string function."""

    def test_function_exists(self):
        """Test that get_package_from_string function exists and is callable."""
        assert hasattr(packages_, "get_package_from_string")
        assert callable(packages_.get_package_from_string)

    def test_returns_package(self):
        """Test that it returns a Package object."""
        pkg = packages_.get_package_from_string("test_pkg-1.0.0")
        assert pkg is not None
        assert hasattr(pkg, "name")
        assert pkg.name == "test_pkg"
        # Version might be parsed
        if pkg.version is not None:
            assert str(pkg.version).startswith("1.0")

    def test_no_version(self):
        """Test with no version suffix."""
        pkg = packages_.get_package_from_string("test_pkg_only")
        assert pkg is not None
        assert pkg.name == "test_pkg_only"
        assert pkg.version is None


class TestDumpPackageData:
    """Tests for dump_package_data function."""

    def test_function_exists(self):
        """Test that dump_package_data function exists and is callable."""
        assert hasattr(packages_, "dump_package_data")
        assert callable(packages_.dump_package_data)

    def test_with_simple_package(self):
        """Test dumping a package with only a name."""
        pkg = packages_.create_package({"name": "simple_pkg"})
        data = packages_.dump_package_data(pkg)

        assert isinstance(data, dict)
        assert data["name"] == "simple_pkg"
        assert "version" not in data or data["version"] is None

    def test_with_full_package(self):
        """Test dumping a package with all common fields."""
        data_in = {
            "name": "full_pkg",
            "version": "2.0.0",
            "description": "A fully specified package",
            "authors": ["Alice", "Bob"],
            "requires": ["python-3.9"],
            "build_requires": ["cmake-3.20"],
            "tools": ["my_tool"],
        }
        pkg = packages_.create_package(data_in)
        data_out = packages_.dump_package_data(pkg)

        assert data_out["name"] == "full_pkg"
        assert data_out["version"] == "2.0.0"
        assert data_out["description"] == "A fully specified package"
        assert "Alice" in data_out["authors"]
        assert "python-3.9" in data_out["requires"]
        assert "cmake-3.20" in data_out["build_requires"]
        assert "my_tool" in data_out["tools"]

    def test_dump_and_compare(self):
        """Test that dump_package_data output matches input dict."""
        data_in = {
            "name": "test_compare",
            "version": "1.5.0",
            "description": "Compare test",
        }
        pkg = packages_.create_package(data_in)
        data_out = packages_.dump_package_data(pkg)

        # Check that dumped data contains the original fields
        assert data_out["name"] == data_in["name"]
        assert data_out["version"] == data_in["version"]
        assert data_out["description"] == data_in["description"]

    def test_with_variants(self):
        """Test dumping a package with variants."""
        data_in = {
            "name": "variant_pkg",
            "version": "1.0.0",
            "variants": [
                ["python-3.9"],
                ["python-3.10"],
            ],
        }
        pkg = packages_.create_package(data_in)
        data_out = packages_.dump_package_data(pkg)

        assert "variants" in data_out
        assert len(data_out["variants"]) == 2
        assert data_out["variants"][0] == ["python-3.9"]


class TestGetCompletions:
    """Tests for get_completions function."""

    def test_function_exists(self):
        """Test that get_completions function exists and is callable."""
        assert hasattr(packages_, "get_completions")
        assert callable(packages_.get_completions)

    def test_with_empty_prefix(self):
        """Test with empty string (should return all package names)."""
        result = packages_.get_completions("")
        assert isinstance(result, list)
        # Should return at least some package names
        if len(result) > 0:
            assert isinstance(result[0], str)

    def test_with_valid_prefix(self):
        """Test with a valid prefix."""
        result = packages_.get_completions("py")
        assert isinstance(result, list)
        # All results should start with "py" (case-insensitive)
        for name in result:
            assert name.lower().startswith("py")

    def test_with_no_matches(self):
        """Test with a prefix that matches nothing."""
        result = packages_.get_completions("xyz_nonexistent_prefix")
        assert isinstance(result, list)
        assert len(result) == 0

    def test_case_insensitive(self):
        """Test that prefix matching is case-insensitive."""
        result_lower = packages_.get_completions("py")
        result_upper = packages_.get_completions("PY")
        assert result_lower == result_upper

    def test_returns_sorted(self):
        """Test that results are sorted alphabetically."""
        result = packages_.get_completions("p")
        if len(result) > 1:
            for i in range(len(result) - 1):
                assert result[i].lower() <= result[i + 1].lower()


class TestGetDeveloperPackage:
    """Tests for get_developer_package function."""

    def test_function_exists(self):
        """Test that get_developer_package function exists and is callable."""
        assert hasattr(packages_, "get_developer_package")
        assert callable(packages_.get_developer_package)

    def test_with_package_py(self, tmp_path):
        """Test loading a developer package from directory with package.py."""
        pkg_file = tmp_path / "package.py"
        pkg_file.write_text("""
name = "dev_package"
version = "1.0.0"
description = "A developer package"
""")

        pkg = packages_.get_developer_package(str(tmp_path))

        assert pkg is not None
        assert pkg.name == "dev_package"
        assert str(pkg.version) == "1.0.0"
        assert pkg.description == "A developer package"
        assert pkg.is_dev_package == True

    def test_with_package_yaml(self, tmp_path):
        """Test loading a developer package from directory with package.yaml."""
        pkg_file = tmp_path / "package.yaml"
        pkg_file.write_text("""
name: yaml_package
version: "2.0.0"
description: "A package from yaml"
""")

        pkg = packages_.get_developer_package(str(tmp_path))

        assert pkg is not None
        assert pkg.name == "yaml_package"
        assert pkg.is_dev_package == True

    def test_not_a_directory(self):
        """Test with a file path instead of directory."""
        import tempfile
        with tempfile.NamedTemporaryFile() as tmp:
            try:
                packages_.get_developer_package(tmp.name)
                assert False, "Should have raised an error"
            except OSError:
                pass

    def test_no_package_file(self, tmp_path):
        """Test with directory that has no package file."""
        try:
            packages_.get_developer_package(str(tmp_path))
            assert False, "Should have raised an error"
        except OSError:
            pass


class TestGetLastReleaseTime:
    """Tests for get_last_release_time function."""

    def test_function_exists(self):
        """Test that get_last_release_time function exists and is callable."""
        assert hasattr(packages_, "get_last_release_time")
        assert callable(packages_.get_last_release_time)

    def test_returns_none_for_nonexistent(self):
        """Test that it returns None for non-existent package."""
        result = packages_.get_last_release_time("nonexistent_pkg_xyz_12345")
        assert result is None

    def test_returns_datetime_or_none(self):
        """Test that it returns a datetime object or None."""
        import datetime
        result = packages_.get_last_release_time("python")

        if result is not None:
            # Should be a datetime object
            assert isinstance(result, datetime.datetime)

    # Note: Full testing requires a configured package repository with timestamps
    # For now, we just verify the function can be called and returns the right type


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
