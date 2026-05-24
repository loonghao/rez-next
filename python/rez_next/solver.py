"""Bridge to rez_next solver module (dependency resolution).

Aligns with rez.solver API:
- ``Solver`` — dependency graph solver (A* + backtracking)
- ``SolverStatus`` — solver status enum
- ``SolverState`` — solver state tracking
- ``FailureReason`` — resolution failure analysis
- ``DependencyConflict`` — individual dependency conflict
- ``DependencyConflicts`` — collection of conflicts
- ``Reduction`` — solver reduction representation
- ``TotalReduction`` — total reduction tracking
- ``RequirementList`` — collection of package requirements
- ``PackageVariant`` — resolved package variant entry
- ``PackageVariantCache`` — variant caching for solver
- ``find_cycle()`` — detect cycles in resolved package graph
- ``package_repo_stats()`` — repository statistics for solver
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "solver.py"
)
globals().update(runpy.run_path(str(_IMPL)))
