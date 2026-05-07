"""Tests for rez_next top-level API compatibility with rez."""

import pytest
import rez_next
import os
from rez_next._native import Package, ResolvedContext


class TestVersion:
    """Test __version__ attribute (rez compatibility)."""

    def test_version_is_string(self):
        """rez_next.__version__ should be a string."""
        assert isinstance(rez_next.__version__, str)

    def test_version_matches_semver(self):
        """Version should be in semver format (X.Y.Z)."""
        parts = rez_next.__version__.split(".")
        assert len(parts) >= 3, "Version should be at least X.Y.Z"
        for part in parts[:3]:
            assert part.isdigit(), f"Version part '{part}' should be numeric"

    def test_version_non_empty(self):
        """Version should not be empty."""
        assert len(rez_next.__version__) > 0


class TestModuleRootPath:
    """Test module_root_path attribute (rez compatibility)."""

    def test_module_root_path_is_string(self):
        """rez_next.module_root_path should be a string."""
        assert isinstance(rez_next.module_root_path, str)

    def test_module_root_path_exists(self):
        """module_root_path should point to an existing directory."""
        assert os.path.isdir(rez_next.module_root_path)

    def test_module_root_path_contains_init_py(self):
        """The directory should contain __init__.py."""
        init_path = os.path.join(rez_next.module_root_path, "__init__.py")
        assert os.path.isfile(init_path), "__init__.py should exist in module root"


class TestPackageDictAccess:
    """Test dict-style access to Package (monkeypatch from __init__.py)."""

    def test_package_getitem_name(self):
        """Package[name] should return package name."""
        pkg = Package("test_pkg")
        assert pkg["name"] == "test_pkg"

    def test_package_getitem_version_str_default(self):
        """Package[version_str] should return version string (default: None or empty)."""
        pkg = Package("test_pkg")
        result = pkg["version_str"]
        # version_str might be None or empty string for default Package
        assert result is None or isinstance(result, str)

    def test_package_getitem_version_default(self):
        """Package[version] should return version string (default: None or empty)."""
        pkg = Package("test_pkg")
        result = pkg["version"]
        assert result is None or isinstance(result, str)

    def test_package_getitem_description_default(self):
        """Package[description] should return description (default: None or empty)."""
        pkg = Package("test_pkg")
        result = pkg["description"]
        assert result is None or isinstance(result, str)

    def test_package_getitem_authors_default(self):
        """Package[authors] should return authors list (default: empty list)."""
        pkg = Package("test_pkg")
        result = pkg["authors"]
        assert isinstance(result, list)

    def test_package_getitem_requires_default(self):
        """Package[requires] should return requires list (default: empty list)."""
        pkg = Package("test_pkg")
        result = pkg["requires"]
        assert isinstance(result, list)

    def test_package_getitem_tools_default(self):
        """Package[tools] should return tools list (default: empty list)."""
        pkg = Package("test_pkg")
        result = pkg["tools"]
        assert isinstance(result, list)

    def test_package_getitem_uuid_default(self):
        """Package[uuid] should return uuid (default: None or empty)."""
        pkg = Package("test_pkg")
        result = pkg["uuid"]
        assert result is None or isinstance(result, str)

    def test_package_getitem_timestamp_default(self):
        """Package[timestamp] should return timestamp (default: 0 or None)."""
        pkg = Package("test_pkg")
        result = pkg["timestamp"]
        assert result is None or isinstance(result, (int, float))

    def test_package_getitem_cachable_default(self):
        """Package[cachable] should return cachable flag (default: None or bool)."""
        pkg = Package("test_pkg")
        result = pkg["cachable"]
        assert result is None or isinstance(result, bool)

    def test_package_getitem_relocatable_default(self):
        """Package[relocatable] should return relocatable flag (default: None or bool)."""
        pkg = Package("test_pkg")
        result = pkg["relocatable"]
        assert result is None or isinstance(result, bool)

    def test_package_getitem_keyerror(self):
        """Package[invalid_key] should raise KeyError."""
        pkg = Package("test_pkg")
        with pytest.raises(KeyError):
            _ = pkg["invalid_key"]

    def test_package_getitem_with_loaded_package(self, tmp_path):
        """Package[name] should return correct name for loaded package."""
        import os
        pkg_file = tmp_path / "package.py"
        pkg_file.write_text('name = "loaded_pkg"\nversion = "1.0.0"\n')
        pkg = Package.load(str(pkg_file))
        assert pkg["name"] == "loaded_pkg"
        assert pkg["version_str"] == "1.0.0"


class TestResolvedContextGet:
    """Test ResolvedContext.get() method (monkeypatch from __init__.py)."""

    def test_context_get_success(self):
        """ResolvedContext.get('success') should return success flag."""
        ctx = ResolvedContext(["python-3.9"])
        result = ctx.get("success")
        assert isinstance(result, bool)

    def test_context_get_packages(self):
        """ResolvedContext.get('packages') should return resolved packages."""
        ctx = ResolvedContext(["python-3.9"])
        result = ctx.get("packages")
        assert isinstance(result, list)

    def test_context_get_resolved_packages(self):
        """ResolvedContext.get('resolved_packages') should return resolved packages."""
        ctx = ResolvedContext(["python-3.9"])
        result = ctx.get("resolved_packages")
        assert isinstance(result, list)

    def test_context_get_id(self):
        """ResolvedContext.get('id') should return context id."""
        ctx = ResolvedContext(["python-3.9"])
        result = ctx.get("id")
        assert isinstance(result, str)

    def test_context_get_created_at(self):
        """ResolvedContext.get('created_at') should return timestamp."""
        ctx = ResolvedContext(["python-3.9"])
        result = ctx.get("created_at")
        assert isinstance(result, (int, float))

    def test_context_get_num_resolved_packages(self):
        """ResolvedContext.get('num_resolved_packages') should return count."""
        ctx = ResolvedContext(["python-3.9"])
        result = ctx.get("num_resolved_packages")
        assert isinstance(result, int)
        assert result >= 0

    def test_context_get_missing_key_with_default(self):
        """ResolvedContext.get('missing', default) should return default."""
        ctx = ResolvedContext(["python-3.9"])
        result = ctx.get("missing_key", "default_value")
        assert result == "default_value"

    def test_context_get_missing_key_without_default(self):
        """ResolvedContext.get('missing') should raise KeyError."""
        ctx = ResolvedContext(["python-3.9"])
        with pytest.raises(KeyError):
            _ = ctx.get("missing_key")


class TestToDot:
    """Test ResolvedContext.to_dot() method (monkeypatch from __init__.py)."""

    def test_to_dot_returns_string(self):
        """to_dot() should return a string."""
        ctx = ResolvedContext(["python-3.9"])
        dot = ctx.to_dot()
        assert isinstance(dot, str)

    def test_to_dot_starts_with_digraph(self):
        """DOT output should start with 'digraph'."""
        ctx = ResolvedContext(["python-3.9"])
        dot = ctx.to_dot()
        assert dot.startswith("digraph")

    def test_to_dot_contains_package_nodes(self):
        """DOT output should contain package nodes (box shapes)."""
        ctx = ResolvedContext(["python-3.9"])
        dot = ctx.to_dot()
        # Check that DOT contains node definitions (package names with version)
        # The format is "package-version"; check for common patterns
        assert "box" in dot or "node" in dot.lower()

    def test_to_dot_contains_rankdir(self):
        """DOT output should contain rankdir=LR."""
        ctx = ResolvedContext(["python-3.9"])
        dot = ctx.to_dot()
        assert "rankdir=LR" in dot

    def test_to_dot_contains_node_style(self):
        """DOT output should contain node style (box, filled, lightblue)."""
        ctx = ResolvedContext(["python-3.9"])
        dot = ctx.to_dot()
        assert "shape=box" in dot
        assert "style=filled" in dot
        assert "fillcolor=lightblue" in dot


class TestTopLevelAPI:
    """Test top-level API compatibility with rez."""

    def test_config_attribute(self):
        """rez_next.config should be a Config instance."""
        assert rez_next.config is not None
        assert hasattr(rez_next.config, "get")

    def test_system_attribute(self):
        """rez_next.system should be a System instance."""
        assert rez_next.system is not None

    def test_resolve_function(self):
        """rez_next.resolve should be callable (alias for resolve_packages)."""
        assert callable(rez_next.resolve)

    def test_create_context_function(self):
        """rez_next.create_context should return ResolvedContext class."""
        assert rez_next.create_context is not None

    def test_action_attribute(self):
        """rez_next.action should be None or a string (from env var)."""
        assert rez_next.action is None or isinstance(rez_next.action, str)

    def test_module_root_path_attribute(self):
        """rez_next.module_root_path should be a string."""
        assert isinstance(rez_next.module_root_path, str)
        assert os.path.isdir(rez_next.module_root_path)

    def test_import_resolve_packages(self):
        """from rez_next import resolve_packages should work."""
        from rez_next import resolve_packages
        assert callable(resolve_packages)

    def test_import_ResolvedContext(self):
        """from rez_next import ResolvedContext should work."""
        from rez_next import ResolvedContext as RC
        assert RC is not None
