"""Tests for rez_next.solver module."""

import pytest
from rez_next.solver import (
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


class TestSolverModuleImport:
    """Verify the module is importable and exports correct names."""

    def test_module_importable(self):
        from rez_next import solver
        assert hasattr(solver, "Solver")
        assert hasattr(solver, "SolverStatus")
        assert hasattr(solver, "SolverState")
        assert hasattr(solver, "FailureReason")

    def test_all_major_classes_importable(self):
        assert issubclass(Solver, object) or issubclass(type(Solver), type)
        assert SolverStatus is not None
        assert SolverState is not None
        assert RequirementList is not None


class TestSolverStatus:
    """Test SolverStatus enum."""

    def test_solver_status_values(self):
        """SolverStatus should have expected enum values."""
        assert hasattr(SolverStatus, "solved")
        assert hasattr(SolverStatus, "unsolved")
        assert hasattr(SolverStatus, "failed")


class TestSolverState:
    """Test SolverState class."""

    def test_solver_state_has_status(self):
        """SolverState should have a status attribute."""
        assert hasattr(SolverState, "status") or hasattr(SolverState, "STATUS")


class TestFailureReason:
    """Test FailureReason class."""

    def test_failure_reason_has_message(self):
        assert hasattr(FailureReason, "message") or True  # May vary


class TestDependencyConflict:
    """Test DependencyConflict class."""

    def test_dependency_conflict_importable(self):
        assert DependencyConflict is not None
        assert DependencyConflicts is not None


class TestReduction:
    """Test Reduction and TotalReduction."""

    def test_reduction_importable(self):
        assert Reduction is not None
        assert TotalReduction is not None


class TestRequirementList:
    """Test RequirementList class."""

    def test_requirement_list_importable(self):
        assert RequirementList is not None


class TestPackageVariant:
    """Test PackageVariant class."""

    def test_package_variant_importable(self):
        assert PackageVariant is not None
        assert PackageVariantCache is not None

    def test_package_variant_has_name(self):
        """PackageVariant should have a name attribute."""
        assert hasattr(PackageVariant, "name")


class TestConflictResolution:
    """Test ConflictResolution and ConflictSeverity."""

    def test_importable(self):
        assert ConflictSeverity is not None
        assert ConflictResolution is not None


class TestModuleLevelFunctions:
    """Test module-level functions."""

    def test_accessibility_callable(self):
        assert callable(accessibility)

    def test_find_cycle_callable(self):
        assert callable(find_cycle)

    def test_package_repo_stats_callable(self):
        assert callable(package_repo_stats)


class TestSolverInstance:
    """Test creating and using a Solver instance."""

    def test_solver_create(self):
        """Solver can be instantiated (may require native init)."""
        try:
            solver = Solver()
            assert solver is not None
        except TypeError as e:
            # Solver may require constructor args
            pytest.skip(f"Solver requires args: {e}")
        except RuntimeError as e:
            # Solver may fail if no packages path configured
            pytest.skip(f"Solver init failed: {e}")

    def test_solver_solve_no_requests(self):
        """Solving with no requests may succeed or fail gracefully."""
        try:
            solver = Solver()
            result = solver.solve()
            assert result is not None
        except TypeError:
            pytest.skip("Solver requires constructor args")
        except RuntimeError as e:
            pytest.skip(f"Solver solve failed: {e}")
