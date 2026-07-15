"""
resolver — aligns with rez.resolver.

Provides the ``Resolver`` class that orchestrates dependency resolution
by wrapping the native ``Solver`` with package filtering and custom ordering.

Key design:
- Pure orchestration layer (no solver logic in this class)
- Uncached execution; explicit cache requests fail until safe invalidation exists
- No legacy ``rez_1_environment_variables`` compatibility
"""

from __future__ import annotations

import enum
from hashlib import sha1
from typing import TYPE_CHECKING, Any, Callable, TypedDict

if TYPE_CHECKING:
    from rez_next.package_filter import PackageFilterList
    from rez_next.package_order import PackageOrderList
    from rez_next.packages import Variant
    from rez_next.resolved_context import ResolvedContext
    from rez_next.version import Requirement


class SolverDict(TypedDict, total=False):
    """Serialisable solve result for caching."""

    status: ResolverStatus
    graph: Any
    solve_time: float | None
    load_time: float | None
    failure_description: str | None
    variant_handles: list[dict[str, Any]]
    ephemerals: list[str]


class ResolverStatus(enum.Enum):
    """Status of a resolver instance."""

    pending = ("The resolve has not yet started.",)
    solved = ("The resolve has completed successfully.",)
    failed = ("The resolve is not possible.",)
    aborted = ("The resolve was stopped by the user (via callback).",)

    def __init__(self, description: str) -> None:
        self.description: str = description


class Resolver:
    """Package resolver combining Solver, package filters, and ordering.

    Rez API: ``rez.resolver.Resolver``
    """

    def __init__(
        self,
        context: ResolvedContext,
        package_requests: list[Requirement],
        package_paths: list[str],
        package_filter: PackageFilterList | None = None,
        package_orderers: PackageOrderList | None = None,
        timestamp: int | None = 0,
        callback: Callable | None = None,  # SolverState -> tuple
        building: bool = False,
        testing: bool = False,
        verbosity: int = 0,
        buf: Any = None,
        package_load_callback: Callable[[Any], None] | None = None,
        caching: bool = False,
        suppress_passive: bool = False,
        print_stats: bool = False,
    ) -> None:
        if caching:
            raise NotImplementedError(
                "Resolver caching is not implemented with safe repository invalidation"
            )

        self.context = context
        self.package_requests = list(package_requests)
        self.package_paths = list(package_paths)
        self.timestamp = timestamp
        self.callback = callback
        self.package_orderers = package_orderers
        self.package_load_callback = package_load_callback
        self.building = building
        self.testing = testing
        self.verbosity = verbosity
        self.caching = False
        self.buf = buf
        self.suppress_passive = suppress_passive
        self.print_stats = print_stats

        self.package_filter = package_filter
        self.package_orderers_hash = self._hash_orderers(package_orderers)
        self.package_filter_hash = (
            package_filter.sha1 if package_filter and hasattr(package_filter, "sha1") else ""
        )

        self.status_: ResolverStatus = ResolverStatus.pending
        self.resolved_packages_: list[Variant] | None = None
        self.resolved_ephemerals_: list[Requirement] | None = None
        self.failure_description: str | None = None
        self.graph_: Any = None
        self.from_cache: bool = False
        self.solve_time: float | None = 0.0
        self.load_time: float | None = 0.0

    @property
    def status(self) -> ResolverStatus:
        return self.status_

    @property
    def resolved_packages(self) -> list[Variant] | None:
        return self.resolved_packages_

    @property
    def resolved_ephemerals(self) -> list[Requirement] | None:
        return self.resolved_ephemerals_

    @property
    def graph(self) -> Any:
        return self.graph_

    def solve(self) -> None:
        """Execute dependency resolution without a stale-result cache."""
        self.from_cache = False
        solver = self._solve()
        solver_dict = self._solver_to_dict(solver)
        self._set_result(solver_dict)

    def _solve(self) -> Any:  # Solver
        from rez_next.config import config as cfg
        from rez_next.solver import Solver as NativeSolver

        solver = NativeSolver(
            package_requests=self.package_requests,
            package_paths=self.package_paths,
            context=self.context,
            package_filter=self.package_filter,
            package_orderers=self.package_orderers,
            callback=self.callback,
            package_load_callback=self.package_load_callback,
            building=self.building,
            verbosity=self.verbosity,
            prune_unfailed=getattr(cfg, "prune_failed_graph", False),
            buf=self.buf,
            suppress_passive=self.suppress_passive,
            print_stats=self.print_stats,
        )
        solver.solve()
        return solver

    def _set_result(self, solver_dict: SolverDict) -> None:
        self.status_ = solver_dict.get("status", ResolverStatus.pending)
        self.graph_ = solver_dict.get("graph")
        self.solve_time = solver_dict.get("solve_time")
        self.load_time = solver_dict.get("load_time")
        self.failure_description = solver_dict.get("failure_description")

        if self.status_ == ResolverStatus.solved:
            self.resolved_packages_ = []
            for vh in solver_dict.get("variant_handles") or []:
                variant = self._get_variant(vh)
                self.resolved_packages_.append(variant)

            self.resolved_ephemerals_ = []
            for req_str in solver_dict.get("ephemerals") or []:
                from rez_next.version import Requirement

                self.resolved_ephemerals_.append(Requirement(req_str))
        else:
            self.resolved_packages_ = None
            self.resolved_ephemerals_ = None

    def _get_variant(self, variant_handle: Any) -> Any:
        from rez_next.packages import get_variant

        return get_variant(variant_handle, context=self.context)

    @staticmethod
    def _hash_orderers(orderers: list | None) -> str:
        if not orderers:
            return ""
        sha1s = "".join(x.sha1 if hasattr(x, "sha1") else "" for x in orderers)
        return sha1(sha1s.encode("utf-8")).hexdigest() if sha1s else ""

    @classmethod
    def _solver_to_dict(cls, solver: Any) -> SolverDict:
        from rez_next.solver import SolverStatus

        graph_ = (
            getattr(solver, "get_graph", lambda: None)() if hasattr(solver, "get_graph") else None
        )
        solve_time = getattr(solver, "solve_time", None)
        load_time = getattr(solver, "load_time", None)
        failure_description: str | None = None
        variant_handles: list | None = None
        ephemerals: list | None = None

        st = getattr(solver, "status", None)
        if st == SolverStatus.unsolved:
            status_ = ResolverStatus.aborted
            failure_description = getattr(solver, "abort_reason", None)
        elif st == SolverStatus.failed:
            status_ = ResolverStatus.failed
            failure_description = (
                getattr(solver, "failure_description", lambda: None)()
                if hasattr(solver, "failure_description")
                else None
            )
        elif st == SolverStatus.solved:
            status_ = ResolverStatus.solved
            resolved = getattr(solver, "resolved_packages", [])
            variant_handles = [getattr(v, "handle", {}) for v in resolved]
            resolved_eps = getattr(solver, "resolved_ephemerals", [])
            ephemerals = [str(e) for e in resolved_eps]
        else:
            status_ = ResolverStatus.pending

        return SolverDict(
            status=status_,
            graph=graph_,
            solve_time=solve_time,
            load_time=load_time,
            failure_description=failure_description,
            variant_handles=variant_handles,
            ephemerals=ephemerals,
        )
