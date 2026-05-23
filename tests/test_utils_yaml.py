"""Tests for rez_next.utils.yaml module."""

from __future__ import annotations

import os

import pytest
from rez_next.utils.yaml import dump_yaml, load_yaml, save_yaml


class TestDumpYaml:
    """Tests for dump_yaml function."""

    def test_dict_output(self):
        data = {"name": "hello", "version": "1.0"}
        result = dump_yaml(data)
        assert "hello" in result
        assert "1.0" in result
        assert result.startswith("{") or "name:" in result

    def test_list_output(self):
        data = ["a", "b", "c"]
        result = dump_yaml(data)
        assert "a" in result
        assert result.strip() != ""

    def test_nested_structure(self):
        data = {"nested": {"inner": "value"}}
        result = dump_yaml(data)
        assert "inner" in result

    def test_default_flow_style_override(self):
        data = {"a": {"b": "c"}}
        flow = dump_yaml(data, default_flow_style=True)
        block = dump_yaml(data, default_flow_style=False)
        assert flow is not None
        assert block is not None

    def test_empty_dict(self):
        assert dump_yaml({}) == "{}"


class TestLoadYaml:
    """Tests for load_yaml function."""

    def test_load_simple_dict(self, tmp_path):
        f = tmp_path / "test.yaml"
        f.write_text("name: hello\nversion: '1.0'\n")
        result = load_yaml(str(f))
        assert result == {"name": "hello", "version": "1.0"}

    def test_load_list(self, tmp_path):
        f = tmp_path / "list.yaml"
        f.write_text("- a\n- b\n- c\n")
        result = load_yaml(str(f))
        assert result == ["a", "b", "c"]

    def test_file_not_found(self):
        with pytest.raises(FileNotFoundError):
            load_yaml("/nonexistent/file.yaml")

    def test_invalid_yaml(self, tmp_path):
        f = tmp_path / "invalid.yaml"
        f.write_text(": invalid : yaml :\n")
        with pytest.raises(Exception):
            load_yaml(str(f))


class TestSaveYaml:
    """Tests for save_yaml function."""

    def test_save_and_load_roundtrip(self, tmp_path):
        fpath = str(tmp_path / "roundtrip.yaml")
        save_yaml(fpath, name="hello", version="1.0")
        assert os.path.exists(fpath)

        loaded = load_yaml(fpath)
        assert loaded == {"name": "hello", "version": "1.0"}

    def test_save_creates_directories(self, tmp_path):
        nested = str(tmp_path / "a" / "b" / "test.yaml")
        save_yaml(nested, key="value")
        assert os.path.exists(nested)
        loaded = load_yaml(nested)
        assert loaded == {"key": "value"}

    def test_overwrite_existing(self, tmp_path):
        fpath = str(tmp_path / "overwrite.yaml")
        save_yaml(fpath, first="original")
        save_yaml(fpath, second="overwritten")
        loaded = load_yaml(fpath)
        assert loaded == {"second": "overwritten"}
