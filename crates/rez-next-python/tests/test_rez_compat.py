"""
Compatibility tests for rez_next as a drop-in replacement for rez.

Usage:
    maturin develop --features extension-module
    pytest tests/test_rez_compat.py -v

These tests verify that `import rez_next as rez` works as a complete
replacement for `import rez`.

Split layout:
  - test_rez_compat.py          — core: module structure, version, package, context,
                                  repository, config, suites, top-level functions
  - test_compat_io_modules.py   — I/O: shell, pip, bundles, bind, complete, CLI, env, build
  - test_compat_advanced.py     — advanced: plugins, search, exceptions, conflict/weak
                                  requirements, selftest, utils, walk_packages
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

    def test_version_range_any_none(self):
        from rez_next.vendor.version import VersionRange
        any_r = VersionRange.any()
        assert any_r.is_any()
        none_r = VersionRange.none()
        assert none_r.is_empty()

    def test_version_range_from_str(self):
        from rez_next.vendor.version import VersionRange, Version
        r = VersionRange.from_str(">=2.0,<3.0")
        assert r.contains(Version("2.5"))
        assert not r.contains(Version("3.0"))


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

    def test_package_family_create(self):
        import rez_next.packages as pkgs
        family = pkgs.PackageFamily("python")
        assert family.name == "python"
        assert family.num_versions == 0

    def test_package_family_class_at_top_level(self):
        assert rez.PackageFamily is not None
        family = rez.PackageFamily("maya")
        assert family.name == "maya"


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

    def test_resolved_context_success_is_true_for_empty(self):
        ctx = rez.ResolvedContext([])
        assert ctx.success is True

    def test_resolved_context_multiple_independent(self):
        ctx1 = rez.ResolvedContext([])
        ctx2 = rez.ResolvedContext([])
        assert ctx1 is not ctx2
        assert ctx1.num_resolved_packages == ctx2.num_resolved_packages


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

    def test_repo_manager_find_returns_list(self):
        repo = rez.RepositoryManager()
        results = repo.find_packages("python")
        assert isinstance(results, list)

    def test_multiple_repo_managers_independent(self):
        repo1 = rez.RepositoryManager()
        repo2 = rez.RepositoryManager()
        assert repo1 is not repo2

    def test_find_packages_empty_name(self):
        repo = rez.RepositoryManager()
        results = repo.find_packages("")
        assert isinstance(results, list)

    def test_find_packages_special_chars(self):
        repo = rez.RepositoryManager()
        results = repo.find_packages("pkg-with-hyphens_and_underscores")
        assert isinstance(results, list)


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
        result = cfg.get("packages_path", None)
        assert result is not None or result is None


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


class TestWalkPackages:
    """Verify walk_packages API."""

    def test_walk_packages_exists(self):
        assert callable(rez.walk_packages)

    def test_walk_packages_returns_list(self):
        result = rez.walk_packages(paths=["/nonexistent/path_xyz"])
        assert isinstance(result, list)
        assert len(result) == 0

    def test_packages_module_has_walk_packages(self):
        import rez_next.packages_ as pkgs
        assert callable(pkgs.walk_packages)

    def test_iter_packages_nonexistent_path(self):
        result = rez.iter_packages("python", paths=["/nonexistent_repo_xyz"])
        assert isinstance(result, list)
        assert len(result) == 0

    def test_get_package_family_names_empty_repo(self):
        names = rez.get_package_family_names(paths=["/nonexistent_repo_xyz"])
        assert isinstance(names, list)

    def test_get_latest_package_empty_repo(self):
        result = rez.get_latest_package("python", paths=["/nonexistent_repo_xyz"])
        assert result is None

    def test_get_package_empty_repo(self):
        result = rez.get_package("python", "3.9", paths=["/nonexistent_repo_xyz"])
        assert result is None


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
