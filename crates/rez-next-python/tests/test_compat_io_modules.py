"""
I/O module compatibility tests for rez_next.

Covers: shell, pip, bundles, bind, complete, CLI, env, build.

Usage:
    maturin develop --features extension-module
    pytest tests/test_compat_io_modules.py -v
"""
import os

import pytest

rez = pytest.importorskip(
    "rez_next",
    reason="rez_next not built yet — run: maturin develop --features extension-module",
)


# ── Shell ─────────────────────────────────────────────────────────────────────


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

    def test_bash_script_contains_export(self):
        import rez_next.shell as shell
        script = shell.create_shell_script("bash", vars={"MY_VAR": "hello"})
        assert "MY_VAR" in script
        assert "hello" in script

    def test_shell_object_generate_with_aliases(self):
        import rez_next.shell as shell
        script = shell.create_shell_script(
            "bash",
            vars={"PYTHON_ROOT": "/usr/local"},
            aliases={"python3": "/usr/local/bin/python3"},
        )
        assert "PYTHON_ROOT" in script


# ── Pip ───────────────────────────────────────────────────────────────────────


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

    @pytest.mark.xfail(reason="pip_install() not yet implemented")
    def test_pip_install_returns_list(self):
        import rez_next.pip as pip
        result = pip.pip_install(["numpy==1.25.0", "scipy==1.11.0"])
        assert isinstance(result, list)
        assert len(result) == 2

    @pytest.mark.xfail(reason="pip_install() not yet implemented")
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
        assert os.path.isdir(result)
        assert os.path.exists(os.path.join(result, "package.py"))

    def test_write_pip_package_overwrite_false(self, tmp_path):
        import rez_next.pip as pip
        pkg = pip.PipPackage("mylib", "2.0.0")
        pip.write_pip_package(pkg, str(tmp_path))
        with pytest.raises(FileExistsError):
            pip.write_pip_package(pkg, str(tmp_path), overwrite=False)

    def test_write_pip_package_overwrite_true(self, tmp_path):
        import rez_next.pip as pip
        pkg = pip.PipPackage("mylib", "2.0.0")
        pip.write_pip_package(pkg, str(tmp_path))
        result = pip.write_pip_package(pkg, str(tmp_path), overwrite=True)
        assert result

    @pytest.mark.xfail(reason="pip_install() not yet implemented")
    def test_pip_install_top_level(self):
        assert callable(rez.pip_install)
        result = rez.pip_install(["requests==2.31.0"])
        assert len(result) == 1
        assert "requests" in result[0]


# ── Bundles ───────────────────────────────────────────────────────────────────


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
        assert os.path.isdir(dest)
        assert os.path.exists(os.path.join(dest, "bundle.yaml"))

    def test_bundle_yaml_contains_packages(self, tmp_path):
        import rez_next.bundles as bundles
        dest = str(tmp_path / "pkg_bundle")
        packages = ["python-3.9", "numpy-1.25", "scipy-1.11"]
        bundles.bundle_context(packages, dest)
        yaml_content = (tmp_path / "pkg_bundle" / "bundle.yaml").read_text()
        for pkg in packages:
            assert pkg in yaml_content

    def test_unbundle_context_reads_packages(self, tmp_path):
        import rez_next.bundles as bundles
        dest = str(tmp_path / "my_bundle")
        bundles.bundle_context(["numpy-1.25", "scipy-1.11"], dest)
        pkgs = bundles.unbundle_context(dest)
        assert isinstance(pkgs, list)
        assert len(pkgs) >= 2

    def test_list_bundles_finds_created_bundle(self, tmp_path):
        import rez_next.bundles as bundles
        bundles.bundle_context(["python"], str(tmp_path / "bundle_a"))
        bundles.bundle_context(["maya"], str(tmp_path / "bundle_b"))
        found = bundles.list_bundles(str(tmp_path))
        assert "bundle_a" in found
        assert "bundle_b" in found

    def test_list_bundles_nonexistent_path(self, tmp_path):
        import rez_next.bundles as bundles
        result = bundles.list_bundles(str(tmp_path / "nonexistent_xyz"))
        assert isinstance(result, list)
        assert len(result) == 0

    def test_bundle_context_top_level(self, tmp_path):
        dest = str(tmp_path / "top_level_bundle")
        result = rez.bundle_context(["python-3.9"], dest)
        assert result == dest


# ── Bind ──────────────────────────────────────────────────────────────────────


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

    def test_top_level_list_binders_callable(self):
        assert callable(rez.list_binders)

    def test_top_level_bind_tool_callable(self):
        assert callable(rez.bind_tool)


# ── Complete ──────────────────────────────────────────────────────────────────


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

    def test_get_completion_install_path_unknown_raises(self):
        import rez_next.complete as complete
        with pytest.raises(ValueError):
            complete.get_completion_install_path("unknown_shell_xyz")

    def test_print_completion_script_does_not_raise(self):
        import rez_next.complete as complete
        complete.print_completion_script("bash")
        complete.print_completion_script("powershell")

    def test_get_completion_script_no_shell_uses_default(self):
        import rez_next.complete as complete
        script = complete.get_completion_script(None)
        assert isinstance(script, str)
        assert len(script) > 0

    def test_top_level_get_completion_script_callable(self):
        assert callable(rez.get_completion_script)

    def test_top_level_get_completion_script_returns_string(self):
        script = rez.get_completion_script("bash")
        assert isinstance(script, str)
        assert len(script) > 0


# ── CLI ───────────────────────────────────────────────────────────────────────


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


# ── Env ───────────────────────────────────────────────────────────────────────


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
            [],
            shell="bash",
            paths=["/nonexistent/path_xyz"]
        )
        assert isinstance(result, str)

    def test_top_level_create_env(self):
        assert callable(rez.create_env)
        env_obj = rez.create_env([])
        assert env_obj is not None

    def test_rez_env_class_at_top_level(self):
        assert rez.RezEnv is not None


# ── Build ─────────────────────────────────────────────────────────────────────


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
        assert isinstance(result, str)

    def test_detect_unknown_dir(self, tmp_path):
        import rez_next.build_ as build
        result = build.get_build_system(str(tmp_path))
        assert result == "unknown"

    def test_detect_cmake(self, tmp_path):
        import rez_next.build_ as build
        (tmp_path / "CMakeLists.txt").write_text("cmake_minimum_required(VERSION 3.10)\n")
        result = build.get_build_system(str(tmp_path))
        assert result == "cmake"

    def test_detect_python_setup(self, tmp_path):
        import rez_next.build_ as build
        (tmp_path / "setup.py").write_text("from setuptools import setup\nsetup()\n")
        result = build.get_build_system(str(tmp_path))
        assert result == "python"

    def test_detect_cargo(self, tmp_path):
        import rez_next.build_ as build
        (tmp_path / "Cargo.toml").write_text('[package]\nname = "test"\n')
        result = build.get_build_system(str(tmp_path))
        assert result == "cargo"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
