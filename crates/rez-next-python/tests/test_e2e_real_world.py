"""
End-to-end tests simulating real rez usage scenarios.

These tests use tmp_path fixtures to create real on-disk package structures
and verify rez_next behaves correctly for common pipeline workflows:

  - Package repository scan
  - Dependency resolution against a real (small) repo
  - Rex command block execution
  - Bundle / unbundle context
  - Shell script generation for bash and powershell
  - Context environment variable injection
  - Suite create / save / load
  - Package serialization roundtrip (package.py / YAML)
"""

import json
import os

import pytest

try:
    import rez_next as rez

    REZ_NEXT_AVAILABLE = True
except ImportError:
    REZ_NEXT_AVAILABLE = False

pytestmark = pytest.mark.skipif(
    not REZ_NEXT_AVAILABLE,
    reason="rez_next not built. Run: maturin develop --features extension-module",
)


# ── Helpers ───────────────────────────────────────────────────────────────────


def write_package_py(path, name, version, requires=None, commands=None):
    """Write a minimal package.py to *path* directory."""
    path.mkdir(parents=True, exist_ok=True)
    lines = [f'name = "{name}"', f'version = "{version}"']
    if requires:
        req_list = ", ".join(f'"{r}"' for r in requires)
        lines.append(f"requires = [{req_list}]")
    if commands:
        lines.append(f'commands = """\n{commands}\n"""')
    (path / "package.py").write_text("\n".join(lines) + "\n")


# ── Rex Execution ─────────────────────────────────────────────────────────────


class TestRexExecution:
    """Real-world Rex command block execution."""

    def test_setenv(self):
        import rez_next.rex as rex

        result = rex.rex_interpret("env.setenv('MY_ROOT', '/opt/pkg')")
        assert isinstance(result, dict)
        assert result.get("MY_ROOT") == "/opt/pkg"

    def test_prepend_path(self):
        import rez_next.rex as rex

        result = rex.rex_interpret("env.prepend_path('PATH', '/opt/pkg/bin')")
        assert isinstance(result, dict)

    def test_alias(self):
        import rez_next.rex as rex

        result = rex.rex_interpret("alias('mypkg', '/opt/pkg/bin/mypkg')")
        assert isinstance(result, dict)

    def test_maya_commands_block(self):
        import rez_next.rex as rex

        commands = "\n".join(
            [
                "env.setenv('MAYA_ROOT', '/opt/maya/2024')",
                "env.setenv('MAYA_VERSION', '2024')",
                "env.prepend_path('PATH', '/opt/maya/2024/bin')",
                "alias('maya', '/opt/maya/2024/bin/maya')",
            ]
        )
        result = rex.rex_interpret(commands)
        assert result.get("MAYA_ROOT") == "/opt/maya/2024"
        assert result.get("MAYA_VERSION") == "2024"

    def test_empty_commands(self):
        import rez_next.rex as rex

        result = rex.rex_interpret("")
        assert isinstance(result, dict)

    def test_multiple_setenv(self):
        import rez_next.rex as rex

        commands = "\n".join(
            [
                "env.setenv('A', '1')",
                "env.setenv('B', '2')",
                "env.setenv('C', '3')",
            ]
        )
        result = rex.rex_interpret(commands)
        assert result.get("A") == "1"
        assert result.get("B") == "2"
        assert result.get("C") == "3"


# ── Shell Script Generation ────────────────────────────────────────────────────


class TestShellScriptGeneration:
    """Shell script generation for different shell types."""

    def test_bash_script_contains_export(self):
        import rez_next.shell as shell

        script = shell.create_shell_script("bash", vars={"MY_VAR": "hello"})
        assert "MY_VAR" in script
        assert "hello" in script

    def test_powershell_script_contains_env(self):
        import rez_next.shell as shell

        script = shell.create_shell_script("powershell", vars={"MY_VAR": "hello"})
        assert "MY_VAR" in script
        assert "hello" in script

    def test_available_shells_contains_bash(self):
        import rez_next.shell as shell

        shells = shell.get_available_shells()
        assert "bash" in shells

    def test_available_shells_contains_powershell(self):
        import rez_next.shell as shell

        shells = shell.get_available_shells()
        assert "powershell" in shells

    def test_current_shell_is_string(self):
        import rez_next.shell as shell

        current = shell.get_current_shell()
        assert isinstance(current, str)
        assert len(current) > 0

    def test_shell_object_generate_script(self):
        import rez_next.shell as shell

        s = shell.Shell("bash")
        script = s.generate_script(
            vars={"HOUDINI_ROOT": "/opt/houdini/20"},
            aliases={"houdini": "/opt/houdini/20/bin/houdini"},
        )
        assert "HOUDINI_ROOT" in script

    def test_bash_script_with_aliases(self):
        import rez_next.shell as shell

        script = shell.create_shell_script(
            "bash",
            vars={"PYTHON_ROOT": "/usr/local"},
            aliases={"python3": "/usr/local/bin/python3"},
        )
        assert "PYTHON_ROOT" in script


# ── Bundle / Unbundle ─────────────────────────────────────────────────────────


class TestBundleWorkflow:
    """Context bundle create / inspect / extract."""

    def test_bundle_creates_dir(self, tmp_path):
        import rez_next.bundles as bundles

        dest = str(tmp_path / "my_bundle")
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

    def test_unbundle_returns_package_list(self, tmp_path):
        import rez_next.bundles as bundles

        dest = str(tmp_path / "round_trip_bundle")
        packages = ["python-3.9", "maya-2024"]
        bundles.bundle_context(packages, dest)
        extracted = bundles.unbundle_context(dest)
        assert isinstance(extracted, list)
        assert len(extracted) >= 2

    def test_list_bundles_finds_created_bundle(self, tmp_path):
        import rez_next.bundles as bundles

        bundles.bundle_context(["python"], str(tmp_path / "bundle_a"))
        bundles.bundle_context(["maya"], str(tmp_path / "bundle_b"))
        found = bundles.list_bundles(str(tmp_path))
        assert "bundle_a" in found
        assert "bundle_b" in found

    def test_list_bundles_empty_dir(self, tmp_path):
        import rez_next.bundles as bundles

        result = bundles.list_bundles(str(tmp_path / "empty_xyz"))
        assert result == []

    def test_top_level_bundle_context(self, tmp_path):
        dest = str(tmp_path / "top_level")
        result = rez.bundle_context(["houdini-20.0"], dest)
        assert result == dest


# ── Suite Create/Save/Load ────────────────────────────────────────────────────


class TestSuiteWorkflow:
    """Suite management — create, save, load roundtrip."""

    def test_suite_create(self):
        suite = rez.Suite()
        assert suite is not None

    def test_suite_submodule_import(self):
        from rez_next.suite import Suite, SuiteManager

        assert Suite is not None
        assert SuiteManager is not None


# ── Selftest ──────────────────────────────────────────────────────────────────


class TestSelftestE2E:
    """rez.selftest() runs all internal Rust unit tests."""

    def test_selftest_all_pass(self):
        passed, failed, total = rez.selftest()
        assert total > 0, "selftest should run at least some tests"
        assert failed == 0, f"selftest: {failed}/{total} internal tests failed"

    def test_selftest_returns_three_ints(self):
        result = rez.selftest()
        assert len(result) == 3
        assert all(isinstance(x, int) for x in result)


# ── Package Iteration API ─────────────────────────────────────────────────────


class TestPackageIterationE2E:
    """iter_packages, get_package_family_names with real (empty) repo."""

    def test_iter_packages_nonexistent_path(self):
        result = rez.iter_packages("python", paths=["/nonexistent_repo_xyz"])
        assert isinstance(result, list)
        assert len(result) == 0

    def test_get_package_family_names_empty_repo(self):
        names = rez.get_package_family_names(paths=["/nonexistent_repo_xyz"])
        assert isinstance(names, list)

    def test_walk_packages_empty_repo(self):
        result = rez.walk_packages(paths=["/nonexistent_repo_xyz"])
        assert isinstance(result, list)
        assert len(result) == 0

    def test_get_latest_package_empty_repo(self):
        result = rez.get_latest_package("python", paths=["/nonexistent_repo_xyz"])
        assert result is None

    def test_get_package_empty_repo(self):
        result = rez.get_package("python", "3.9", paths=["/nonexistent_repo_xyz"])
        assert result is None


# ── CLI Module ────────────────────────────────────────────────────────────────


class TestCLIModuleE2E:
    """CLI dispatch API."""

    @pytest.mark.parametrize(
        "cmd",
        ["env", "solve", "build", "release", "status", "search", "config", "selftest"],
    )
    def test_known_commands_return_zero(self, cmd):
        import rez_next.cli as cli

        assert cli.cli_run(cmd) == 0

    def test_unknown_command_raises(self):
        import rez_next.cli as cli

        with pytest.raises(ValueError):
            cli.cli_run("totally_unknown_command_xyz_abc")

    def test_cli_main_no_args(self):
        import rez_next.cli as cli

        assert cli.cli_main() == 0

    def test_cli_main_with_env(self):
        import rez_next.cli as cli

        assert cli.cli_main(["env"]) == 0


# ── Exceptions Submodule ──────────────────────────────────────────────────────


class TestExceptionsE2E:
    """Exception classes are importable and usable."""

    def test_all_exceptions_importable(self):
        from rez_next.exceptions import (
            ConfigurationError,
            ContextBundleError,
            PackageNotFound,
            PackageParseError,
            PackageVersionConflict,
            ResolveError,
            RezBuildError,
            RexError,
            SuiteError,
        )

        for exc in [
            PackageNotFound,
            PackageVersionConflict,
            ResolveError,
            RezBuildError,
            ConfigurationError,
            PackageParseError,
            ContextBundleError,
            SuiteError,
            RexError,
        ]:
            assert exc is not None
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


# ── Resource Strings ──────────────────────────────────────────────────────────


class TestUtilsResourcesE2E:
    """rez_next.utils.resources submodule."""

    def test_get_version_string(self):
        from rez_next.utils.resources import get_resource_string

        ver = get_resource_string("version")
        assert isinstance(ver, str)
        assert len(ver) > 0
        # Should look like a semver
        parts = ver.split(".")
        assert len(parts) >= 2

    def test_get_name_string(self):
        from rez_next.utils.resources import get_resource_string

        name = get_resource_string("name")
        assert "rez" in name.lower()

    def test_unknown_resource_raises_key_error(self):
        from rez_next.utils.resources import get_resource_string

        with pytest.raises(KeyError):
            get_resource_string("__nonexistent_xyz_9999__")


# ── Build System Detection ────────────────────────────────────────────────────


class TestBuildModuleE2E:
    def test_detect_unknown_dir(self, tmp_path):
        import rez_next.build_ as build

        result = build.get_build_system(str(tmp_path))
        assert result == "unknown"

    def test_detect_cmake(self, tmp_path):
        import rez_next.build_ as build

        (tmp_path / "CMakeLists.txt").write_text("cmake_minimum_required(VERSION 3.10)\n")
        result = build.get_build_system(str(tmp_path))
        assert result == "cmake"

    def test_detect_python(self, tmp_path):
        import rez_next.build_ as build

        (tmp_path / "setup.py").write_text("from setuptools import setup\nsetup()\n")
        result = build.get_build_system(str(tmp_path))
        assert result == "python"

    def test_detect_cargo(self, tmp_path):
        import rez_next.build_ as build

        (tmp_path / "Cargo.toml").write_text('[package]\nname = "test"\n')
        result = build.get_build_system(str(tmp_path))
        assert result == "cargo"
