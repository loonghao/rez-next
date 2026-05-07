#!/usr/bin/env python
"""Simple performance benchmark: rez vs rez_next.

This script compares the performance of key operations between
the original rez package and the rez_next Rust implementation.

Run with: python benchmarks/bench_rez_vs_rez_next.py
"""

import time
import statistics
from typing import Callable, Optional


def measure_time(func: Callable, iterations: int = 100) -> dict:
    """Measure execution time of a function."""
    times = []
    for _ in range(iterations):
        start = time.perf_counter()
        func()
        end = time.perf_counter()
        times.append(end - start)
    
    return {
        'mean': statistics.mean(times),
        'median': statistics.median(times),
        'stdev': statistics.stdev(times) if len(times) > 1 else 0,
        'min': min(times),
        'max': max(times),
        'iterations': iterations,
    }


def format_result(name: str, rez_stats: Optional[dict], rez_next_stats: Optional[dict]) -> str:
    """Format benchmark comparison result."""
    lines = ["=" * 60, f"Benchmark: {name}", "=" * 60]
    
    if rez_stats:
        lines.append(f"rez:      mean={rez_stats['mean']:.6f}s, median={rez_stats['median']:.6f}s")
    
    if rez_next_stats:
        lines.append(f"rez_next: mean={rez_next_stats['mean']:.6f}s, median={rez_next_stats['median']:.6f}s")
    
    if rez_stats and rez_next_stats:
        speedup = rez_stats['mean'] / rez_next_stats['mean']
        if speedup > 1:
            lines.append(f"Speedup: {speedup:.2f}x (rez_next is faster)")
        else:
            lines.append(f"Speedup: {1/speedup:.2f}x (rez is faster)")
    
    lines.append("=" * 60)
    return "\n".join(lines)


def benchmark_version_parsing():
    """Benchmark version parsing performance."""
    print("\n--- Version Parsing Benchmark ---")
    
    versions = ["1.2.3", "2.0.0", "0.1.0a1", "1.2.3b2", "1.0.0rc1", "3.14.159"]
    
    try:
        import rez
        def rez_version():
            for v in versions:
                _ = rez.Version(v)
        rez_stats = measure_time(rez_version, iterations=1000)
    except ImportError:
        rez_stats = None
        print("  rez not available, skipping...")
    
    try:
        import rez_next
        def rez_next_version():
            for v in versions:
                _ = rez_next.Version(v)
        rez_next_stats = measure_time(rez_next_version, iterations=1000)
    except ImportError:
        rez_next_stats = None
        print("  rez_next not available, skipping...")
    
    print(format_result("Version Parsing (x6 variants, 1000 iterations)", rez_stats, rez_next_stats))


def benchmark_import_time():
    """Benchmark module import time."""
    print("\n--- Import Time Benchmark ---")
    
    try:
        def import_rez():
            import rez
        rez_stats = measure_time(import_rez, iterations=10)
    except ImportError:
        rez_stats = None
    
    try:
        def import_rez_next():
            import rez_next
        rez_next_stats = measure_time(import_rez_next, iterations=10)
    except ImportError:
        rez_next_stats = None
    
    print(format_result("Module Import Time (10 iterations)", rez_stats, rez_next_stats))


def benchmark_package_query():
    """Benchmark package query operations."""
    print("\n--- Package Query Benchmark ---")
    
    try:
        import rez.packages as rez_pkg
        def rez_get_latest():
            _ = rez_pkg.get_latest_package("python")
        rez_stats = measure_time(rez_get_latest, iterations=10)
    except (ImportError, Exception) as e:
        rez_stats = None
        print(f"  rez package query failed: {e}")
    
    try:
        import rez_next.packages_ as rez_next_pkg
        def rez_next_get_latest():
            _ = rez_next_pkg.get_latest_package("python")
        rez_next_stats = measure_time(rez_next_get_latest, iterations=10)
    except (ImportError, Exception) as e:
        rez_next_stats = None
        print(f"  rez_next package query failed: {e}")
    
    print(format_result("Package Query: get_latest_package('python')", rez_stats, rez_next_stats))


def main():
    """Run all benchmarks."""
    print("=" * 60)
    print("Rez vs Rez-Next Performance Benchmark")
    print("=" * 60)
    
    benchmark_version_parsing()
    benchmark_import_time()
    benchmark_package_query()
    
    print("\n" + "=" * 60)
    print("Benchmark completed!")
    print("=" * 60)


if __name__ == "__main__":
    main()
