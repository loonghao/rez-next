"""Tests for rez_next.resolver module."""

import pytest
from rez_next.resolver import (
    Resolver,
    ResolverStatus,
    SolverDict,
)


class TestResolverStatus:
    def test_pending(self):
        assert ResolverStatus.pending.description == "The resolve has not yet started."

    def test_solved(self):
        assert ResolverStatus.solved.description == "The resolve has completed successfully."

    def test_failed(self):
        assert ResolverStatus.failed.description == "The resolve is not possible."

    def test_aborted(self):
        assert ResolverStatus.aborted.description == "The resolve was stopped by the user (via callback)."


class TestSolverDict:
    def test_empty_dict(self):
        sd: SolverDict = {}
        assert isinstance(sd, dict)

    def test_full_dict(self):
        sd: SolverDict = {
            "status": ResolverStatus.solved,
            "variant_handles": [],
            "ephemerals": [],
        }
        assert sd["status"] == ResolverStatus.solved


class TestResolver:
    def test_create_minimal(self):
        resolver = Resolver(
            context=None,
            package_requests=[],
            package_paths=["/tmp"],
        )
        assert resolver.status == ResolverStatus.pending
        assert resolver.resolved_packages is None
        assert resolver.resolved_ephemerals is None

    def test_create_with_requests(self):
        from rez_next import PackageRequirement
        req = PackageRequirement("python-3.9")
        resolver = Resolver(
            context=None,
            package_requests=[req],
            package_paths=["/tmp"],
        )
        assert len(resolver.package_requests) == 1

    def test_status_property(self):
        resolver = Resolver(
            context=None,
            package_requests=[],
            package_paths=["/tmp"],
        )
        assert resolver.status == ResolverStatus.pending
        assert resolver.status.description == "The resolve has not yet started."

    def test_not_solved_by_default(self):
        resolver = Resolver(
            context=None,
            package_requests=[],
            package_paths=["/tmp"],
        )
        assert resolver.resolved_packages is None
        assert resolver.resolved_ephemerals is None
        assert resolver.graph is None

    def test_from_cache_default(self):
        resolver = Resolver(
            context=None,
            package_requests=[],
            package_paths=["/tmp"],
        )
        assert resolver.from_cache is False

    def test_with_timestamp_and_filter(self):
        resolver = Resolver(
            context=None,
            package_requests=[],
            package_paths=["/tmp"],
            timestamp=1234567890,
        )
        assert resolver.timestamp == 1234567890
        # package_filter defaults to None when timestamp is set (no native TimestampRule available)
        assert resolver.package_filter is None

    def test_solver_to_dict_mapping(self):
        sd = Resolver._solver_to_dict(None)
        assert "status" in sd
