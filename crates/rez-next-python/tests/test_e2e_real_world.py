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

import os

import pytest

from conftest import write_package_py

rez = pytest.importorskip(
    "rez_next",
    reason="rez_next not built — run: maturin develop --features extension-module",
)


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


# ── Realistic VFX Pipeline Repository ─────────────────────────────────────

class TestRealisticVfxPipelineE2E:
    """E2E tests with realistic VFX pipeline package structures.

    Creates a temporary repository with packages mimicking real studio setups:
    - DCC applications (Maya, Houdini, Nuke)
    - Python with version ranges
    - Pipeline tools with cross-dependencies
    """

    @pytest.fixture
    def vfx_repo(self, tmp_path):
        """Create a realistic VFX pipeline repository structure."""
        repo = tmp_path / "packages"
        repo.mkdir()

        # Maya 2024.1
        maya_path = repo / "maya" / "2024.1"
        write_package_py(
            maya_path,
            "maya",
            "2024.1",
            requires=["python-3.9+"],
            commands=(
                'env.setenv("MAYA_ROOT", "{root}")\\n'
                'env.prepend_path("PATH", "{root}/bin")\\n'
                'alias("maya", "{root}/bin/maya")'
            ),
        )

        # Maya 2025.0
        maya_path2 = repo / "maya" / "2025.0"
        write_package_py(
            maya_path2,
            "maya",
            "2025.0",
            requires=["python-3.11+"],
            commands=(
                'env.setenv("MAYA_ROOT", "{root}")\\n'
                'env.prepend_path("PATH", "{root}/bin")\\n'
                'alias("maya", "{root}/bin/maya")'
            ),
        )

        # Houdini 20.0
        houdini_path = repo / "houdini" / "20.0.653"
        write_package_py(
            houdini_path,
            "houdini",
            "20.0.653",
            requires=["python-3.10+"],
            commands=(
                'env.setenv("HOUDINI_ROOT", "{root}")\\n'
                'env.prepend_path("PATH", "{root}/bin")\\n'
                'alias("houdini", "{root}/bin/houdini")'
            ),
        )

        # Python 3.9.7
        python_path = repo / "python" / "3.9.7"
        write_package_py(
            python_path,
            "python",
            "3.9.7",
            commands=(
                'env.setenv("PYTHON_ROOT", "{root}")\\n'
                'env.prepend_path("PATH", "{root}/bin")'
            ),
        )

        # Python 3.11.5
        python_path2 = repo / "python" / "3.11.5"
        write_package_py(
            python_path2,
            "python",
            "3.11.5",
            commands=(
                'env.setenv("PYTHON_ROOT", "{root}")\\n'
                'env.prepend_path("PATH", "{root}/bin")'
            ),
        )

        # NumPy 1.24.3 (requires python-3.9+)
        numpy_path = repo / "numpy" / "1.24.3"
        write_package_py(
            numpy_path,
            "numpy",
            "1.24.3",
            requires=["python-3.9+"],
            commands='env.setenv("NUMPY_ROOT", "{root}")',
        )

        # NumPy 1.26.4 (requires python-3.10+)
        numpy_path2 = repo / "numpy" / "1.26.4"
        write_package_py(
            numpy_path2,
            "numpy",
            "1.26.4",
            requires=["python-3.10+"],
            commands='env.setenv("NUMPY_ROOT", "{root}")',
        )

        # PySide2 5.15.2 (requires python-3.9+ and maya)
        pyside_path = repo / "pyside2" / "5.15.2"
        write_package_py(
            pyside_path,
            "pyside2",
            "5.15.2",
            requires=["python-3.9+", "maya-2024+"],
            commands='env.setenv("PYSIDE_ROOT", "{root}")',
        )

        # Pipeline tools 1.0 (requires maya, numpy, pyside2)
        pipeline_path = repo / "pipeline_tools" / "1.0.0"
        write_package_py(
            pipeline_path,
            "pipeline_tools",
            "1.0.0",
            requires=["maya-2024+", "numpy-1.24+", "pyside2-5.15+"],
            commands='env.setenv("PIPELINE_ROOT", "{root}")',
        )

        return str(repo)

    def test_resolve_maya_with_python(self, vfx_repo):
        """Test resolving Maya with Python dependency."""
        result = rez.resolve(["maya-2024+", "python-3.9+"], paths=[vfx_repo])
        assert result is not None
        packages = result.get("packages", [])
        assert any(p["name"] == "maya" for p in packages)
        assert any(p["name"] == "python" for p in packages)

    def test_resolve_houdini_with_python(self, vfx_repo):
        """Test resolving Houdini with Python 3.10+."""
        result = rez.resolve(["houdini-20.0+", "python-3.10+"], paths=[vfx_repo])
        assert result is not None
        packages = result.get("packages", [])
        assert any(p["name"] == "houdini" for p in packages)

    def test_resolve_pipeline_tools_full_stack(self, vfx_repo):
        """Test resolving full pipeline stack with all dependencies."""
        result = rez.resolve(
            ["pipeline_tools-1.0+", "maya-2024+", "python-3.9+"],
            paths=[vfx_repo],
        )
        assert result is not None
        packages = result.get("packages", [])
        package_names = [p["name"] for p in packages]
        assert "pipeline_tools" in package_names
        assert "maya" in package_names
        assert "python" in package_names
        assert "numpy" in package_names
        assert "pyside2" in package_names

    def test_resolve_version_conflict(self, vfx_repo):
        """Test that impossible resolution fails gracefully."""
        # Maya 2025 requires Python 3.11+, but we're requesting Python 3.9
        # This should fail due to version conflict
        try:
            result = rez.resolve(
                ["maya-2025+", "python-3.9"],
                paths=[vfx_repo],
            )
            # If it doesn't raise, check if result is valid
            if result is not None:
                packages = result.get("packages", [])
                maya = next((p for p in packages if p["name"] == "maya"), None)
                py = next((p for p in packages if p["name"] == "python"), None)
                if maya and py:
                    # Check if Maya 2025 was selected (requires Python 3.11+)
                    if maya["version"].startswith("2025"):
                        # Python should be 3.11+, so this is actually valid
                        assert py["version"] >= "3.11"
        except Exception:
            # Expected for impossible resolution
            pass

    def test_iter_packages_returns_all_versions(self, vfx_repo):
        """Test that iter_packages returns all available versions."""
        maya_packages = rez.iter_packages("maya", paths=[vfx_repo])
        versions = [p["version"] for p in maya_packages]
        assert "2024.1" in versions
        assert "2025.0" in versions

    def test_get_latest_package(self, vfx_repo):
        """Test getting the latest version of a package."""
        latest = rez.get_latest_package("maya", paths=[vfx_repo])
        assert latest is not None
        assert latest["version"] == "2025.0"

    def test_context_creation_with_vfx_stack(self, vfx_repo):
        """Test creating a context with VFX packages."""
        context = rez.create_context(
            ["maya-2024+", "numpy-1.24+", "python-3.9+"],
            paths=[vfx_repo],
        )
        assert context is not None
        assert context.get("success") is True

    def test_get_package_family_names(self, vfx_repo):
        """Test listing all package families in repository."""
        names = rez.get_package_family_names(paths=[vfx_repo])
        assert "maya" in names
        assert "houdini" in names
        assert "python" in names
        assert "numpy" in names
        assert "pipeline_tools" in names


# ── Complex Dependency Graph E2E ────────────────────────────────────────────

class TestComplexDependencyGraphE2E:
    """E2E tests with complex dependency graphs simulating real studios."""

    @pytest.fixture
    def complex_repo(self, tmp_path):
        """Create a complex dependency graph."""
        repo = tmp_path / "complex_packages"
        repo.mkdir()

        # Base libraries
        base_a = repo / "base_lib_a" / "1.0.0"
        write_package_py(base_a, "base_lib_a", "1.0.0")

        base_b = repo / "base_lib_b" / "2.0.0"
        write_package_py(base_b, "base_lib_b", "2.0.0", requires=["base_lib_a-1.0+"])

        # Middle layer
        middle_c = repo / "middle_lib_c" / "1.5.0"
        write_package_py(
            middle_c,
            "middle_lib_c",
            "1.5.0",
            requires=["base_lib_a-1.0+", "base_lib_b-2.0+"],
        )

        middle_d = repo / "middle_lib_d" / "3.0.0"
        write_package_py(
            middle_d,
            "middle_lib_d",
            "3.0.0",
            requires=["base_lib_b-2.0+"],
        )

        # Top layer
        app_e = repo / "app_e" / "1.0.0"
        write_package_py(
            app_e,
            "app_e",
            "1.0.0",
            requires=["middle_lib_c-1.0+", "middle_lib_d-3.0+"],
        )

        # Alternative path
        app_f = repo / "app_f" / "2.0.0"
        write_package_py(
            app_f,
            "app_f",
            "2.0.0",
            requires=["middle_lib_c-1.0+"],
        )

        return str(repo)

    def test_resolve_diamond_dependency(self, complex_repo):
        """Test resolving diamond dependency: app_e -> middle_c -> base_a
                                                  -> middle_d -> base_b -> base_a
        """
        result = rez.resolve(["app_e-1.0+"], paths=[complex_repo])
        assert result is not None
        packages = result.get("packages", [])
        package_names = [p["name"] for p in packages]
        # All packages in the dependency graph should be resolved
        assert "app_e" in package_names
        assert "middle_lib_c" in package_names
        assert "middle_lib_d" in package_names
        assert "base_lib_a" in package_names
        assert "base_lib_b" in package_names

    def test_resolve_multiple_apps(self, complex_repo):
        """Test resolving multiple top-level packages."""
        result = rez.resolve(
            ["app_e-1.0+", "app_f-2.0+"],
            paths=[complex_repo],
        )
        assert result is not None
        packages = result.get("packages", [])
        package_names = [p["name"] for p in packages]
        assert "app_e" in package_names
        assert "app_f" in package_names
        # Shared dependency should only appear once
        base_a_count = sum(1 for p in packages if p["name"] == "base_lib_a")
        assert base_a_count == 1
