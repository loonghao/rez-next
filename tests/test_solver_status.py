"""
Tests for SolverStatus class (Cycle 283).

Validates that rez_next.solver_.SolverStatus matches
the original rez.solver.SolverStatus API contract.
"""

import pytest
import rez_next.solver_ as solver_module


class TestSolverStatusExistence:
    """SolverStatus class and members exist."""

    def test_solver_status_class_exists(self):
        assert hasattr(solver_module, "SolverStatus")

    def test_pending_exists(self):
        assert hasattr(solver_module.SolverStatus, "pending")

    def test_solved_exists(self):
        assert hasattr(solver_module.SolverStatus, "solved")

    def test_exhausted_exists(self):
        assert hasattr(solver_module.SolverStatus, "exhausted")

    def test_failed_exists(self):
        assert hasattr(solver_module.SolverStatus, "failed")

    def test_cyclic_exists(self):
        assert hasattr(solver_module.SolverStatus, "cyclic")

    def test_unsolved_exists(self):
        assert hasattr(solver_module.SolverStatus, "unsolved")


class TestSolverStatusAttributes:
    """Each member has name and description."""

    def test_pending_name(self):
        status = solver_module.SolverStatus.pending
        assert status.name == "pending"

    def test_pending_description(self):
        status = solver_module.SolverStatus.pending
        assert "not yet started" in status.description

    def test_solved_name(self):
        status = solver_module.SolverStatus.solved
        assert status.name == "solved"

    def test_solved_description(self):
        status = solver_module.SolverStatus.solved
        assert "completed successfully" in status.description

    def test_exhausted_name(self):
        status = solver_module.SolverStatus.exhausted
        assert status.name == "exhausted"

    def test_exhausted_description(self):
        status = solver_module.SolverStatus.exhausted
        assert "exhausted" in status.description

    def test_failed_name(self):
        status = solver_module.SolverStatus.failed
        assert status.name == "failed"

    def test_failed_description(self):
        status = solver_module.SolverStatus.failed
        assert "not possible" in status.description

    def test_cyclic_name(self):
        status = solver_module.SolverStatus.cyclic
        assert status.name == "cyclic"

    def test_cyclic_description(self):
        status = solver_module.SolverStatus.cyclic
        assert "cycle" in status.description

    def test_unsolved_name(self):
        status = solver_module.SolverStatus.unsolved
        assert status.name == "unsolved"

    def test_unsolved_description(self):
        status = solver_module.SolverStatus.unsolved
        assert "not yet solved" in status.description


class TestSolverStatusEquality:
    """SolverStatus members compare correctly."""

    def test_pending_not_equal_to_solved(self):
        assert solver_module.SolverStatus.pending != solver_module.SolverStatus.solved

    def test_pending_equal_to_pending(self):
        assert solver_module.SolverStatus.pending == solver_module.SolverStatus.pending

    def test_solved_not_equal_to_failed(self):
        assert solver_module.SolverStatus.solved != solver_module.SolverStatus.failed


class TestSolverStatusRepr:
    """__repr__ output is informative."""

    def test_pending_repr(self):
        r = repr(solver_module.SolverStatus.pending)
        assert "pending" in r

    def test_solved_repr(self):
        r = repr(solver_module.SolverStatus.solved)
        assert "solved" in r
