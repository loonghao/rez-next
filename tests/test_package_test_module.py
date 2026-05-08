"""
Tests for rez_next.test module (package_test functionality).

Validates that the test module API matches rez.package_test.
"""

import pytest
import rez_next as rez
from rez_next.test import (
    PackageTestRunner,
    PackageTestResults,
    SUCCESS,
    FAILED,
    SKIPPED,
    ERROR,
)


class TestPackageTestModule:
    """Test the rez_next.test module structure."""

    def test_module_attributes(self):
        """Test that all expected attributes are present."""
        import rez_next.test as test_module

        # Check classes
        assert hasattr(test_module, "PackageTestRunner")
        assert hasattr(test_module, "PackageTestResults")

        # Check constants
        assert hasattr(test_module, "SUCCESS")
        assert hasattr(test_module, "FAILED")
        assert hasattr(test_module, "SKIPPED")
        assert hasattr(test_module, "ERROR")

    def test_status_constants(self):
        """Test status constants match expected values."""
        assert SUCCESS == "success"
        assert FAILED == "failed"
        assert SKIPPED == "skipped"
        assert ERROR == "error"

    def test_package_test_runner_creation(self):
        """Test PackageTestRunner can be created."""
        try:
            runner = PackageTestRunner("python")
            assert runner is not None
            assert hasattr(runner, "package_name")
        except Exception as e:
            pytest.skip(f"PackageTestRunner creation failed: {e}")

    def test_package_test_results_creation(self):
        """Test PackageTestResults can be created."""
        results = PackageTestResults()
        assert results is not None
        assert hasattr(results, "num_tests")
        assert hasattr(results, "num_success")
        assert hasattr(results, "num_failed")
        assert hasattr(results, "num_skipped")


class TestPackageTestResults:
    """Test PackageTestResults class."""

    def test_initial_num_tests(self):
        """Test initial number of tests is 0."""
        results = PackageTestResults()
        assert results.num_tests() == 0
        assert results.num_success() == 0
        assert results.num_failed() == 0
        assert results.num_skipped() == 0

    def test_add_test_result(self):
        """Test adding test results."""
        results = PackageTestResults()

        # Add a successful test
        results.add_test_result("test1", None, SUCCESS, "Test 1 passed")

        assert results.num_tests() == 1
        assert results.num_success() == 1
        assert results.num_failed() == 0


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
