#!/usr/bin/env python3
"""Python-layer performance benchmark: rez_next (Rust binding) vs pure-Python fallback.

This script measures key operations through the rez_next Python API and compares
them against the published rez baseline timings
(metrics/benchmarking/data/rez_baseline.json).

It does NOT require rez itself to be installed; comparison against the rez Python
baseline is done by reading the static JSON file.

Usage
-----
    # Requires: maturin develop (from crates/rez-next-python)
    python metrics/benchmarking/scripts/python_bench.py

    # JSON output (for CI / generate_results.py):
    python metrics/benchmarking/scripts/python_bench.py --json /tmp/py_bench.json

    # Quick mode (fewer iterations):
    python metrics/benchmarking/scripts/python_bench.py --quick

Output columns
--------------
  operation     : human-readable name
  rez_baseline  : official rez Python mean time (from rez_baseline.json)
  rez_next_mean : measured rez_next mean time per call
  speedup       : rez_baseline / rez_next_mean  (>1 = faster than rez)
  iterations    : number of timed calls used for the measurement
"""

from __future__ import annotations

import argparse
import json
import statistics
import sys
import time
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Callable, Optional

REPO_ROOT = Path(__file__).resolve().parents[3]
DATA_DIR = Path(__file__).resolve().parent.parent / "data"

# ---------------------------------------------------------------------------
# Result dataclass
# ---------------------------------------------------------------------------


@dataclass
class PythonBenchResult:
    operation: str
    iterations: int
    mean_us: float          # microseconds per call
    median_us: float
    stdev_us: float
    min_us: float
    max_us: float
    rez_baseline_ms: Optional[float]    # official rez mean in ms (None if not in baseline)
    speedup: Optional[float]            # rez_baseline_ms * 1000 / mean_us  (higher = faster)
    error: Optional[str] = None         # set when the operation could not be measured


@dataclass
class PythonBenchSuite:
    timestamp: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    python_version: str = field(default_factory=lambda: sys.version.split()[0])
    rez_next_available: bool = False
    results: list[PythonBenchResult] = field(default_factory=list)

    def as_dict(self) -> dict:
        return {
            "timestamp": self.timestamp,
            "python_version": self.python_version,
            "rez_next_available": self.rez_next_available,
            "results": [asdict(r) for r in self.results],
        }


# ---------------------------------------------------------------------------
# Timing helpers
# ---------------------------------------------------------------------------


def _timeit(fn: Callable, iterations: int) -> list[float]:
    """Return list of per-call durations in microseconds."""
    timings = []
    for _ in range(iterations):
        t0 = time.perf_counter()
        fn()
        t1 = time.perf_counter()
        timings.append((t1 - t0) * 1_000_000)
    return timings


def _measure(
    operation: str,
    fn: Callable,
    iterations: int,
    rez_baseline_ms: Optional[float] = None,
) -> PythonBenchResult:
    try:
        timings = _timeit(fn, iterations)
        mean = statistics.mean(timings)
        median = statistics.median(timings)
        stdev = statistics.stdev(timings) if len(timings) > 1 else 0.0
        speedup = (rez_baseline_ms * 1000 / mean) if rez_baseline_ms and mean > 0 else None
        return PythonBenchResult(
            operation=operation,
            iterations=iterations,
            mean_us=round(mean, 3),
            median_us=round(median, 3),
            stdev_us=round(stdev, 3),
            min_us=round(min(timings), 3),
            max_us=round(max(timings), 3),
            rez_baseline_ms=rez_baseline_ms,
            speedup=round(speedup, 1) if speedup else None,
        )
    except Exception as exc:  # noqa: BLE001
        return PythonBenchResult(
            operation=operation,
            iterations=0,
            mean_us=0.0,
            median_us=0.0,
            stdev_us=0.0,
            min_us=0.0,
            max_us=0.0,
            rez_baseline_ms=rez_baseline_ms,
            speedup=None,
            error=str(exc),
        )


# ---------------------------------------------------------------------------
# Load rez baseline data
# ---------------------------------------------------------------------------


def _load_baseline() -> dict[str, float]:
    """Return mapping: operation_key → mean_ms from rez_baseline.json."""
    path = DATA_DIR / "rez_baseline.json"
    if not path.exists():
        return {}
    data = json.loads(path.read_text())
    ops = data.get("operations", {})
    return {k: v.get("mean_ms", 0.0) for k, v in ops.items()}


# ---------------------------------------------------------------------------
# Benchmark definitions
# ---------------------------------------------------------------------------


def run_benchmarks(quick: bool = False) -> PythonBenchSuite:
    suite = PythonBenchSuite()
    baseline = _load_baseline()

    n_single = 100 if quick else 1000
    n_batch = 20 if quick else 200

    # Try importing rez_next bindings
    try:
        import rez_next  # noqa: F401

        suite.rez_next_available = True
    except ImportError:
        suite.rez_next_available = False

    # ------------------------------------------------------------------
    # 1. Version.parse — single call (matches baseline "version_parse_1000")
    # ------------------------------------------------------------------
    if suite.rez_next_available:
        from rez_next import Version

        suite.results.append(
            _measure(
                "version_parse_single",
                lambda: Version("2.3.1"),
                iterations=n_single,
                rez_baseline_ms=baseline.get("version_parse_1000"),
            )
        )

        # Batch 1000 version parses as a single timed call
        versions = [f"{i}.{j}.{k}" for i in range(5) for j in range(10) for k in range(20)][:1000]

        def _batch_version_parse():
            for v in versions:
                Version(v)

        suite.results.append(
            _measure(
                "version_parse_batch_1000",
                _batch_version_parse,
                iterations=n_batch,
                rez_baseline_ms=baseline.get("version_parse_1000"),
            )
        )
    else:
        for op in ("version_parse_single", "version_parse_batch_1000"):
            suite.results.append(
                PythonBenchResult(
                    operation=op,
                    iterations=0,
                    mean_us=0.0,
                    median_us=0.0,
                    stdev_us=0.0,
                    min_us=0.0,
                    max_us=0.0,
                    rez_baseline_ms=baseline.get("version_parse_1000"),
                    speedup=None,
                    error="rez_next not available (run: maturin develop)",
                )
            )

    # ------------------------------------------------------------------
    # 2. VersionRange creation (matches baseline "version_range_parse_1000")
    # ------------------------------------------------------------------
    if suite.rez_next_available:
        from rez_next import VersionRange

        suite.results.append(
            _measure(
                "version_range_parse_single",
                lambda: VersionRange(">=1.0,<2.0"),
                iterations=n_single,
                rez_baseline_ms=baseline.get("version_range_parse_1000"),
            )
        )

        ranges = [
            f">={i}.{j}" for i in range(10) for j in range(100)
        ][:1000]

        def _batch_range_parse():
            for r in ranges:
                VersionRange(r)

        suite.results.append(
            _measure(
                "version_range_parse_batch_1000",
                _batch_range_parse,
                iterations=n_batch,
                rez_baseline_ms=baseline.get("version_range_parse_1000"),
            )
        )
    else:
        for op in ("version_range_parse_single", "version_range_parse_batch_1000"):
            suite.results.append(
                PythonBenchResult(
                    operation=op,
                    iterations=0,
                    mean_us=0.0,
                    median_us=0.0,
                    stdev_us=0.0,
                    min_us=0.0,
                    max_us=0.0,
                    rez_baseline_ms=baseline.get("version_range_parse_1000"),
                    speedup=None,
                    error="rez_next not available",
                )
            )

    # ------------------------------------------------------------------
    # 3. Requirement (PackageRequest) parsing (matches baseline "req_parse_1000")
    # ------------------------------------------------------------------
    if suite.rez_next_available:
        from rez_next import Requirement

        suite.results.append(
            _measure(
                "requirement_parse_single",
                lambda: Requirement("python>=3.8"),
                iterations=n_single,
                rez_baseline_ms=baseline.get("req_parse_1000"),
            )
        )

        pkgs = ["python", "maya", "houdini", "nuke", "numpy"] * 200
        constraints = [">=2.0", "<3.0", "==1.2.3", ">=1.0,<2.0", ""] * 200
        reqs = [f"{p}{c}" for p, c in zip(pkgs, constraints)][:1000]

        def _batch_req_parse():
            for r in reqs:
                Requirement(r)

        suite.results.append(
            _measure(
                "requirement_parse_batch_1000",
                _batch_req_parse,
                iterations=n_batch,
                rez_baseline_ms=baseline.get("req_parse_1000"),
            )
        )
    else:
        for op in ("requirement_parse_single", "requirement_parse_batch_1000"):
            suite.results.append(
                PythonBenchResult(
                    operation=op,
                    iterations=0,
                    mean_us=0.0,
                    median_us=0.0,
                    stdev_us=0.0,
                    min_us=0.0,
                    max_us=0.0,
                    rez_baseline_ms=baseline.get("req_parse_1000"),
                    speedup=None,
                    error="rez_next not available",
                )
            )

    # ------------------------------------------------------------------
    # 4. RepositoryManager construction (proxy for startup cost)
    # ------------------------------------------------------------------
    if suite.rez_next_available:
        from rez_next import RepositoryManager

        suite.results.append(
            _measure(
                "repository_manager_create",
                lambda: RepositoryManager([]),
                iterations=n_single,
                rez_baseline_ms=baseline.get("startup_import"),
            )
        )
    else:
        suite.results.append(
            PythonBenchResult(
                operation="repository_manager_create",
                iterations=0,
                mean_us=0.0,
                median_us=0.0,
                stdev_us=0.0,
                min_us=0.0,
                max_us=0.0,
                rez_baseline_ms=baseline.get("startup_import"),
                speedup=None,
                error="rez_next not available",
            )
        )

    # ------------------------------------------------------------------
    # 5. Version comparison operators (no direct baseline — microbenchmark)
    # ------------------------------------------------------------------
    if suite.rez_next_available:
        from rez_next import Version

        a = Version("2.0.0")
        b = Version("3.1.0")

        suite.results.append(
            _measure(
                "version_compare_lt",
                lambda: a < b,
                iterations=n_single * 5,
                rez_baseline_ms=None,
            )
        )
    else:
        suite.results.append(
            PythonBenchResult(
                operation="version_compare_lt",
                iterations=0,
                mean_us=0.0,
                median_us=0.0,
                stdev_us=0.0,
                min_us=0.0,
                max_us=0.0,
                rez_baseline_ms=None,
                speedup=None,
                error="rez_next not available",
            )
        )

    # ------------------------------------------------------------------
    # 6. VersionRange.contains (microbenchmark)
    # ------------------------------------------------------------------
    if suite.rez_next_available:
        from rez_next import Version, VersionRange

        vr = VersionRange(">=2.0,<3.0")
        v_in = Version("2.5.0")
        v_out = Version("3.5.0")

        def _contains_check():
            vr.contains(v_in)
            vr.contains(v_out)

        suite.results.append(
            _measure(
                "version_range_contains",
                _contains_check,
                iterations=n_single * 5,
                rez_baseline_ms=None,
            )
        )
    else:
        suite.results.append(
            PythonBenchResult(
                operation="version_range_contains",
                iterations=0,
                mean_us=0.0,
                median_us=0.0,
                stdev_us=0.0,
                min_us=0.0,
                max_us=0.0,
                rez_baseline_ms=None,
                speedup=None,
                error="rez_next not available",
            )
        )

    return suite


# ---------------------------------------------------------------------------
# Reporting
# ---------------------------------------------------------------------------


def _print_table(suite: PythonBenchSuite) -> None:
    ts = suite.timestamp[:19].replace("T", " ")
    print(f"\n{'='*72}")
    print(f"  rez-next Python Binding Benchmark  [{ts} UTC]")
    print(f"  Python {suite.python_version}  |  rez_next available: {suite.rez_next_available}")
    print(f"{'='*72}")
    print(
        f"  {'Operation':<35} {'Mean':>10} {'Baseline':>12} {'Speedup':>10}"
    )
    print(f"  {'-'*35} {'-'*10} {'-'*12} {'-'*10}")

    for r in suite.results:
        if r.error:
            print(f"  {r.operation:<35} {'SKIP':>10}   (! {r.error[:30]})")
            continue
        mean_str = f"{r.mean_us:.2f} µs"
        base_str = f"{r.rez_baseline_ms:.1f} ms" if r.rez_baseline_ms else "—"
        sp_str = f"{r.speedup:.1f}×" if r.speedup else "—"
        print(f"  {r.operation:<35} {mean_str:>10} {base_str:>12} {sp_str:>10}")

    print(f"{'='*72}\n")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Python-layer performance benchmark for rez_next bindings"
    )
    parser.add_argument(
        "--quick",
        action="store_true",
        help="Run fewer iterations for a faster result",
    )
    parser.add_argument(
        "--json",
        metavar="FILE",
        help="Write results as JSON to FILE (for CI integration)",
    )
    args = parser.parse_args()

    suite = run_benchmarks(quick=args.quick)

    _print_table(suite)

    if args.json:
        out = Path(args.json)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(json.dumps(suite.as_dict(), indent=2, ensure_ascii=False))
        print(f"[OK] JSON written: {out}")


if __name__ == "__main__":
    main()
