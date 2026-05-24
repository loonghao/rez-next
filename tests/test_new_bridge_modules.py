"""
Tests for newly added rez-aligned bridge modules.

Tests alignment with Rez API:
- ``serialise`` — file serialisation (FileFormat, load_from_file, etc.)
- ``package_serialise`` — package data serialisation (dump_package_data)
- ``package_test`` — package test runner (PackageTestRunner, PackageTestResults)
- ``rezconfig`` — configuration defaults
"""

from __future__ import annotations

import os
import sys
import tempfile
import pytest

# Ensure workspace bridges are importable
_workspace_root = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
_rez_next_pkg = os.path.join(
    _workspace_root, "crates", "rez-next-python", "python"
)
for _p in (_workspace_root, _rez_next_pkg):
    if _p not in sys.path:
        sys.path.insert(0, _p)

# Explicitly clean any cached modules
for _mod in list(sys.modules.keys()):
    if _mod.startswith("rez_next"):
        del sys.modules[_mod]

import rez_next  # noqa: E402
from rez_next import serialise  # noqa: E402
from rez_next import package_serialise  # noqa: E402
from rez_next import package_test  # noqa: E402
from rez_next import rezconfig  # noqa: E402


# ═══════════════════════════════════════════════════════════════════════════
# serialise module tests
# ═══════════════════════════════════════════════════════════════════════════


class TestSerialiseFileFormat:
    """Tests for serialise.FileFormat enum (aligns rez.serialise.FileFormat)."""

    def test_file_format_py(self):
        assert serialise.FileFormat.py.extension == ".py"

    def test_file_format_yaml(self):
        assert serialise.FileFormat.yaml.extension == ".yaml"

    def test_file_format_txt(self):
        assert serialise.FileFormat.txt.extension == ".txt"

    def test_file_format_values(self):
        """Verify all Rez FileFormat members are present."""
        members = {m.name for m in serialise.FileFormat}
        assert members == {"py", "yaml", "txt"}


class TestSerialiseLoadPy:
    """Tests for serialise.load_py."""

    def test_load_py_simple(self):
        data = serialise.load_py("name = 'mylib'\nversion = '1.0.0'")
        assert data.get("name") == "mylib"
        assert data.get("version") == "1.0.0"

    def test_load_py_skips_private(self):
        data = serialise.load_py("__private__ = 'secret'\nname = 'mylib'")
        assert "name" in data
        assert "__private__" not in data

    def test_load_py_skips_modules(self):
        data = serialise.load_py("name = 'mylib'\nimport os")
        assert data.get("name") == "mylib"

    def test_load_py_empty(self):
        data = serialise.load_py("")
        assert data == {}


class TestSerialiseLoadYaml:
    """Tests for serialise.load_yaml."""

    def test_load_yaml_simple(self):
        data = serialise.load_yaml("name: mylib\nversion: \"1.0.0\"")
        assert data.get("name") == "mylib"
        assert data.get("version") == "1.0.0"

    def test_load_yaml_empty(self):
        data = serialise.load_yaml("")
        assert data == {}

    def test_load_yaml_empty_doc(self):
        data = serialise.load_yaml("---\n...")
        assert data == {}


class TestSerialiseLoadTxt:
    """Tests for serialise.load_txt."""

    def test_load_txt(self):
        result = serialise.load_txt("hello world")
        assert result == "hello world"

    def test_load_txt_multiline(self):
        result = serialise.load_txt("line1\nline2\nline3")
        assert result == "line1\nline2\nline3"


class TestSerialiseOpenFileForWrite:
    """Tests for serialise.open_file_for_write."""

    def test_open_file_for_write(self):
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".txt", delete=False
        ) as tmp:
            tmp_path = tmp.name

        try:
            with serialise.open_file_for_write(tmp_path) as buf:
                buf.write("test content")

            with open(tmp_path, "r", encoding="utf-8") as f:
                assert f.read() == "test content"
        finally:
            if os.path.exists(tmp_path):
                os.unlink(tmp_path)


class TestSerialiseLoadFromFile:
    """Tests for serialise.load_from_file."""

    def test_load_from_file_py(self):
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".py", delete=False
        ) as tmp:
            tmp.write("name = 'mylib'\nversion = '1.0.0'")
            tmp_path = tmp.name

        try:
            data = serialise.load_from_file(tmp_path, format_=serialise.FileFormat.py)
            assert data.get("name") == "mylib"
        finally:
            if os.path.exists(tmp_path):
                os.unlink(tmp_path)

    def test_load_from_file_txt(self):
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".txt", delete=False
        ) as tmp:
            tmp.write("just text")
            tmp_path = tmp.name

        try:
            data = serialise.load_from_file(tmp_path, format_=serialise.FileFormat.txt)
            assert data == "just text"
        finally:
            if os.path.exists(tmp_path):
                os.unlink(tmp_path)

    def test_load_from_file_with_callback(self):
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".py", delete=False
        ) as tmp:
            tmp.write("name = 'mylib'")
            tmp_path = tmp.name

        def cb(fmt, data):
            data["version"] = "2.0.0"
            return data

        try:
            data = serialise.load_from_file(
                tmp_path, format_=serialise.FileFormat.py,
                update_data_callback=cb
            )
            assert data.get("name") == "mylib"
            assert data.get("version") == "2.0.0"
        finally:
            if os.path.exists(tmp_path):
                os.unlink(tmp_path)


class TestSerialiseEarlyThis:
    """Tests for serialise.EarlyThis."""

    def test_early_this_getattr(self):
        data = {"name": "mylib", "version": "1.0.0"}
        this = serialise.EarlyThis(data)
        assert this.name == "mylib"
        assert this.version == "1.0.0"

    def test_early_this_missing_attr(self):
        this = serialise.EarlyThis({})
        with pytest.raises(AttributeError):
            _ = this.missing


class TestSerialiseSetObjects:
    """Tests for serialise.set_objects / get_objects."""

    def test_set_and_get_objects(self):
        with serialise.set_objects({"building": True}):
            objs = serialise.get_objects()
            assert objs.get("building") is True

    def test_set_objects_restore(self):
        with serialise.set_objects({"building": True}):
            pass
        objs = serialise.get_objects()
        assert "building" not in objs


class TestSerialiseProcessPythonObjects:
    """Tests for serialise.process_python_objects."""

    def test_process_basic(self):
        data = {"name": "mylib", "version": "1.0.0"}
        result = serialise.process_python_objects(data)
        assert result.get("name") == "mylib"

    def test_process_skips_private(self):
        data = {"__private__": "secret", "name": "mylib"}
        result = serialise.process_python_objects(data)
        assert "__private__" not in result
        assert result.get("name") == "mylib"


class TestSerialiseClearFileCaches:
    """Tests for serialise.clear_file_caches."""

    def test_clear_file_caches(self):
        serialise.clear_file_caches()  # Should not raise


# ═══════════════════════════════════════════════════════════════════════════
# package_serialise module tests
# ═══════════════════════════════════════════════════════════════════════════


class TestPackageSerialise:
    """Tests for rez_next.package_serialise (aligns rez.package_serialise)."""

    def test_dump_package_data_to_string(self):
        data = {"name": "mylib", "version": "1.0.0"}
        result = package_serialise.dump_package_data(data)
        assert isinstance(result, str)
        assert "mylib" in result

    def test_dump_package_data_to_file(self):
        data = {"name": "mylib", "version": "1.0.0"}
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".yaml", delete=False
        ) as tmp:
            tmp_path = tmp.name

        try:
            package_serialise.dump_package_data(data, destination=tmp_path)
            with open(tmp_path, "r", encoding="utf-8") as f:
                content = f.read()
            assert "mylib" in content
        finally:
            if os.path.exists(tmp_path):
                os.unlink(tmp_path)

    def test_dump_package_data_format(self):
        data = {"name": "mylib", "version": "1.0.0"}
        result = package_serialise.dump_package_data(data, format="yaml")
        assert "mylib" in result
        assert isinstance(result, str)

    def test_package_key_order_exists(self):
        assert package_serialise.package_key_order is not None
        assert isinstance(package_serialise.package_key_order, (list, tuple))

    def test_file_format_exported(self):
        assert package_serialise.FileFormat is not None


# ═══════════════════════════════════════════════════════════════════════════
# package_test module tests
# ═══════════════════════════════════════════════════════════════════════════


class TestPackageTest:
    """Tests for rez_next.package_test (aligns rez.package_test)."""

    def test_package_test_runner_exists(self):
        assert package_test.PackageTestRunner is not None

    def test_package_test_results_exists(self):
        assert package_test.PackageTestResults is not None

    def test_status_constants(self):
        assert package_test.SUCCESS is not None
        assert package_test.FAILED is not None
        assert package_test.SKIPPED is not None
        assert package_test.ERROR is not None


# ═══════════════════════════════════════════════════════════════════════════
# rezconfig module tests
# ═══════════════════════════════════════════════════════════════════════════


class TestRezconfig:
    """Tests for rez_next.rezconfig (aligns rez.rezconfig)."""

    def test_packages_path_default(self):
        assert isinstance(rezconfig.packages_path, list)
        assert len(rezconfig.packages_path) > 0

    def test_implicit_packages_default(self):
        assert isinstance(rezconfig.implicit_packages, list)
        assert any("~platform" in p for p in rezconfig.implicit_packages)

    def test_cache_settings(self):
        assert isinstance(rezconfig.resolve_caching, bool)
        assert isinstance(rezconfig.cache_package_files, bool)

    def test_debug_settings_exist(self):
        assert isinstance(rezconfig.debug_resolve, bool)
        assert isinstance(rezconfig.debug_build, bool)

    def test_color_enabled(self):
        assert isinstance(rezconfig.color_enabled, bool)

    def test_variant_select_mode(self):
        assert rezconfig.variant_select_mode == "version_priority"

    def test_build_settings(self):
        assert rezconfig.build_directory == "build"
        assert rezconfig.build_thread_count == "physical_cores"

    def test_behavior_defaults(self):
        assert rezconfig.catch_rex_errors is True
        assert rezconfig.allow_unversioned_packages is True
        assert rezconfig.default_relocatable is True


# ═══════════════════════════════════════════════════════════════════════════
# Import tests (verify all modules importable at rez_next level)
# ═══════════════════════════════════════════════════════════════════════════


def test_rez_next_imports_new_modules():
    """Verify new modules are accessible via rez_next.<module>."""
    assert hasattr(rez_next, "serialise")
    assert hasattr(rez_next, "package_serialise")
    assert hasattr(rez_next, "package_test")
    assert hasattr(rez_next, "rezconfig")
