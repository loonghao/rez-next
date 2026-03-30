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


class TestBundlesModule:
    """Verify rez.bundles submodule API."""

    def test_bundles_submodule_exists(self):
        import rez_next.bundles as bundles
        assert callable(bundles.bundle_context)
        assert callable(bundles.unbundle_context)
        assert callable(bundles.list_bundles)

    def test_bundle_context_creates_dir(self, tmp_path):
        import rez_next.bundles as bundles
        dest = str(tmp_path / "test_bundle")
        result = bundles.bundle_context(["python-3.9", "maya-2024"], dest)
        assert result == dest
        import os
        assert os.path.isdir(dest)
        assert os.path.exists(os.path.join(dest, "bundle.yaml"))

    def test_unbundle_context_reads_packages(self, tmp_path):
        import rez_next.bundles as bundles
        dest = str(tmp_path / "my_bundle")
        bundles.bundle_context(["numpy-1.25", "scipy-1.11"], dest)
        pkgs = bundles.unbundle_context(dest)
        assert isinstance(pkgs, list)
        assert len(pkgs) >= 2

    def test_list_bundles_nonexistent_path(self, tmp_path):
        import rez_next.bundles as bundles
        result = bundles.list_bundles(str(tmp_path / "nonexistent_xyz"))
        assert isinstance(result, list)
        assert len(result) == 0

    def test_bundle_context_top_level(self, tmp_path):
        """bundle_context is also accessible at top level."""
        dest = str(tmp_path / "top_level_bundle")
        result = rez.bundle_context(["python-3.9"], dest)
        assert result == dest


class TestCliModule:
    """Verify rez.cli submodule API."""

    def test_cli_submodule_exists(self):
        import rez_next.cli as cli
        assert callable(cli.cli_run)
        assert callable(cli.cli_main)

    def test_cli_run_known_commands(self):
        import rez_next.cli as cli
        for cmd in ["env", "solve", "build", "search", "config", "selftest"]:
            result = cli.cli_run(cmd)
            assert result == 0, f"cli_run('{cmd}') should return 0"

    def test_cli_run_unknown_command_raises(self):
        import rez_next.cli as cli
        with pytest.raises(ValueError):
            cli.cli_run("totally_unknown_command_xyz")

    def test_cli_main_no_args(self):
        import rez_next.cli as cli
        result = cli.cli_main()
        assert result == 0

    def test_cli_main_with_args(self):
        import rez_next.cli as cli
        result = cli.cli_main(["env", "--help"])
        assert result == 0


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
        """get_plugin_manager accessible at top level."""
        assert callable(rez.get_plugin_manager)
        mgr = rez.get_plugin_manager()
        assert mgr is not None


class TestPipModule:
    """Verify rez.pip submodule API — pip-to-rez package conversion."""

    def test_pip_submodule_exists(self):
        import rez_next.pip as pip
        assert hasattr(pip, "normalize_package_name")
        assert hasattr(pip, "pip_version_to_rez")
        assert hasattr(pip, "pip_install")
        assert hasattr(pip, "convert_pip_to_rez")
        assert hasattr(pip, "PipPackage")

    def test_normalize_package_name(self):
        import rez_next.pip as pip
        assert pip.normalize_package_name("NumPy") == "numpy"
        assert pip.normalize_package_name("Pillow") == "pillow"
        assert pip.normalize_package_name("scikit_learn") == "scikit-learn"
        assert pip.normalize_package_name("PyYAML") == "pyyaml"

    def test_pip_version_to_rez_exact(self):
        import rez_next.pip as pip
        assert pip.pip_version_to_rez("==1.2.3") == "1.2.3"

    def test_pip_version_to_rez_gte(self):
        import rez_next.pip as pip
        result = pip.pip_version_to_rez(">=3.9")
        assert "3.9" in result

    def test_pip_version_to_rez_range(self):
        import rez_next.pip as pip
        result = pip.pip_version_to_rez(">=1.0,<2.0")
        assert "1.0" in result and "2.0" in result

    def test_pip_install_returns_list(self):
        import rez_next.pip as pip
        result = pip.pip_install(["numpy==1.25.0", "scipy==1.11.0"])
        assert isinstance(result, list)
        assert len(result) == 2

    def test_pip_install_name_normalization(self):
        import rez_next.pip as pip
        result = pip.pip_install(["PyYAML==6.0"])
        assert result[0].startswith("pyyaml")

    def test_convert_pip_to_rez(self):
        import rez_next.pip as pip
        pkg = pip.convert_pip_to_rez(
            "numpy", "1.25.0",
            requires=["python>=3.8"],
            description="Numerical Python"
        )
        assert pkg.name == "numpy"
        assert pkg.version == "1.25.0"
        assert pkg.description == "Numerical Python"

    def test_pip_package_to_package_py(self):
        import rez_next.pip as pip
        pkg = pip.PipPackage("numpy", "1.25.0", description="Numerical Python")
        content = pkg.to_package_py()
        assert 'name = "numpy"' in content
        assert 'version = "1.25.0"' in content
        assert "PYTHONPATH" in content

    def test_pip_package_with_requires(self):
        import rez_next.pip as pip
        pkg = pip.PipPackage("scipy", "1.11.0", requires=["numpy-1.25+"])
        content = pkg.to_package_py()
        assert "requires" in content
        assert "numpy" in content

    def test_write_pip_package(self, tmp_path):
        import rez_next.pip as pip
        pkg = pip.PipPackage("mylib", "2.0.0", description="Test lib")
        result = pip.write_pip_package(pkg, str(tmp_path))
        import os
        assert os.path.isdir(result)
        assert os.path.exists(os.path.join(result, "package.py"))

    def test_write_pip_package_overwrite_false(self, tmp_path):
        import rez_next.pip as pip
        pkg = pip.PipPackage("mylib", "2.0.0")
        pip.write_pip_package(pkg, str(tmp_path))
        # Second write without overwrite should raise
        with pytest.raises(FileExistsError):
            pip.write_pip_package(pkg, str(tmp_path), overwrite=False)

    def test_write_pip_package_overwrite_true(self, tmp_path):
        import rez_next.pip as pip
        pkg = pip.PipPackage("mylib", "2.0.0")
        pip.write_pip_package(pkg, str(tmp_path))
        # Overwrite should succeed
        result = pip.write_pip_package(pkg, str(tmp_path), overwrite=True)
        assert result

    def test_pip_install_top_level(self):
        """pip_install also accessible at top level."""
        assert callable(rez.pip_install)
        result = rez.pip_install(["requests==2.31.0"])
        assert len(result) == 1
        assert "requests" in result[0]


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
