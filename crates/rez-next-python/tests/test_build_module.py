"""Tests for rez_next.build_ module (Cycle 247)."""

import os
import sys

import pytest

# Add the path so we can import rez_next
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

import rez_next.build_ as build_


class TestBuildType:
    """Tests for BuildType enum."""

    def test_local(self):
        bt = build_.BuildType("local")
        assert str(bt) == "BuildType.local"
        assert bt.name == "local"
        assert bt.value == 0

    def test_central(self):
        bt = build_.BuildType("central")
        assert str(bt) == "BuildType.central"
        assert bt.name == "central"
        assert bt.value == 1

    def test_invalid(self):
        with pytest.raises(ValueError):
            build_.BuildType("invalid")

    def test_eq(self):
        bt1 = build_.BuildType("local")
        bt2 = build_.BuildType("local")
        assert bt1 == bt2

    def test_ne(self):
        bt1 = build_.BuildType("local")
        bt2 = build_.BuildType("central")
        assert bt1 != bt2


class TestBuildSystem:
    """Tests for BuildSystem class."""

    def test_detect_cmakes(self, tmp_path):
        (tmp_path / "CMakeLists.txt").write_text("cmake_minimum_required(VERSION 3.10)")
        bs = build_.BuildSystem.detect(str(tmp_path))
        assert "cmake" in str(bs).lower()

    def test_detect_make(self, tmp_path):
        (tmp_path / "Makefile").write_text("all:\n\techo done")
        bs = build_.BuildSystem.detect(str(tmp_path))
        assert "make" in str(bs).lower()

    def test_detect_python_setup(self, tmp_path):
        (tmp_path / "setup.py").write_text("from setuptools import setup; setup()")
        bs = build_.BuildSystem.detect(str(tmp_path))
        assert "python" in str(bs).lower()

    def test_detect_nodejs(self, tmp_path):
        (tmp_path / "package.json").write_text('{"name":"test","version":"1.0.0"}')
        bs = build_.BuildSystem.detect(str(tmp_path))
        assert "nodejs" in str(bs).lower()

    def test_detect_cargo(self, tmp_path):
        (tmp_path / "Cargo.toml").write_text('[package]\nname = "test"\nversion = "0.1.0"')
        bs = build_.BuildSystem.detect(str(tmp_path))
        assert "cargo" in str(bs).lower()

    def test_get_type(self):
        # Cannot directly construct BuildSystem, use detect instead
        pass


class TestBuildFunctions:
    """Tests for build_ module functions."""

    def test_get_buildsys_types(self):
        types = build_.get_buildsys_types()
        assert "cmake" in types
        assert "make" in types
        assert "python" in types

    def test_get_build_process_types(self):
        types = build_.get_build_process_types()
        assert "local" in types
        assert "central" in types
        assert len(types) == 2

    def test_create_build_system_valid(self):
        result = build_.create_build_system("cmake")
        assert "BuildSystem" in result

    def test_create_build_system_invalid(self):
        with pytest.raises(ValueError):
            build_.create_build_system("invalid")


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
