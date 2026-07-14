"""Tests for rez_next.package_repository module."""

import os
import pytest
from rez_next.package_repository import (
    PackageRepository,
    PackageRepositoryManager,
    PackageRepositoryGlobalStats,
    get_package_repository_types,
    package_repo_stats,
    package_repository_manager,
)


class TestPackageRepositoryGlobalStats:
    def test_thread_local_defaults(self):
        stats = PackageRepositoryGlobalStats()
        assert stats.package_load_time == 0.0

    def test_package_loading_context(self):
        stats = PackageRepositoryGlobalStats()
        with stats.package_loading():
            pass
        assert stats.package_load_time >= 0.0

    def test_singleton_exists(self):
        assert package_repo_stats is not None
        assert hasattr(package_repo_stats, "package_loading")


class TestPackageRepository:
    def test_name_raises(self):
        with pytest.raises(NotImplementedError):
            PackageRepository.name()

    def test_abstract_methods(self):
        class _ConcreteRepo(PackageRepository):
            @classmethod
            def name(cls):
                return "test_repo"

        test_path = "/tmp/test_repo"
        repo = _ConcreteRepo(test_path)
        # __init__ calls os.path.abspath, so expected path varies per platform
        expected_loc = os.path.abspath(test_path)
        assert str(repo) == f"test_repo@{expected_loc}"
        assert repo.uid == ("test_repo", expected_loc)

    def test_is_empty_raises(self):
        repo = PackageRepository("/tmp/test_repo")
        with pytest.raises(NotImplementedError):
            repo.is_empty()

    def test_remove_sentinel(self):
        assert PackageRepository.remove is not None
        assert isinstance(PackageRepository.remove, object)


class TestPackageRepositoryManager:
    def test_singleton_exists(self):
        assert package_repository_manager is not None
        assert hasattr(package_repository_manager, "get_repository")
        assert hasattr(package_repository_manager, "clear_caches")

    def test_init(self):
        mgr = PackageRepositoryManager()
        assert mgr.repositories == {}
        assert hasattr(mgr, "pool")

    def test_clear_caches(self):
        mgr = PackageRepositoryManager()
        mgr.clear_caches()
        assert mgr.repositories == {}


class TestFactoryFunctions:
    def test_get_package_repository_types(self):
        types = get_package_repository_types()
        assert isinstance(types, list)
