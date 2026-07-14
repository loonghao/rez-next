"""Solver module for rez_next.

This module provides the same API as rez.solver for drop-in compatibility.
"""

from __future__ import annotations

import enum

import rez_next._native  # ensure extension module is initialized  # noqa: F401
from rez_next._native.solver_ import (  # noqa: F401,F403
    Solver,
    SolverStatus,
    SolverState,
    FailureReason,
    DependencyConflict,
    DependencyConflicts,
    Reduction,
    TotalReduction,
    RequirementList,
    PackageVariant,
    PackageVariantCache,
    ConflictSeverity,
    ConflictResolution,
    accessibility,
    find_cycle,
    package_repo_stats,
)


#: Internal solver version — bump when solver behaviour changes.
SOLVER_VERSION = 2


class VariantSelectMode(enum.Enum):
    """Variant selection mode for the solver.

    Rez API: ``rez.solver.VariantSelectMode``
    """
    version_priority = 0
    intersection_priority = 1


class SolverCallbackReturn(enum.Enum):
    """Enum returned by the ``callback`` callable passed to a ``Solver`` instance.

    Rez API: ``rez.solver.SolverCallbackReturn``
    """
    keep_going = ("Continue the solve",)
    abort = ("Abort the solve",)
    fail = ("Stop the solve and set to most recent failure",)

    def __init__(self, description: str) -> None:
        self.description: str = description
