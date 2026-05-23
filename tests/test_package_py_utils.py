"""Tests for rez_next.package_py_utils module."""

import pytest
from rez_next.package_py_utils import (
    expand_requirement,
    expand_requires,
    exec_command,
    exec_python,
    find_site_python,
)


class TestExpandRequirement:
    def test_no_wildcard_returns_unchanged(self):
        assert expand_requirement("python-3.9") == "python-3.9"

    def test_empty_string(self):
        assert expand_requirement("") == ""

    def test_simple_request(self):
        result = expand_requirement("python")
        assert result == "python"


class TestExpandRequires:
    def test_multiple_requests(self):
        result = expand_requires("python", "maya")
        assert len(result) == 2
        assert isinstance(result[0], str)
        assert isinstance(result[1], str)

    def test_empty(self):
        result = expand_requires()
        assert result == []


class TestExecCommand:
    def test_simple_echo(self):
        import sys
        cmd = [sys.executable, "-c", "print('hello')"]
        out, err = exec_command("test_attr", cmd)
        assert out == "hello"
        assert err == ""


class TestExecPython:
    def test_simple_code(self):
        result = exec_python("test_attr", "print('hello world')")
        assert result == "hello world"

    def test_with_list_source(self):
        result = exec_python("test_attr", ["x = 42", "print(x)"])
        assert result == "42"


class TestFindSitePython:
    def test_raises_for_nonexistent_module(self):
        from rez_next.exceptions import InvalidPackageError
        with pytest.raises(InvalidPackageError):
            find_site_python("_nonexistent_module_xyz_")
