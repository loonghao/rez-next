"""
Tests for rez_next.serialise_ module.

This module tests the package serialisation functionality,
including dump_package_data, dump_yaml, as_block_string,
dict_to_attributes_code, and package_key_order.
"""

import sys
import os
import tempfile
import json

import pytest
import rez_next
import rez_next.serialise_ as serialise


class TestSerialiseModule:
    """Test that the serialise_ module is accessible and has the right functions."""

    def test_module_accessible(self):
        """Test that rez_next.serialise_ is accessible."""
        assert hasattr(rez_next, "serialise_")
        assert serialise is not None

    def test_dump_package_data_exists(self):
        """Test that dump_package_data function exists."""
        assert hasattr(serialise, "dump_package_data")
        assert callable(serialise.dump_package_data)

    def test_dump_yaml_exists(self):
        """Test that dump_yaml function exists."""
        assert hasattr(serialise, "dump_yaml")
        assert callable(serialise.dump_yaml)

    def test_as_block_string_exists(self):
        """Test that as_block_string function exists."""
        assert hasattr(serialise, "as_block_string")
        assert callable(serialise.as_block_string)

    def test_dict_to_attributes_code_exists(self):
        """Test that dict_to_attributes_code function exists."""
        assert hasattr(serialise, "dict_to_attributes_code")
        assert callable(serialise.dict_to_attributes_code)

    def test_package_key_order_exists(self):
        """Test that package_key_order function exists."""
        assert hasattr(serialise, "package_key_order")
        assert callable(serialise.package_key_order)

    def test_file_format_exists(self):
        """Test that FileFormat class exists."""
        assert hasattr(serialise, "FileFormat")
        assert hasattr(serialise.FileFormat, "yaml")
        assert hasattr(serialise.FileFormat, "json")
        assert hasattr(serialise.FileFormat, "python")
        assert hasattr(serialise.FileFormat, "toml")


class TestDumpYaml:
    """Test dump_yaml function."""

    def test_dump_yaml_simple(self):
        """Test dump_yaml with simple data."""
        data = {"name": "test_package", "version": "1.0.0"}
        result = serialise.dump_yaml(data)
        assert "name:" in result
        assert "test_package" in result
        assert "version:" in result
        assert "1.0.0" in result

    def test_dump_yaml_roundtrip(self):
        """Test that dump_yaml output can be parsed as YAML."""
        import yaml
        data = {"name": "test_package", "version": "1.0.0", "requires": ["python-3.9"]}
        result = serialise.dump_yaml(data)
        parsed = yaml.safe_load(result)
        assert parsed["name"] == "test_package"
        assert parsed["version"] == "1.0.0"
        assert "python-3.9" in parsed["requires"]


class TestAsBlockString:
    """Test as_block_string function."""

    def test_as_block_string_single_line(self):
        """Test as_block_string with single line."""
        result = serialise.as_block_string("hello world", 0)
        assert "'hello world'" in result or '"hello world"' in result

    def test_as_block_string_multi_line(self):
        """Test as_block_string with multiple lines."""
        s = "line1\nline2\nline3"
        result = serialise.as_block_string(s, 4)
        assert result.startswith("|")
        assert "    line1" in result
        assert "    line2" in result
        assert "    line3" in result

    def test_as_block_string_empty(self):
        """Test as_block_string with empty string."""
        result = serialise.as_block_string("", 0)
        assert result == "''"


class TestDictToAttributesCode:
    """Test dict_to_attributes_code function."""

    def test_dict_to_attributes_code_simple(self):
        """Test dict_to_attributes_code with simple data."""
        data = {"name": "test_package", "version": "1.0.0"}
        result = serialise.dict_to_attributes_code(data)
        assert "test_package" in result
        assert "1.0.0" in result

    def test_dict_to_attributes_code_multiline(self):
        """Test dict_to_attributes_code with multiline description."""
        data = {
            "name": "test_package",
            "description": "This is a\nmultiline description"
        }
        result = serialise.dict_to_attributes_code(data)
        assert "test_package" in result
        # Should contain block string for multiline
        assert "|" in result or "'''" in result


class TestPackageKeyOrder:
    """Test package_key_order function."""

    def test_package_key_order(self):
        """Test that package_key_order returns the standard key order."""
        result = serialise.package_key_order()
        assert isinstance(result, list)
        assert len(result) > 0
        assert result[0] == "name"
        assert result[1] == "version"

    def test_package_key_order_contains_required_fields(self):
        """Test that package_key_order contains required fields."""
        result = serialise.package_key_order()
        assert "name" in result
        assert "version" in result
        assert "requires" in result


class TestDumpPackageData:
    """Test dump_package_data function."""

    def test_dump_package_data_yaml(self):
        """Test dump_package_data with YAML format."""
        data = {"name": "test_package", "version": "1.0.0"}
        with tempfile.NamedTemporaryFile(mode='w', suffix='.yaml', delete=False) as f:
            path = f.name
        try:
            serialise.dump_package_data(data, path, "yaml")
            assert os.path.exists(path)
            with open(path, 'r') as f:
                content = f.read()
                assert "name:" in content
                assert "test_package" in content
        finally:
            os.unlink(path)

    def test_dump_package_data_json(self):
        """Test dump_package_data with JSON format."""
        data = {"name": "test_package", "version": "1.0.0"}
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
            path = f.name
        try:
            serialise.dump_package_data(data, path, "json")
            assert os.path.exists(path)
            with open(path, 'r') as f:
                content = json.load(f)
                assert content["name"] == "test_package"
                assert content["version"] == "1.0.0"
        finally:
            os.unlink(path)

    def test_dump_package_data_python(self):
        """Test dump_package_data with Python format."""
        data = {"name": "test_package", "version": "1.0.0"}
        with tempfile.NamedTemporaryFile(mode='w', suffix='.py', delete=False) as f:
            path = f.name
        try:
            serialise.dump_package_data(data, path, "python")
            assert os.path.exists(path)
            with open(path, 'r') as f:
                content = f.read()
                assert "test_package" in content
                assert "1.0.0" in content
        finally:
            os.unlink(path)

    def test_dump_package_data_toml(self):
        """Test dump_package_data with TOML format."""
        data = {"name": "test_package", "version": "1.0.0"}
        with tempfile.NamedTemporaryFile(mode='w', suffix='.toml', delete=False) as f:
            path = f.name
        try:
            serialise.dump_package_data(data, path, "toml")
            assert os.path.exists(path)
            with open(path, 'r') as f:
                content = f.read()
                assert "name" in content
                assert "test_package" in content
        finally:
            os.unlink(path)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
