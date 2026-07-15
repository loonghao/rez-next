"""Tests for rez_next.serialise_ module."""

import os
import tempfile

import rez_next.serialise_ as serialise


class TestDumpYaml:
    """Tests for dump_yaml function."""

    def test_simple_dict(self):
        """Test dump_yaml with a simple dict."""
        data = {"name": "test_package", "version": "1.0.0"}
        result = serialise.dump_yaml(data)
        assert "name:" in result
        assert "test_package" in result
        assert "version:" in result

    def test_nested_dict(self):
        """Test dump_yaml with nested dict."""
        data = {"name": "test", "requires": ["python-3.9", "maya-2024"]}
        result = serialise.dump_yaml(data)
        assert "requires:" in result


class TestAsBlockString:
    """Tests for as_block_string function."""

    def test_single_line(self):
        """Test as_block_string with single line."""
        result = serialise.as_block_string("hello world", 0)
        assert result == "'hello world'"

    def test_multi_line(self):
        """Test as_block_string with multiple lines."""
        result = serialise.as_block_string("line1\nline2\nline3", 0)
        assert result.startswith("|")
        assert "line1" in result
        assert "line2" in result

    def test_empty_string(self):
        """Test as_block_string with empty string."""
        result = serialise.as_block_string("", 0)
        assert result == "''"


class TestDictToAttributesCode:
    """Tests for dict_to_attributes_code function."""

    def test_simple_dict(self):
        """Test dict_to_attributes_code with simple dict."""
        data = {"name": "test_package", "version": "1.0.0"}
        result = serialise.dict_to_attributes_code(data)
        assert "test_package" in result
        assert "1.0.0" in result

    def test_with_requires(self):
        """Test dict_to_attributes_code with requires."""
        data = {"name": "test", "requires": ["python-3.9"]}
        result = serialise.dict_to_attributes_code(data)
        assert "requires" in result


class TestPackageKeyOrder:
    """Tests for the upstream-compatible package_key_order list."""

    def test_returns_list(self):
        """Test package_key_order returns a list."""
        result = serialise.package_key_order
        assert isinstance(result, list)
        assert len(result) > 0

    def test_first_key_is_name(self):
        """Test package_key_order starts with 'name'."""
        result = serialise.package_key_order
        assert result[0] == "name"

    def test_contains_version(self):
        """Test package_key_order contains 'version'."""
        result = serialise.package_key_order
        assert "version" in result


class TestFileFormat:
    """Tests for FileFormat class."""

    def test_yaml_attr(self):
        """Test FileFormat.yaml attribute."""
        assert serialise.FileFormat.yaml == "yaml"

    def test_json_attr(self):
        """Test FileFormat.json attribute."""
        assert serialise.FileFormat.json == "json"

    def test_python_attr(self):
        """Test FileFormat.python attribute."""
        assert serialise.FileFormat.python == "python"

    def test_toml_attr(self):
        """Test FileFormat.toml attribute."""
        assert serialise.FileFormat.toml == "toml"


class TestDumpPackageData:
    """Tests for dump_package_data function."""

    def test_dump_to_yaml_file(self):
        """Test dump_package_data to YAML file."""
        data = {"name": "test_package", "version": "1.0.0"}
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            filepath = f.name

        try:
            serialise.dump_package_data(data, filepath, "yaml")
            assert os.path.exists(filepath)
            with open(filepath) as f:
                content = f.read()
                assert "name:" in content
        finally:
            os.unlink(filepath)

    def test_dump_to_json_file(self):
        """Test dump_package_data to JSON file."""
        data = {"name": "test_package", "version": "1.0.0"}
        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            filepath = f.name

        try:
            serialise.dump_package_data(data, filepath, "json")
            assert os.path.exists(filepath)
            with open(filepath) as f:
                content = f.read()
                assert "name" in content
        finally:
            os.unlink(filepath)
