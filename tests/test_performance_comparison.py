"""
Performance comparison tests: rez vs rez_next.

Measures key operations and records timing differences.
"""

import time
import statistics
from typing import Callable, Any

import pytest

try:
    import rez as rez_original
    from rez.version import Version as RezVersion
    from rez.version import VersionRange as RezVersionRange
    HAS_REZ = rez_original.__name__ != "rez_next"
except ImportError:
    HAS_REZ = False
    RezVersion = None
    RezVersionRange = None

try:
    import rez_next as rez_next
    HAS_REZ_NEXT = True
except ImportError:
    HAS_REZ_NEXT = False


# ── Helpers ─────────────────────────────────────────────────────────────

def _time_call(fn: Callable, iterations: int = 100) -> list[float]:
    """Time `fn()` over `iterations` calls. Returns list of elapsed seconds."""
    times = []
    for _ in range(iterations):
        start = time.perf_counter()
        fn()
        end = time.perf_counter()
        times.append(end - start)
    return times


def _stats(times: list[float]) -> dict[str, float]:
    """Calculate mean, median, min, max, stdev."""
    return {
        "mean": statistics.mean(times),
        "median": statistics.median(times),
        "min": min(times),
        "max": max(times),
        "stdev": statistics.stdev(times) if len(times) > 1 else 0.0,
    }


def _format_us(seconds: float) -> str:
    """Format seconds as microseconds."""
    return f"{seconds * 1_000_000:.2f} µs"


def _print_comparison(name: str, rez_times: list[float] | None, rez_next_times: list[float] | None):
    """Print performance comparison."""
    print(f"\n── {name} ──")
    if rez_times:
        s = _stats(rez_times)
        print(f"  rez:      mean={_format_us(s['mean'])}, median={_format_us(s['median'])}, min={_format_us(s['min'])}, max={_format_us(s['max'])}, stdev={_format_us(s['stdev'])}")
    if rez_next_times:
        s = _stats(rez_next_times)
        print(f"  rez_next: mean={_format_us(s['mean'])}, median={_format_us(s['median'])}, min={_format_us(s['min'])}, max={_format_us(s['max'])}, stdev={_format_us(s['stdev'])}")
    if rez_times and rez_next_times:
        speedup = _stats(rez_times)["mean"] / _stats(rez_next_times)["mean"]
        print(f"  → rez_next is {speedup:.2f}x {'faster' if speedup > 1 else 'slower'}")


# ── Version Parsing ────────────────────────────────────────────────────

@pytest.mark.skipif(not HAS_REZ, reason="rez not installed")
@pytest.mark.skipif(not HAS_REZ_NEXT, reason="rez_next not installed")
class TestVersionParsingPerformance:
    """Compare version parsing performance."""

    def test_parse_simple_version(self):
        """Parse a simple version string (e.g., '1.2.3')."""

        def rez_op():
            RezVersion("1.2.3")

        def rez_next_op():
            rez_next.Version("1.2.3")

        rez_times = _time_call(rez_op, iterations=1000) if HAS_REZ else None
        rez_next_times = _time_call(rez_next_op, iterations=1000) if HAS_REZ_NEXT else None

        _print_comparison("Parse simple version '1.2.3'", rez_times, rez_next_times)

    def test_parse_complex_version(self):
        """Parse a complex version string (e.g., '1.2.3alpha4.5beta6-7')."""

        def rez_op():
            RezVersion("1.2.3alpha4.5beta6-7")

        def rez_next_op():
            rez_next.Version("1.2.3alpha4.5beta6-7")

        rez_times = _time_call(rez_op, iterations=1000) if HAS_REZ else None
        rez_next_times = _time_call(rez_next_op, iterations=1000) if HAS_REZ_NEXT else None

        _print_comparison("Parse complex version '1.2.3alpha4.5beta6-7'", rez_times, rez_next_times)

    def test_parse_version_with_local(self):
        """Parse a version with local identifier (e.g., '1.2.3+local.4')."""

        def rez_op():
            RezVersion("1.2.3+local.4")

        def rez_next_op():
            rez_next.Version("1.2.3+local.4")

        rez_times = _time_call(rez_op, iterations=1000) if HAS_REZ else None
        rez_next_times = _time_call(rez_next_op, iterations=1000) if HAS_REZ_NEXT else None

        _print_comparison("Parse version with local '1.2.3+local.4'", rez_times, rez_next_times)


# ── Version Range Parsing ─────────────────────────────────────────────

@pytest.mark.skipif(not HAS_REZ, reason="rez not installed")
@pytest.mark.skipif(not HAS_REZ_NEXT, reason="rez_next not installed")
class TestVersionRangeParsingPerformance:
    """Compare version range parsing performance."""

    def test_parse_simple_range(self):
        """Parse a simple version range (e.g., '>=1.0,<2.0')."""

        def rez_op():
            RezVersionRange(">=1.0,<2.0")

        def rez_next_op():
            rez_next.VersionRange(">=1.0,<2.0")

        rez_times = _time_call(rez_op, iterations=1000) if HAS_REZ else None
        rez_next_times = _time_call(rez_next_op, iterations=1000) if HAS_REZ_NEXT else None

        _print_comparison("Parse simple range '>=1.0,<2.0'", rez_times, rez_next_times)

    def test_parse_complex_range(self):
        """Parse a complex version range (e.g., '>=1.0,<2.0 | >=3.0')."""

        def rez_op():
            # Note: rez uses '|' for union, but let's skip if syntax is wrong
            try:
                RezVersionRange(">=1.0,<2.0 | >=3.0")
            except Exception:
                return None
            return RezVersionRange(">=1.0,<2.0")

        def rez_next_op():
            # Note: rez_next uses '|' for union
            try:
                rez_next.VersionRange(">=1.0,<2.0 | >=3.0")
            except Exception:
                return None
            return rez_next.VersionRange(">=1.0,<2.0")

        rez_times = _time_call(rez_op, iterations=1000) if HAS_REZ else None
        rez_next_times = _time_call(rez_next_op, iterations=1000) if HAS_REZ_NEXT else None

        _print_comparison("Parse complex range '>=1.0,<2.0 | >=3.0'", rez_times, rez_next_times)


# ── Version Comparison ────────────────────────────────────────────────

@pytest.mark.skipif(not HAS_REZ, reason="rez not installed")
@pytest.mark.skipif(not HAS_REZ_NEXT, reason="rez_next not installed")
class TestVersionComparisonPerformance:
    """Compare version comparison performance."""

    def test_version_lt(self):
        """Compare two versions with '<'."""
        v1_rez = RezVersion("1.2.3")
        v2_rez = RezVersion("2.0.0")
        v1_next = rez_next.Version("1.2.3")
        v2_next = rez_next.Version("2.0.0")

        def rez_op():
            return v1_rez < v2_rez

        def rez_next_op():
            return v1_next < v2_next

        rez_times = _time_call(rez_op, iterations=10000) if HAS_REZ else None
        rez_next_times = _time_call(rez_next_op, iterations=10000) if HAS_REZ_NEXT else None

        _print_comparison("Version comparison '1.2.3' < '2.0.0'", rez_times, rez_next_times)


# ── Version Range Contains ────────────────────────────────────────────

@pytest.mark.skipif(not HAS_REZ, reason="rez not installed")
@pytest.mark.skipif(not HAS_REZ_NEXT, reason="rez_next not installed")
class TestVersionRangeContainsPerformance:
    """Compare version range 'contains' performance."""

    def test_range_contains(self):
        """Check if a version is in a range."""
        rng_rez = RezVersionRange(">=1.0,<2.0")
        v_rez = RezVersion("1.5.0")
        rng_next = rez_next.VersionRange(">=1.0,<2.0")
        v_next = rez_next.Version("1.5.0")

        def rez_op():
            # rez uses 'in' operator
            return v_rez in rng_rez

        def rez_next_op():
            # rez_next uses contains() method
            return rng_next.contains(v_next)

        rez_times = _time_call(rez_op, iterations=10000) if HAS_REZ else None
        rez_next_times = _time_call(rez_next_op, iterations=10000) if HAS_REZ_NEXT else None

        _print_comparison("Range '>=1.0,<2.0' contains '1.5.0'", rez_times, rez_next_times)


# ── Package Query ────────────────────────────────────────────────────

@pytest.mark.skipif(not HAS_REZ, reason="rez not installed")
@pytest.mark.skipif(not HAS_REZ_NEXT, reason="rez_next not installed")
class TestPackageQueryPerformance:
    """Compare package query performance."""

    def test_get_latest_package(self):
        """Get the latest version of a package."""
        # Try to import from rez - may need to adjust based on rez version
        try:
            from rez.packages_ import get_latest_package as rez_get_latest
        except ImportError:
            from rez.packages import get_latest_package as rez_get_latest

        from rez_next.packages_ import get_latest_package as next_get_latest

        def rez_op():
            return rez_get_latest("python")

        def rez_next_op():
            return next_get_latest("python")

        # Use fewer iterations for I/O-bound operations
        rez_times = _time_call(rez_op, iterations=10) if HAS_REZ else None
        rez_next_times = _time_call(rez_next_op, iterations=10) if HAS_REZ_NEXT else None

        _print_comparison("Get latest package 'python'", rez_times, rez_next_times)

    def test_iter_packages(self):
        """Iterate over all versions of a package."""
        # Try to import from rez - may need to adjust based on rez version
        try:
            from rez.packages_ import iter_packages as rez_iter
        except ImportError:
            from rez.packages import iter_packages as rez_iter

        from rez_next.packages_ import iter_packages as next_iter

        def rez_op():
            return list(rez_iter("python"))

        def rez_next_op():
            return list(next_iter("python"))

        # Use fewer iterations for I/O-bound operations
        rez_times = _time_call(rez_op, iterations=10) if HAS_REZ else None
        rez_next_times = _time_call(rez_next_op, iterations=10) if HAS_REZ_NEXT else None

        _print_comparison("Iterate packages 'python'", rez_times, rez_next_times)


# ── Summary ──────────────────────────────────────────────────────────

def test_print_performance_summary():
    """Print performance summary (placeholder for future expansion)."""
    print("\n" + "=" * 60)
    print("Performance Comparison: rez vs rez_next")
    print("=" * 60)
    print("\nKey findings:")
    print("  - Rust-based rez_next should be faster for CPU-bound operations")
    print("  - I/O-bound operations (package queries) may have similar performance")
    print("\nSee individual test output for detailed measurements.")
    print("=" * 60)
