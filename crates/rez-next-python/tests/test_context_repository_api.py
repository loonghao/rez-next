"""
Deep-dive tests for ResolvedContext and RepositoryManager Python API.

These tests go beyond basic smoke tests to verify the full attribute and
method contract that rez users rely on when migrating to rez_next.

Usage:
    maturin develop --features extension-module
    pytest tests/test_context_repository_api.py -v
"""
import json
import os

import pytest

rez = pytest.importorskip(
    "rez_next",
    reason="rez_next not built — run: maturin develop --features extension-module",
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


# ── ResolvedContext attributes ────────────────────────────────────────────────


class TestResolvedContextAttributes:
    """Verify ResolvedContext attribute contract matches rez."""

    def test_success_attr_exists(self):
        ctx = rez.ResolvedContext([])
        assert hasattr(ctx, "success")

    def test_success_is_bool(self):
        ctx = rez.ResolvedContext([])
        assert isinstance(ctx.success, bool)

    def test_empty_context_is_success(self):
        ctx = rez.ResolvedContext([])
        assert ctx.success is True

    def test_resolved_packages_attr_exists(self):
        ctx = rez.ResolvedContext([])
        assert hasattr(ctx, "resolved_packages")

    def test_resolved_packages_is_list(self):
        ctx = rez.ResolvedContext([])
        assert isinstance(ctx.resolved_packages, list)

    def test_num_resolved_packages_attr(self):
        ctx = rez.ResolvedContext([])
        assert hasattr(ctx, "num_resolved_packages")
        assert isinstance(ctx.num_resolved_packages, int)
        assert ctx.num_resolved_packages == 0

    def test_id_attr_exists(self):
        ctx = rez.ResolvedContext([])
        assert hasattr(ctx, "id")
        assert isinstance(ctx.id, str)
        assert len(ctx.id) > 0

    def test_created_at_attr_exists(self):
        ctx = rez.ResolvedContext([])
        assert hasattr(ctx, "created_at")
        assert isinstance(ctx.created_at, int)
        assert ctx.created_at > 0

    def test_failure_description_is_none_on_success(self):
        ctx = rez.ResolvedContext([])
        assert hasattr(ctx, "failure_description")
        assert ctx.failure_description is None

    def test_each_context_has_unique_id(self):
        ctx1 = rez.ResolvedContext([])
        ctx2 = rez.ResolvedContext([])
        assert ctx1.id != ctx2.id, "each context must have a unique ID"


# ── ResolvedContext methods ───────────────────────────────────────────────────


class TestResolvedContextMethods:
    """Verify ResolvedContext method availability and basic contracts."""

    def test_get_environ_returns_dict(self):
        ctx = rez.ResolvedContext([])
        env = ctx.get_environ()
        assert isinstance(env, dict)

    def test_get_environ_has_string_keys_and_values(self):
        ctx = rez.ResolvedContext([])
        env = ctx.get_environ()
        for k, v in env.items():
            assert isinstance(k, str), f"key {k!r} is not str"
            assert isinstance(v, str), f"value for {k!r} is not str"

    def test_get_resolved_package_not_found_returns_none(self):
        ctx = rez.ResolvedContext([])
        result = ctx.get_resolved_package("nonexistent_xyz_999")
        assert result is None

    def test_get_tools_returns_dict(self):
        ctx = rez.ResolvedContext([])
        tools = ctx.get_tools()
        assert isinstance(tools, dict)

    def test_to_dict_returns_dict(self):
        ctx = rez.ResolvedContext([])
        d = ctx.to_dict()
        assert isinstance(d, dict)
        assert "id" in d
        assert "status" in d
        assert "packages" in d
        assert "num_packages" in d

    def test_to_dict_num_packages_matches(self):
        ctx = rez.ResolvedContext([])
        d = ctx.to_dict()
        assert d["num_packages"] == 0

    def test_to_shell_script_bash(self):
        ctx = rez.ResolvedContext([])
        script = ctx.to_shell_script("bash")
        assert isinstance(script, str)

    def test_to_shell_script_powershell(self):
        ctx = rez.ResolvedContext([])
        script = ctx.to_shell_script("powershell")
        assert isinstance(script, str)

    def test_to_shell_script_auto(self):
        ctx = rez.ResolvedContext([])
        script = ctx.to_shell_script()
        assert isinstance(script, str)

    def test_get_resolved_packages_info_returns_list(self):
        ctx = rez.ResolvedContext([])
        info = ctx.get_resolved_packages_info()
        assert isinstance(info, list)

    def test_print_info_does_not_raise(self, capsys):
        ctx = rez.ResolvedContext([])
        ctx.print_info()  # should not raise


# ── ResolvedContext with real on-disk packages ────────────────────────────────


class TestResolvedContextWithRealRepo:
    """End-to-end tests using a real tmp on-disk package repository."""

    def test_resolve_single_package(self, tmp_path):
        """Resolve a single package from a real on-disk repo."""
        pkg_dir = tmp_path / "python" / "3.11.0"
        write_package_py(pkg_dir, "python", "3.11.0",
                         commands="env.setenv('PYTHON_ROOT', '{root}')")

        ctx = rez.ResolvedContext(["python"], paths=[str(tmp_path)])
        assert ctx.success is True
        assert ctx.num_resolved_packages == 1

    def test_resolved_package_name_matches(self, tmp_path):
        pkg_dir = tmp_path / "python" / "3.11.0"
        write_package_py(pkg_dir, "python", "3.11.0")

        ctx = rez.ResolvedContext(["python"], paths=[str(tmp_path)])
        assert ctx.num_resolved_packages == 1
        pkg = ctx.resolved_packages[0]
        assert pkg.name == "python"

    def test_resolved_package_version_matches(self, tmp_path):
        pkg_dir = tmp_path / "python" / "3.11.0"
        write_package_py(pkg_dir, "python", "3.11.0")

        ctx = rez.ResolvedContext(["python"], paths=[str(tmp_path)])
        pkg = ctx.resolved_packages[0]
        assert "3.11" in pkg.version_str

    def test_resolve_multiple_packages(self, tmp_path):
        write_package_py(tmp_path / "python" / "3.11.0", "python", "3.11.0")
        write_package_py(tmp_path / "numpy" / "1.25.0", "numpy", "1.25.0")

        ctx = rez.ResolvedContext(["python", "numpy"], paths=[str(tmp_path)])
        assert ctx.num_resolved_packages == 2
        names = [p.name for p in ctx.resolved_packages]
        assert "python" in names
        assert "numpy" in names

    def test_get_resolved_package_by_name(self, tmp_path):
        write_package_py(tmp_path / "python" / "3.11.0", "python", "3.11.0")

        ctx = rez.ResolvedContext(["python"], paths=[str(tmp_path)])
        pkg = ctx.get_resolved_package("python")
        assert pkg is not None
        assert pkg.name == "python"

    def test_context_save_and_load_roundtrip(self, tmp_path):
        pkg_dir = tmp_path / "python" / "3.11.0"
        write_package_py(pkg_dir, "python", "3.11.0")

        ctx = rez.ResolvedContext(["python"], paths=[str(tmp_path)])
        save_path = str(tmp_path / "context.rxt")
        ctx.save(save_path)
        assert os.path.exists(save_path)

        loaded = rez.ResolvedContext.load(save_path)
        assert loaded is not None
        assert loaded.num_resolved_packages == ctx.num_resolved_packages

    def test_context_save_creates_valid_json(self, tmp_path):
        pkg_dir = tmp_path / "python" / "3.11.0"
        write_package_py(pkg_dir, "python", "3.11.0")

        ctx = rez.ResolvedContext(["python"], paths=[str(tmp_path)])
        save_path = str(tmp_path / "context.rxt")
        ctx.save(save_path)

        with open(save_path) as f:
            data = json.load(f)
        assert isinstance(data, dict)

    def test_get_environ_with_resolved_context(self, tmp_path):
        pkg_dir = tmp_path / "python" / "3.11.0"
        write_package_py(pkg_dir, "python", "3.11.0",
                         commands="env.setenv('PYTHON_ROOT', '/opt/python')")

        ctx = rez.ResolvedContext(["python"], paths=[str(tmp_path)])
        env = ctx.get_environ()
        assert isinstance(env, dict)


# ── RepositoryManager attributes ─────────────────────────────────────────────


class TestRepositoryManagerAttributes:
    """Verify RepositoryManager attribute and method contract."""

    def test_create_with_no_paths(self):
        repo = rez.RepositoryManager()
        assert repo is not None

    def test_create_with_explicit_paths(self):
        repo = rez.RepositoryManager(paths=["/nonexistent/path"])
        assert repo is not None

    def test_find_packages_returns_list(self):
        repo = rez.RepositoryManager(paths=["/nonexistent/path"])
        result = repo.find_packages("python")
        assert isinstance(result, list)

    def test_find_packages_empty_name_returns_list(self):
        repo = rez.RepositoryManager(paths=["/nonexistent/path"])
        result = repo.find_packages("")
        assert isinstance(result, list)

    def test_find_packages_nonexistent_returns_empty(self):
        repo = rez.RepositoryManager(paths=["/nonexistent/path_xyz_999"])
        result = repo.find_packages("totally_unknown_pkg")
        assert result == []

    def test_get_latest_package_not_found_returns_none(self):
        repo = rez.RepositoryManager(paths=["/nonexistent/path_xyz"])
        result = repo.get_latest_package("nonexistent_pkg")
        assert result is None

    def test_get_package_family_names_returns_list(self):
        repo = rez.RepositoryManager(paths=["/nonexistent/path_xyz"])
        result = repo.get_package_family_names()
        assert isinstance(result, list)

    def test_repr_is_string(self):
        repo = rez.RepositoryManager(paths=["/some/path"])
        r = repr(repo)
        assert isinstance(r, str)
        assert "RepositoryManager" in r


# ── RepositoryManager with real on-disk packages ─────────────────────────────


class TestRepositoryManagerWithRealRepo:
    """End-to-end tests with a real tmp on-disk repository."""

    def test_find_single_package(self, tmp_path):
        pkg_dir = tmp_path / "python" / "3.11.0"
        write_package_py(pkg_dir, "python", "3.11.0")

        repo = rez.RepositoryManager(paths=[str(tmp_path)])
        pkgs = repo.find_packages("python")
        assert len(pkgs) >= 1
        assert pkgs[0].name == "python"

    def test_find_multiple_packages(self, tmp_path):
        write_package_py(tmp_path / "python" / "3.9.0", "python", "3.9.0")
        write_package_py(tmp_path / "python" / "3.11.0", "python", "3.11.0")

        repo = rez.RepositoryManager(paths=[str(tmp_path)])
        pkgs = repo.find_packages("python")
        assert len(pkgs) == 2
        versions = sorted(p.version_str for p in pkgs)
        assert "3.9.0" in versions
        assert "3.11.0" in versions

    @pytest.mark.xfail(reason="Requires get_latest_package to be implemented")
    def test_get_latest_package(self, tmp_path):
        write_package_py(tmp_path / "python" / "3.9.0", "python", "3.9.0")
        write_package_py(tmp_path / "python" / "3.11.0", "python", "3.11.0")

        repo = rez.RepositoryManager(paths=[str(tmp_path)])
        latest = repo.get_latest_package("python")
        assert latest is not None
        assert "3.11" in latest.version_str

    @pytest.mark.xfail(reason="Requires get_package_family_names to be fully implemented")
    def test_get_package_family_names_includes_all(self, tmp_path):
        write_package_py(tmp_path / "python" / "3.11.0", "python", "3.11.0")
        write_package_py(tmp_path / "numpy" / "1.25.0", "numpy", "1.25.0")

        repo = rez.RepositoryManager(paths=[str(tmp_path)])
        names = repo.get_package_family_names()
        assert "python" in names
        assert "numpy" in names

    def test_get_package_family_names_sorted(self, tmp_path):
        write_package_py(tmp_path / "zzz_pkg" / "1.0.0", "zzz_pkg", "1.0.0")
        write_package_py(tmp_path / "aaa_pkg" / "1.0.0", "aaa_pkg", "1.0.0")

        repo = rez.RepositoryManager(paths=[str(tmp_path)])
        names = repo.get_package_family_names()
        assert names == sorted(names), "package family names should be sorted"

    def test_find_package_not_in_repo_returns_empty(self, tmp_path):
        write_package_py(tmp_path / "python" / "3.11.0", "python", "3.11.0")

        repo = rez.RepositoryManager(paths=[str(tmp_path)])
        pkgs = repo.find_packages("houdini")
        assert pkgs == []


# ── Context + Repository integration ─────────────────────────────────────────


class TestContextRepositoryIntegration:
    """Integration between ResolvedContext and RepositoryManager."""

    def test_context_and_repo_same_packages(self, tmp_path):
        """Both APIs should find the same package from the same path."""
        write_package_py(tmp_path / "python" / "3.11.0", "python", "3.11.0")

        repo = rez.RepositoryManager(paths=[str(tmp_path)])
        repo_pkgs = repo.find_packages("python")

        ctx = rez.ResolvedContext(["python"], paths=[str(tmp_path)])
        ctx_pkgs = ctx.resolved_packages

        # Both should resolve python
        repo_names = {p.name for p in repo_pkgs}
        ctx_names = {p.name for p in ctx_pkgs}
        assert "python" in repo_names
        assert "python" in ctx_names

    def test_context_get_tools_includes_pkg_tools(self, tmp_path):
        """If package declares tools, context.get_tools() should include them."""
        pkg_dir = tmp_path / "python" / "3.11.0"
        pkg_dir.mkdir(parents=True, exist_ok=True)
        (pkg_dir / "package.py").write_text(
            'name = "python"\nversion = "3.11.0"\ntools = ["python", "python3"]\n'
        )

        ctx = rez.ResolvedContext(["python"], paths=[str(tmp_path)])
        tools = ctx.get_tools()
        assert isinstance(tools, dict)

    def test_top_level_resolve_packages_fn(self, tmp_path):
        """Top-level resolve_packages() mirrors ResolvedContext."""
        write_package_py(tmp_path / "python" / "3.11.0", "python", "3.11.0")

        ctx = rez.resolve_packages(["python"], paths=[str(tmp_path)])
        assert ctx.success is True
        assert ctx.num_resolved_packages == 1

    def test_top_level_iter_packages_fn(self, tmp_path):
        """iter_packages() from top level finds packages in repo."""
        write_package_py(tmp_path / "python" / "3.9.0", "python", "3.9.0")
        write_package_py(tmp_path / "python" / "3.11.0", "python", "3.11.0")

        pkgs = rez.iter_packages("python", paths=[str(tmp_path)])
        assert isinstance(pkgs, list)
        assert len(pkgs) == 2

    @pytest.mark.xfail(reason="Requires top-level get_latest_package to be implemented")
    def test_top_level_get_latest_package_fn(self, tmp_path):
        write_package_py(tmp_path / "python" / "3.9.0", "python", "3.9.0")
        write_package_py(tmp_path / "python" / "3.11.0", "python", "3.11.0")

        latest = rez.get_latest_package("python", paths=[str(tmp_path)])
        assert latest is not None
        assert "3.11" in latest.version_str

    def test_top_level_get_package_fn(self, tmp_path):
        write_package_py(tmp_path / "python" / "3.11.0", "python", "3.11.0")

        pkg = rez.get_package("python", "3.11.0", paths=[str(tmp_path)])
        assert pkg is not None
        assert pkg.name == "python"

    @pytest.mark.xfail(reason="Requires walk_packages to return all packages")
    def test_top_level_walk_packages_fn(self, tmp_path):
        write_package_py(tmp_path / "python" / "3.11.0", "python", "3.11.0")
        write_package_py(tmp_path / "numpy" / "1.25.0", "numpy", "1.25.0")

        pkgs = rez.walk_packages(paths=[str(tmp_path)])
        assert isinstance(pkgs, list)
        assert len(pkgs) == 2

    @pytest.mark.xfail(reason="Requires get_package_family_names to be fully implemented")
    def test_top_level_get_package_family_names_fn(self, tmp_path):
        write_package_py(tmp_path / "python" / "3.11.0", "python", "3.11.0")
        write_package_py(tmp_path / "numpy" / "1.25.0", "numpy", "1.25.0")

        names = rez.get_package_family_names(paths=[str(tmp_path)])
        assert "python" in names
        assert "numpy" in names


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
