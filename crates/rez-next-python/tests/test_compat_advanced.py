"""
Advanced module compatibility tests for rez_next.

Covers: plugins, search, exceptions, conflict/weak requirements,
        selftest, utils, rex.

Usage:
    maturin develop --features extension-module
    pytest tests/test_compat_advanced.py -v
"""
import pytest

rez = pytest.importorskip(
    "rez_next",
    reason="rez_next not built yet — run: maturin develop --features extension-module",
)


# ── Plugins ───────────────────────────────────────────────────────────────────


class TestPluginsModule:
    """Verify rez.plugins plugin manager API."""

    def test_plugins_submodule_exists(self):
        import rez_next.plugins as plugins
        assert hasattr(plugins, "get_plugin_manager")
        assert hasattr(plugins, "get_shell_types")
        assert hasattr(plugins, "get_build_system_types")
        assert hasattr(plugins, "is_shell_supported")
        assert hasattr(plugins, "plugin_manager")

    def test_plugin_manager_singleton(self):
        import rez_next.plugins as plugins
        mgr = plugins.plugin_manager
        assert mgr is not None

    def test_get_plugin_manager_function(self):
        import rez_next.plugins as plugins
        mgr = plugins.get_plugin_manager()
        assert mgr is not None
        assert mgr.count > 0

    def test_get_shell_types(self):
        import rez_next.plugins as plugins
        shells = plugins.get_shell_types()
        assert isinstance(shells, list)
        assert "bash" in shells
        assert "powershell" in shells
        assert "fish" in shells
        assert "cmd" in shells

    def test_get_build_system_types(self):
        import rez_next.plugins as plugins
        build_systems = plugins.get_build_system_types()
        assert isinstance(build_systems, list)
        assert "cmake" in build_systems
        assert "python_rezbuild" in build_systems

    def test_is_shell_supported(self):
        import rez_next.plugins as plugins
        assert plugins.is_shell_supported("bash")
        assert plugins.is_shell_supported("powershell")
        assert not plugins.is_shell_supported("nonexistent_xyz")

    def test_plugin_manager_get_plugins(self):
        import rez_next.plugins as plugins
        mgr = plugins.get_plugin_manager()
        shell_plugins = mgr.get_plugins("shell")
        assert isinstance(shell_plugins, list)
        assert len(shell_plugins) > 0

    def test_plugin_manager_has_plugin(self):
        import rez_next.plugins as plugins
        mgr = plugins.get_plugin_manager()
        assert mgr.has_plugin("shell", "bash")
        assert mgr.has_plugin("build_system", "cmake")
        assert not mgr.has_plugin("shell", "nonexistent")

    def test_plugin_manager_plugin_types(self):
        import rez_next.plugins as plugins
        mgr = plugins.get_plugin_manager()
        types = mgr.plugin_types()
        assert "shell" in types
        assert "build_system" in types
        assert "release_hook" in types

    def test_plugin_object_attributes(self):
        import rez_next.plugins as plugins
        mgr = plugins.get_plugin_manager()
        plugin = mgr.get_plugin("shell", "bash")
        assert plugin is not None
        assert plugin.name == "bash"
        assert plugin.plugin_type == "shell"
        assert plugin.description

    def test_top_level_get_plugin_manager(self):
        assert callable(rez.get_plugin_manager)
        mgr = rez.get_plugin_manager()
        assert mgr is not None


# ── Search ────────────────────────────────────────────────────────────────────


class TestSearchModule:
    """Verify rez.search / rez_next.search Python API."""

    def test_search_submodule_exists(self):
        import rez_next.search as search
        assert hasattr(search, "search_packages")
        assert hasattr(search, "search_package_names")
        assert hasattr(search, "search_latest_packages")
        assert hasattr(search, "PackageSearcher")
        assert hasattr(search, "SearchResult")

    def test_search_packages_empty_paths_empty_result(self):
        import rez_next.search as search
        results = search.search_packages(
            pattern="python", paths=["/nonexistent/path_xyz"]
        )
        assert results == [], f"nonexistent path must yield empty results, got {results}"

    def test_search_package_names_returns_empty_for_nonexistent_path(self):
        import rez_next.search as search
        names = search.search_package_names(
            pattern="", paths=["/nonexistent/path_xyz"]
        )
        assert names == [], f"nonexistent path must yield empty names, got {names}"

    def test_search_latest_packages_returns_empty_for_nonexistent_path(self):
        import rez_next.search as search
        results = search.search_latest_packages(
            pattern="", paths=["/nonexistent/path_xyz"]
        )
        assert results == [], f"nonexistent path must yield empty results, got {results}"

    def test_package_searcher_create(self):
        import rez_next.search as search
        searcher = search.PackageSearcher(
            pattern="py",
            paths=["/nonexistent/path_xyz"],
            scope="families",
        )
        assert hasattr(searcher, "search"), "PackageSearcher must expose .search()"

    def test_package_searcher_repr(self):
        import rez_next.search as search
        searcher = search.PackageSearcher(pattern="maya", scope="latest")
        r = repr(searcher)
        assert "maya" in r
        assert "latest" in r

    def test_package_searcher_search_returns_empty_for_nonexistent_path(self):
        import rez_next.search as search
        searcher = search.PackageSearcher(
            pattern="", paths=["/nonexistent/path_xyz"]
        )
        results = searcher.search()
        assert results == [], f"search on nonexistent path must be empty, got {results}"

    def test_search_scope_families_vs_latest_both_empty_for_nonexistent(self):
        import rez_next.search as search
        families = search.search_packages(
            scope="families", paths=["/nonexistent/path_xyz"]
        )
        latest = search.search_packages(
            scope="latest", paths=["/nonexistent/path_xyz"]
        )
        assert families == [], f"families scope on nonexistent path must be empty, got {families}"
        assert latest == [], f"latest scope on nonexistent path must be empty, got {latest}"

    def test_top_level_search_packages_callable(self):
        assert callable(rez.search_packages)

    def test_top_level_search_package_names_callable(self):
        assert callable(rez.search_package_names)


# ── Exceptions ────────────────────────────────────────────────────────────────


class TestExceptionsSubmodule:
    """Verify rez.exceptions submodule is complete."""

    def test_all_exceptions_present(self):
        from rez_next.exceptions import (
            PackageNotFound,
            PackageVersionConflict,
            ResolveError,
            RezBuildError,
            ConfigurationError,
            PackageParseError,
            ContextBundleError,
            SuiteError,
            RexError,
        )
        assert all(e is not None for e in [
            PackageNotFound, PackageVersionConflict, ResolveError,
            RezBuildError, ConfigurationError, PackageParseError,
            ContextBundleError, SuiteError, RexError,
        ])

    def test_all_exceptions_are_subclasses_of_exception(self):
        from rez_next.exceptions import (
            PackageNotFound, PackageVersionConflict, ResolveError,
            RezBuildError, ConfigurationError, PackageParseError,
            ContextBundleError, SuiteError, RexError,
        )
        for exc in [
            PackageNotFound, PackageVersionConflict, ResolveError,
            RezBuildError, ConfigurationError, PackageParseError,
            ContextBundleError, SuiteError, RexError,
        ]:
            assert issubclass(exc, Exception)

    def test_top_level_exceptions(self):
        assert issubclass(rez.PackageNotFound, Exception)
        assert issubclass(rez.ResolveError, Exception)
        assert issubclass(rez.RezError, Exception)

    def test_raise_package_not_found(self):
        with pytest.raises(rez.PackageNotFound):
            raise rez.PackageNotFound("test package not found")

    def test_raise_resolve_error(self):
        with pytest.raises(rez.ResolveError):
            raise rez.ResolveError("test resolve error")


# ── Conflict / Weak requirements ──────────────────────────────────────────────


class TestConflictWeakRequirement:
    """Verify conflict (!pkg) and weak (~pkg) requirement semantics."""

    def test_conflict_requirement_flag(self):
        req = rez.PackageRequirement("!python")
        assert req.conflict is True
        assert req.weak is False

    def test_weak_requirement_flag(self):
        req = rez.PackageRequirement("~python")
        assert req.weak is True
        assert req.conflict is False

    def test_normal_requirement_no_flags(self):
        req = rez.PackageRequirement("python-3.9")
        assert req.conflict is False
        assert req.weak is False

    def test_conflict_requirement_with_version(self):
        req = rez.PackageRequirement("!python-3.9")
        assert req.name == "python"
        assert req.conflict is True

    def test_conflict_requirement_str_starts_with_bang(self):
        req = rez.PackageRequirement("!python")
        s = str(req)
        assert s.startswith("!"), f"Expected '!' prefix in '{s}'"

    def test_weak_requirement_str_starts_with_tilde(self):
        req = rez.PackageRequirement("~python")
        s = str(req)
        assert s.startswith("~"), f"Expected '~' prefix in '{s}'"

    def test_conflict_requirement_method(self):
        req = rez.PackageRequirement("python-3.9")
        conflict_str = req.conflict_requirement()
        assert conflict_str.startswith("!")
        assert "python" in conflict_str

    def test_vendor_version_range_any_none(self):
        from rez_next.vendor.version import VersionRange
        any_r = VersionRange.any()
        assert any_r.is_any()
        none_r = VersionRange.none()
        assert none_r.is_empty()

    def test_vendor_version_range_from_str(self):
        from rez_next.vendor.version import VersionRange, Version
        r = VersionRange.from_str(">=2.0,<3.0")
        assert r.contains(Version("2.5"))
        assert not r.contains(Version("3.0"))

    def test_multiple_conflict_requirements(self):
        """Verify conflict flag is consistent across different names."""
        for name in ("python", "maya", "houdini", "numpy"):
            req = rez.PackageRequirement(f"!{name}")
            assert req.conflict is True, f"!{name} should be conflict"

    def test_multiple_weak_requirements(self):
        for name in ("python", "maya"):
            req = rez.PackageRequirement(f"~{name}")
            assert req.weak is True, f"~{name} should be weak"


# ── Selftest ──────────────────────────────────────────────────────────────────


class TestSelftest:
    """Verify selftest function."""

    def test_selftest_exists(self):
        assert callable(rez.selftest)

    def test_selftest_returns_tuple(self):
        result = rez.selftest()
        assert isinstance(result, tuple)
        assert len(result) == 3
        passed, failed, total = result
        assert total == passed + failed

    def test_selftest_no_failures(self):
        passed, failed, total = rez.selftest()
        assert total > 0, "selftest should run at least some tests"
        assert failed == 0, f"selftest failed: {failed} tests failed"


# ── Utils ─────────────────────────────────────────────────────────────────────


class TestUtilsModule:
    """Verify rez.utils.resources submodule."""

    def test_utils_resources_submodule(self):
        from rez_next.utils.resources import get_resource_string
        ver = get_resource_string("version")
        assert isinstance(ver, str)
        assert len(ver) > 0

    def test_get_resource_string_name(self):
        from rez_next.utils.resources import get_resource_string
        name = get_resource_string("name")
        assert "rez" in name.lower()

    def test_get_resource_string_unknown_raises(self):
        from rez_next.utils.resources import get_resource_string
        with pytest.raises(KeyError):
            get_resource_string("nonexistent_resource_xyz_12345")

    def test_get_version_string_is_semver_like(self):
        from rez_next.utils.resources import get_resource_string
        ver = get_resource_string("version")
        parts = ver.split(".")
        assert len(parts) >= 2


# ── Rex ───────────────────────────────────────────────────────────────────────


class TestRexModule:
    """Verify rez.rex submodule."""

    def test_rex_submodule_exists(self):
        import rez_next.rex as rex
        assert hasattr(rex, "rex_interpret")

    def test_rex_interpret_function(self):
        import rez_next.rex as rex
        result = rex.rex_interpret("")
        assert isinstance(result, dict)

    def test_rex_interpret_setenv(self):
        import rez_next.rex as rex
        result = rex.rex_interpret("env.setenv('MY_VAR', 'hello')")
        assert isinstance(result, dict)
        assert result.get("MY_VAR") == "hello"

    def test_rex_interpret_prepend_path(self):
        import rez_next.rex as rex
        result = rex.rex_interpret("env.prepend_path('PATH', '/opt/pkg/bin')")
        assert isinstance(result, dict)

    def test_rex_interpret_alias(self):
        import rez_next.rex as rex
        result = rex.rex_interpret("alias('mypkg', '/opt/pkg/bin/mypkg')")
        assert isinstance(result, dict)

    def test_rex_interpret_multiple_commands(self):
        import rez_next.rex as rex
        commands = "\n".join([
            "env.setenv('A', '1')",
            "env.setenv('B', '2')",
            "env.setenv('C', '3')",
        ])
        result = rex.rex_interpret(commands)
        assert result.get("A") == "1"
        assert result.get("B") == "2"
        assert result.get("C") == "3"

    def test_rex_interpret_maya_style(self):
        import rez_next.rex as rex
        commands = "\n".join([
            "env.setenv('MAYA_ROOT', '/opt/maya/2024')",
            "env.setenv('MAYA_VERSION', '2024')",
            "env.prepend_path('PATH', '/opt/maya/2024/bin')",
        ])
        result = rex.rex_interpret(commands)
        assert result.get("MAYA_ROOT") == "/opt/maya/2024"
        assert result.get("MAYA_VERSION") == "2024"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
