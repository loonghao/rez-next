"""
Pytest configuration and fixtures for rez-core tests.

This module provides common fixtures and configuration for testing
the rez-core Python bindings.
"""

import shutil
import tempfile
from pathlib import Path
from typing import Any, Dict, Generator

import pytest
import rez_core


@pytest.fixture
def temp_dir() -> Generator[Path, None, None]:
    """Create a temporary directory for tests."""
    temp_path = Path(tempfile.mkdtemp())
    try:
        yield temp_path
    finally:
        shutil.rmtree(temp_path, ignore_errors=True)


@pytest.fixture
def sample_versions():
    """Provide sample version strings for testing."""
    return [
        "1.0.0",
        "2.1.3",
        "0.9.12",
        "10.0.0-alpha.1",
        "1.2.3-beta.2+build.123",
        "3.0.0-rc.1",
    ]


@pytest.fixture
def sample_version_ranges():
    """Provide sample version range strings for testing."""
    return [
        ">=1.0.0",
        "<2.0.0",
        ">=1.0.0,<2.0.0",
        "~1.2.0",
        "^1.0.0",
        "==1.2.3",
    ]


@pytest.fixture
def config():
    """Provide a default configuration for testing."""
    return rez_core.Config()


class VersionTestCase:
    """Test case data for version testing."""

    def __init__(self, version_str: str, expected_parts: Dict[str, Any] = None):
        self.version_str = version_str
        self.expected_parts = expected_parts or {}


@pytest.fixture
def version_test_cases():
    """Provide comprehensive version test cases."""
    return [
        VersionTestCase("1.0.0", {"major": 1, "minor": 0, "patch": 0}),
        VersionTestCase("2.1.3", {"major": 2, "minor": 1, "patch": 3}),
        VersionTestCase("0.9.12", {"major": 0, "minor": 9, "patch": 12}),
        VersionTestCase("10.0.0", {"major": 10, "minor": 0, "patch": 0}),
    ]


@pytest.fixture
def comparison_test_cases():
    """Provide version comparison test cases."""
    return [
        ("1.0.0", "2.0.0", "less"),
        ("2.0.0", "1.0.0", "greater"),
        ("1.0.0", "1.0.0", "equal"),
        ("1.0.0", "1.0.1", "less"),
        ("1.1.0", "1.0.9", "greater"),
        ("2.0.0-alpha", "2.0.0", "less"),
        ("2.0.0", "2.0.0-alpha", "greater"),
    ]


# Pytest markers for categorizing tests
pytest_plugins = []


def pytest_configure(config):
    """Configure pytest with custom markers."""
    config.addinivalue_line("markers", "unit: mark test as a unit test")
    config.addinivalue_line("markers", "integration: mark test as an integration test")
    config.addinivalue_line("markers", "performance: mark test as a performance test")
    config.addinivalue_line("markers", "slow: mark test as slow running")
    config.addinivalue_line(
        "markers", "compat: mark test as compatibility test with original rez"
    )


def pytest_collection_modifyitems(config, items):
    """Modify test collection to add markers based on test location."""
    for item in items:
        # Add unit marker to tests in unit directories
        if "unit" in str(item.fspath):
            item.add_marker(pytest.mark.unit)

        # Add integration marker to tests in integration directories
        if "integration" in str(item.fspath):
            item.add_marker(pytest.mark.integration)

        # Add performance marker to performance tests
        if "performance" in str(item.fspath) or "benchmark" in item.name:
            item.add_marker(pytest.mark.performance)

        # Add slow marker to tests that might be slow
        if "slow" in item.name or "stress" in item.name:
            item.add_marker(pytest.mark.slow)
