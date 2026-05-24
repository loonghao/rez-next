"""Solver module for rez_next.

This module provides the same API as rez.solver for drop-in compatibility.
"""

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





