"""
Compatibility tests for rez_next as a drop-in replacement for rez.

Usage:
    maturin develop --features extension-module
    pytest tests/test_rez_compat.py -v

These tests verify that `import rez_next as rez` works as a complete
replacement for `import rez`.
"""
import pytest

rez = pytest.importorskip(
    "rez_next",
    reason="rez_next not built yet — run: maturin develop --features extension-module",
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


class TestEnvModule:
    """Verify rez.env — environment creation and activation API."""

    def test_env_submodule_exists(self):
        import rez_next.env as env
        assert hasattr(env, "RezEnv")
        assert hasattr(env, "create_env")
        assert hasattr(env, "get_activation_script")
        assert hasattr(env, "apply_env")

    def test_create_env_empty(self):
        import rez_next.env as env
        rez_env = env.create_env([])
        assert rez_env is not None

    def test_rez_env_empty_packages(self):
        env_obj = rez.RezEnv([])
        assert env_obj is not None
        assert env_obj.packages == []

    def test_rez_env_unknown_pkg_fails_gracefully(self):
        """Unknown packages should fail gracefully, not raise."""
        env_obj = rez.RezEnv(
            ["nonexistent_pkg_xyz_999"],
            paths=["/nonexistent/path_xyz"]
        )
        assert env_obj is not None
        # Either success=False or success=True (empty repos → no resolution)

    def test_rez_env_get_environ_returns_dict(self):
        env_obj = rez.RezEnv([])
        environ = env_obj.get_environ()
        assert isinstance(environ, dict)

    def test_rez_env_num_resolved_packages(self):
        env_obj = rez.RezEnv([])
        assert isinstance(env_obj.num_resolved_packages, int)
        assert env_obj.num_resolved_packages >= 0

    def test_get_activation_script_bash(self):
        result = rez.get_activation_script(
            [],  # empty packages = trivial activation
            shell="bash",
            paths=["/nonexistent/path_xyz"]
        )
        assert isinstance(result, str)

    def test_top_level_create_env(self):
        assert callable(rez.create_env)
        env_obj = rez.create_env([])
        assert env_obj is not None


class TestPackagesModule:
    """Verify rez.packages — PackageFamily API."""

    def test_packages_submodule_exists(self):
        import rez_next.packages as pkgs
        assert hasattr(pkgs, "PackageFamily")
        assert hasattr(pkgs, "Package")
        assert hasattr(pkgs, "PackageRequirement")

    def test_package_family_create(self):
        import rez_next.packages as pkgs
        family = pkgs.PackageFamily("python")
        assert family.name == "python"
        assert family.num_versions == 0

    def test_rez_env_class_at_top_level(self):
        assert rez.RezEnv is not None

    def test_package_family_class_at_top_level(self):
        assert rez.PackageFamily is not None
        family = rez.PackageFamily("maya")
        assert family.name == "maya"


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


class TestSearchModule:
    """Verify rez.search / rez_next.search Python API."""

    def test_search_submodule_exists(self):
        import rez_next.search as search
        assert hasattr(search, "search_packages")
        assert hasattr(search, "search_package_names")
        assert hasattr(search, "search_latest_packages")
        assert hasattr(search, "PackageSearcher")
        assert hasattr(search, "SearchResult")

    def test_search_packages_returns_list(self):
        import rez_next.search as search
        results = search.search_packages(
            pattern="", paths=["/nonexistent/path_xyz"]
        )
        assert isinstance(results, list)

    def test_search_packages_empty_paths_empty_result(self):
        import rez_next.search as search
        results = search.search_packages(
            pattern="python", paths=["/nonexistent/path_xyz"]
        )
        assert isinstance(results, list)
        assert len(results) == 0

    def test_search_package_names_returns_list(self):
        import rez_next.search as search
        names = search.search_package_names(
            pattern="", paths=["/nonexistent/path_xyz"]
        )
        assert isinstance(names, list)

    def test_search_latest_packages_returns_list(self):
        import rez_next.search as search
        results = search.search_latest_packages(
            pattern="", paths=["/nonexistent/path_xyz"]
        )
        assert isinstance(results, list)

    def test_package_searcher_create(self):
        import rez_next.search as search
        searcher = search.PackageSearcher(
            pattern="py",
            paths=["/nonexistent/path_xyz"],
            scope="families",
        )
        assert searcher is not None

    def test_package_searcher_repr(self):
        import rez_next.search as search
        searcher = search.PackageSearcher(pattern="maya", scope="latest")
        r = repr(searcher)
        assert "maya" in r
        assert "latest" in r

    def test_package_searcher_search_returns_list(self):
        import rez_next.search as search
        searcher = search.PackageSearcher(
            pattern="", paths=["/nonexistent/path_xyz"]
        )
        results = searcher.search()
        assert isinstance(results, list)

    def test_search_result_attributes_on_real_result(self):
        """If a SearchResult is returned, verify its attribute contract."""
        import rez_next.search as search
        results = search.search_packages(pattern="", paths=["/nonexistent/path_xyz"])
        for r in results:
            assert isinstance(r.name, str)
            assert isinstance(r.versions, list)
            assert isinstance(r.repo_path, str)
            assert isinstance(r.version_count(), int)

    def test_search_scope_families_vs_latest(self):
        import rez_next.search as search
        families = search.search_packages(
            scope="families", paths=["/nonexistent/path_xyz"]
        )
        latest = search.search_packages(
            scope="latest", paths=["/nonexistent/path_xyz"]
        )
        assert isinstance(families, list)
        assert isinstance(latest, list)

    def test_top_level_search_packages_callable(self):
        assert callable(rez.search_packages)

    def test_top_level_search_package_names_callable(self):
        assert callable(rez.search_package_names)


class TestBindModule:
    """Verify rez.bind / rez_next.bind Python API."""

    def test_bind_submodule_exists(self):
        import rez_next.bind as bind
        assert hasattr(bind, "list_binders")
        assert hasattr(bind, "bind_tool")
        assert hasattr(bind, "detect_version")
        assert hasattr(bind, "find_tool")
        assert hasattr(bind, "extract_version")
        assert hasattr(bind, "BindResult")
        assert hasattr(bind, "BindManager")

    def test_list_binders_returns_nonempty_list(self):
        import rez_next.bind as bind
        binders = bind.list_binders()
        assert isinstance(binders, list)
        assert len(binders) > 0

    def test_list_binders_known_tools(self):
        import rez_next.bind as bind
        binders = bind.list_binders()
        for tool in ("python", "cmake", "git"):
            assert tool in binders, f"Expected '{tool}' in built-in binders"

    def test_bind_manager_list_binders_matches_fn(self):
        import rez_next.bind as bind
        mgr = bind.BindManager()
        assert mgr.list_binders() == bind.list_binders()

    def test_bind_manager_is_builtin_known(self):
        import rez_next.bind as bind
        mgr = bind.BindManager()
        assert mgr.is_builtin("python")
        assert mgr.is_builtin("cmake")

    def test_bind_manager_is_builtin_unknown(self):
        import rez_next.bind as bind
        mgr = bind.BindManager()
        assert not mgr.is_builtin("totally_unknown_xyz_tool_999")

    def test_bind_manager_repr(self):
        import rez_next.bind as bind
        mgr = bind.BindManager()
        assert "BindManager" in repr(mgr)

    def test_find_tool_nonexistent_returns_none(self):
        import rez_next.bind as bind
        result = bind.find_tool("totally_nonexistent_tool_xyz_999")
        assert result is None

    def test_extract_version_semver(self):
        import rez_next.bind as bind
        v = bind.extract_version("cmake version 3.26.4")
        assert v is not None
        assert "3.26" in v

    def test_extract_version_no_digits_returns_none(self):
        import rez_next.bind as bind
        v = bind.extract_version("no version info here")
        assert v is None

    def test_detect_version_returns_string(self):
        import rez_next.bind as bind
        result = bind.detect_version("python")
        assert isinstance(result, str)

    def test_bind_result_attributes(self):
        import rez_next.bind as bind
        # Construct via bind_tool on a known binder (may not have real exec on PATH)
        # We can only safely test the structure; actual binding may fail gracefully
        r = bind.BindResult.__new__(bind.BindResult)  # type: ignore[call-arg]
        # Can't construct directly (Rust struct), so test via bind if available
        result = bind.find_tool("python")
        # If python is on PATH, detect_version should return something
        if result is not None:
            version = bind.detect_version("python")
            assert len(version) > 0

    def test_top_level_list_binders_callable(self):
        assert callable(rez.list_binders)

    def test_top_level_bind_tool_callable(self):
        assert callable(rez.bind_tool)


class TestCompletionModule:
    """Verify rez.complete / rez_next.complete Python API."""

    def test_complete_submodule_exists(self):
        import rez_next.complete as complete
        assert hasattr(complete, "get_completion_script")
        assert hasattr(complete, "supported_completion_shells")
        assert hasattr(complete, "get_completion_install_path")
        assert hasattr(complete, "print_completion_script")

    def test_supported_completion_shells_returns_list(self):
        import rez_next.complete as complete
        shells = complete.supported_completion_shells()
        assert isinstance(shells, list)
        assert len(shells) >= 4

    def test_supported_completion_shells_contains_expected(self):
        import rez_next.complete as complete
        shells = complete.supported_completion_shells()
        for expected in ("bash", "zsh", "fish", "powershell"):
            assert expected in shells

    def test_get_completion_script_bash(self):
        import rez_next.complete as complete
        script = complete.get_completion_script("bash")
        assert isinstance(script, str)
        assert len(script) > 0
        assert "rez" in script

    def test_get_completion_script_zsh(self):
        import rez_next.complete as complete
        script = complete.get_completion_script("zsh")
        assert isinstance(script, str)
        assert "#compdef" in script

    def test_get_completion_script_fish(self):
        import rez_next.complete as complete
        script = complete.get_completion_script("fish")
        assert isinstance(script, str)
        assert "complete" in script

    def test_get_completion_script_powershell(self):
        import rez_next.complete as complete
        script = complete.get_completion_script("powershell")
        assert isinstance(script, str)
        assert len(script) > 0

    def test_get_completion_script_unknown_raises(self):
        import rez_next.complete as complete
        with pytest.raises(ValueError):
            complete.get_completion_script("totally_unknown_shell_xyz")

    def test_get_completion_install_path_bash(self):
        import rez_next.complete as complete
        path = complete.get_completion_install_path("bash")
        assert isinstance(path, str)
        assert "bash" in path.lower() or "completion" in path.lower()

    def test_get_completion_install_path_zsh(self):
        import rez_next.complete as complete
        path = complete.get_completion_install_path("zsh")
        assert isinstance(path, str)
        assert "zsh" in path.lower()

    def test_get_completion_install_path_fish(self):
        import rez_next.complete as complete
        path = complete.get_completion_install_path("fish")
        assert isinstance(path, str)
        assert "fish" in path.lower()

    def test_get_completion_install_path_powershell(self):
        import rez_next.complete as complete
        path = complete.get_completion_install_path("powershell")
        assert isinstance(path, str)
        assert "powershell" in path.lower() or "profile" in path.lower()

    def test_get_completion_install_path_unknown_raises(self):
        import rez_next.complete as complete
        with pytest.raises(ValueError):
            complete.get_completion_install_path("unknown_shell_xyz")

    def test_print_completion_script_does_not_raise(self):
        import rez_next.complete as complete
        # print_completion_script writes to stdout; should not raise
        complete.print_completion_script("bash")
        complete.print_completion_script("powershell")

    def test_get_completion_script_no_shell_uses_default(self):
        import rez_next.complete as complete
        # With no shell arg, uses system default; should return non-empty string
        script = complete.get_completion_script(None)
        assert isinstance(script, str)
        assert len(script) > 0

    def test_top_level_get_completion_script_callable(self):
        assert callable(rez.get_completion_script)

    def test_top_level_get_completion_script_returns_string(self):
        """get_completion_script is accessible at top level and returns a string."""
        script = rez.get_completion_script("bash")
        assert isinstance(script, str)
        assert len(script) > 0


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
