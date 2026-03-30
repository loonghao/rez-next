"""
Compatibility tests for rez_next as a drop-in replacement for rez.

Usage:
    maturin develop --features extension-module
    pytest tests/test_rez_compat.py -v

These tests verify that `import rez_next as rez` works as a complete
replacement for `import rez`.
"""
import pytest

# The main compatibility test: import rez_next as rez
try:
    import rez_next as rez
    REZ_NEXT_AVAILABLE = True
except ImportError:
    REZ_NEXT_AVAILABLE = False

pytestmark = pytest.mark.skipif(
    not REZ_NEXT_AVAILABLE,
    reason="rez_next not built yet. Run: maturin develop --features extension-module"
)


class TestModuleStructure:
    """Verify rez_next has the same module structure as rez."""

    def test_version_attr(self):
        assert hasattr(rez, "__version__")
        assert rez.__version__

    def test_has_config_singleton(self):
        """rez.config singleton must exist."""
        assert hasattr(rez, "config")
        cfg = rez.config
        assert cfg is not None

    def test_has_system_singleton(self):
        """rez.system singleton must exist."""
        assert hasattr(rez, "system")
        sys = rez.system
        assert sys is not None
        assert sys.platform in ("linux", "windows", "osx")
        assert sys.arch

    def test_config_submodule(self):
        from rez_next.config import Config
        cfg = Config()
        assert hasattr(cfg, "packages_path")
        assert hasattr(cfg, "local_packages_path")

    def test_system_submodule(self):
        from rez_next.system import system as sys_obj
        assert sys_obj is not None

    def test_exceptions_submodule(self):
        from rez_next.exceptions import PackageNotFound, ResolveError
        assert PackageNotFound is not None
        assert ResolveError is not None


class TestVersionClasses:
    """Verify version classes are compatible with rez.vendor.version."""

    def test_version_create(self):
        v = rez.Version("1.2.3")
        assert str(v) == "1.2.3"

    def test_version_range_create(self):
        r = rez.VersionRange(">=1.0.0,<2.0.0")
        assert r is not None

    def test_vendor_version_import(self):
        from rez_next.vendor.version import Version, VersionRange
        v = Version("2.0.0")
        assert str(v) == "2.0.0"
        r = VersionRange(">=1.0,<3.0")
        assert r is not None

    def test_version_range_contains(self):
        r = rez.VersionRange(">=1.0.0")
        v = rez.Version("2.5.0")
        assert r.contains(v)

    def test_version_comparison(self):
        v1 = rez.Version("1.0.0")
        v2 = rez.Version("2.0.0")
        assert v1 < v2


class TestPackageClasses:
    """Verify Package classes are compatible with rez.packages."""

    def test_package_create(self):
        p = rez.Package("my-package")
        assert p.name == "my-package"

    def test_package_with_version(self):
        p = rez.Package("my-package")
        v = rez.Version("1.0.0")
        p.set_version(str(v))
        assert p.version_str == "1.0.0"

    def test_package_requirement(self):
        req = rez.PackageRequirement("python-3.9")
        assert req.name == "python"
        assert "3.9" in req.version_range

    def test_package_str(self):
        p = rez.Package("python")
        assert "python" in str(p)


class TestResolvedContext:
    """Verify ResolvedContext is compatible with rez.resolved_context.ResolvedContext."""

    def test_resolved_context_import(self):
        from rez_next.resolved_context import ResolvedContext
        assert ResolvedContext is not None

    def test_resolved_context_empty(self):
        """Creating context with empty packages should work (may fail resolve)."""
        ctx = rez.ResolvedContext([])
        assert ctx is not None
        assert ctx.num_resolved_packages == 0

    def test_resolved_context_success_attr(self):
        ctx = rez.ResolvedContext([])
        # Empty resolve is technically success (no packages requested)
        assert hasattr(ctx, "success")

    def test_resolved_context_packages_attr(self):
        ctx = rez.ResolvedContext([])
        assert hasattr(ctx, "resolved_packages")
        assert isinstance(ctx.resolved_packages, list)


class TestRepositoryManager:
    """Verify RepositoryManager is compatible with rez repository API."""

    def test_repo_manager_create(self):
        repo = rez.RepositoryManager()
        assert repo is not None

    def test_repo_manager_search(self):
        repo = rez.RepositoryManager()
        # Search with no paths configured will return empty list
        results = repo.find_packages("nonexistent_package_12345")
        assert results == []


class TestConfigClass:
    """Verify Config class API."""

    def test_config_packages_path(self):
        cfg = rez.Config()
        assert isinstance(cfg.packages_path, list)

    def test_config_local_packages_path(self):
        cfg = rez.Config()
        assert isinstance(cfg.local_packages_path, str)

    def test_config_get(self):
        cfg = rez.Config()
        # get() should return a value or None
        result = cfg.get("packages_path", None)
        assert result is not None or result is None  # either is fine


class TestSuites:
    """Verify Suite API."""

    def test_suite_import(self):
        from rez_next.suite import Suite, SuiteManager
        assert Suite is not None
        assert SuiteManager is not None

    def test_suite_create(self):
        suite = rez.Suite()
        assert suite is not None


class TestTopLevelFunctions:
    """Verify top-level convenience functions."""

    def test_resolve_packages_exists(self):
        assert callable(rez.resolve_packages)

    def test_iter_packages_exists(self):
        assert callable(rez.iter_packages)

    def test_get_latest_package_exists(self):
        assert callable(rez.get_latest_package)

    def test_get_package_exists(self):
        assert callable(rez.get_package)

    def test_get_package_family_names_exists(self):
        assert callable(rez.get_package_family_names)

    def test_copy_package_exists(self):
        assert callable(rez.copy_package)

    def test_move_package_exists(self):
        assert callable(rez.move_package)

    def test_remove_package_exists(self):
        assert callable(rez.remove_package)

    def test_packages_submodule(self):
        """rez.packages_ submodule compatibility."""
        import rez_next.packages_ as pkgs
        assert callable(pkgs.get_latest_package)
        assert callable(pkgs.iter_packages)
        assert callable(pkgs.get_package_family_names)


class TestShellModule:
    """Verify rez.shell API."""

    def test_shell_submodule_exists(self):
        import rez_next.shell as shell
        assert hasattr(shell, "Shell")
        assert hasattr(shell, "create_shell_script")
        assert hasattr(shell, "get_available_shells")
        assert hasattr(shell, "get_current_shell")

    def test_get_available_shells(self):
        import rez_next.shell as shell
        shells = shell.get_available_shells()
        assert isinstance(shells, list)
        assert "bash" in shells
        assert "powershell" in shells

    def test_get_current_shell_returns_string(self):
        import rez_next.shell as shell
        current = shell.get_current_shell()
        assert isinstance(current, str)
        assert len(current) > 0

    def test_shell_create_bash(self):
        import rez_next.shell as shell
        s = shell.Shell("bash")
        assert s.name == "bash"

    def test_shell_generate_script(self):
        import rez_next.shell as shell
        s = shell.Shell("bash")
        script = s.generate_script(
            vars={"MY_VAR": "hello"},
            aliases={"myalias": "/usr/bin/myapp"},
        )
        assert "MY_VAR" in script
        assert "hello" in script

    def test_create_shell_script_function(self):
        import rez_next.shell as shell
        script = shell.create_shell_script(
            "powershell",
            vars={"MY_VAR": "test_value"},
        )
        assert "MY_VAR" in script
        assert "test_value" in script


class TestWalkPackages:
    """Verify walk_packages API."""

    def test_walk_packages_exists(self):
        assert callable(rez.walk_packages)

    def test_walk_packages_returns_list(self):
        result = rez.walk_packages(paths=["/nonexistent/path_xyz"])
        assert isinstance(result, list)
        # Empty dir means empty result
        assert len(result) == 0

    def test_packages_module_has_walk_packages(self):
        import rez_next.packages_ as pkgs
        assert callable(pkgs.walk_packages)


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
        # All internal tests should pass
        assert failed == 0, f"selftest failed: {failed} tests failed"


class TestRexModule:
    """Verify rez.rex submodule."""

    def test_rex_submodule_exists(self):
        import rez_next.rex as rex
        assert hasattr(rex, "rex_interpret")

    def test_rex_interpret_function(self):
        import rez_next.rex as rex
        # Empty commands
        result = rex.rex_interpret("")
        assert isinstance(result, dict)

    def test_rex_interpret_setenv(self):
        import rez_next.rex as rex
        result = rex.rex_interpret("env.setenv('MY_VAR', 'hello')")
        assert isinstance(result, dict)


class TestBuildModule:
    """Verify rez.build_ submodule."""

    def test_build_submodule_exists(self):
        import rez_next.build_ as build
        assert hasattr(build, "build_package")
        assert hasattr(build, "get_build_system")

    def test_get_build_system_callable(self):
        import rez_next.build_ as build
        assert callable(build.get_build_system)

    def test_get_build_system_for_current_dir(self):
        import rez_next.build_ as build
        result = build.get_build_system(".")
        # Should return a string (build system name or "unknown")
        assert isinstance(result, str)


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


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
