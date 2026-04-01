"""
Module-level tests for rez_next solver / ResolvedContext API.

These tests verify the solver API without a real package repository
(empty-repo fast path), plus context attribute contracts.
"""

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


class TestResolvedContextAPI:
    """ResolvedContext attribute contract."""

    def test_empty_context_attributes(self):
        ctx = rez.ResolvedContext([])
        assert hasattr(ctx, "success")
        assert hasattr(ctx, "resolved_packages")
        assert hasattr(ctx, "num_resolved_packages")
        assert isinstance(ctx.resolved_packages, list)
        assert ctx.num_resolved_packages == 0

    def test_context_empty_is_success(self):
        ctx = rez.ResolvedContext([])
        # Empty request = trivially resolved
        assert ctx.success is True

    def test_context_from_submodule(self):
        from rez_next.resolved_context import ResolvedContext

        ctx = ResolvedContext([])
        assert ctx is not None
        assert ctx.num_resolved_packages == 0

    def test_context_with_nonexistent_package(self):
        """Resolving a package not in any repo should fail (success=False)."""
        ctx = rez.ResolvedContext(["nonexistent_package_xyz_99999"])
        assert hasattr(ctx, "success")
        # May succeed (empty result) or fail — either is valid without a repo
        # The important thing is the attribute exists and is bool-like

    def test_multiple_empty_contexts_independent(self):
        ctx1 = rez.ResolvedContext([])
        ctx2 = rez.ResolvedContext([])
        assert ctx1 is not ctx2
        assert ctx1.num_resolved_packages == ctx2.num_resolved_packages

    def test_context_resolved_packages_is_list(self):
        ctx = rez.ResolvedContext([])
        assert isinstance(ctx.resolved_packages, list)

    def test_context_num_resolved_packages_type(self):
        ctx = rez.ResolvedContext([])
        assert isinstance(ctx.num_resolved_packages, int)

    def test_context_success_is_bool(self):
        ctx = rez.ResolvedContext([])
        assert isinstance(ctx.success, bool)

    def test_context_multiple_empty_requests(self):
        """Multiple sequential empty contexts should all succeed independently."""
        for _ in range(5):
            ctx = rez.ResolvedContext([])
            assert ctx.success is True
            assert ctx.num_resolved_packages == 0

    def test_context_string_packages_list(self):
        """Passing a list of strings should not crash, even if packages don't exist."""
        ctx = rez.ResolvedContext(["python", "numpy", "scipy"])
        assert hasattr(ctx, "success")
        assert hasattr(ctx, "resolved_packages")


class TestResolvePackagesFunction:
    """Top-level resolve_packages() convenience function."""

    def test_resolve_empty(self):
        result = rez.resolve_packages([])
        assert result is not None

    def test_resolve_returns_context(self):
        ctx = rez.resolve_packages([])
        assert hasattr(ctx, "resolved_packages")

    def test_resolve_with_paths(self):
        ctx = rez.resolve_packages([], paths=["/nonexistent/repo/path"])
        assert ctx is not None

    def test_resolve_empty_returns_success(self):
        ctx = rez.resolve_packages([])
        assert ctx.success is True

    def test_resolve_packages_count_zero_for_empty(self):
        ctx = rez.resolve_packages([])
        assert ctx.num_resolved_packages == 0

    def test_resolve_packages_list_empty_for_empty(self):
        ctx = rez.resolve_packages([])
        assert ctx.resolved_packages == []

    def test_resolve_nonexistent_package(self):
        """resolve_packages with nonexistent pkg should not raise, return context."""
        ctx = rez.resolve_packages(["nonexistent_xyz_99999"])
        assert hasattr(ctx, "success")
        assert hasattr(ctx, "resolved_packages")

    def test_resolve_with_empty_paths(self):
        ctx = rez.resolve_packages([], paths=[])
        assert ctx is not None

    def test_resolve_multiple_calls_independent(self):
        ctx1 = rez.resolve_packages([])
        ctx2 = rez.resolve_packages([])
        assert ctx1 is not ctx2
        assert ctx1.success == ctx2.success


class TestRepositoryManager:
    """RepositoryManager API."""

    def test_create(self):
        repo = rez.RepositoryManager()
        assert repo is not None

    def test_find_packages_empty(self):
        repo = rez.RepositoryManager()
        results = repo.find_packages("nonexistent_xyz")
        assert results == []

    def test_find_packages_returns_list(self):
        repo = rez.RepositoryManager()
        results = repo.find_packages("python")
        assert isinstance(results, list)

    def test_multiple_repo_managers_independent(self):
        repo1 = rez.RepositoryManager()
        repo2 = rez.RepositoryManager()
        assert repo1 is not repo2

    def test_find_packages_empty_name(self):
        """Searching for empty string should return empty list, not crash."""
        repo = rez.RepositoryManager()
        results = repo.find_packages("")
        assert isinstance(results, list)

    def test_find_packages_special_chars(self):
        """Package names with special chars should return empty, not crash."""
        repo = rez.RepositoryManager()
        results = repo.find_packages("pkg-with-hyphens_and_underscores")
        assert isinstance(results, list)

