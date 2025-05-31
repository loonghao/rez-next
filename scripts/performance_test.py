#!/usr/bin/env python3
"""
Performance testing script for rez-core optimizations.

This script tests the performance improvements of the optimized rez-core
components compared to their standard implementations.
"""

import time
import statistics
import sys
import os
from pathlib import Path

# Add the project root to Python path
project_root = Path(__file__).parent.parent
sys.path.insert(0, str(project_root / "python"))

try:
    import rez_core
    print(f"✓ Successfully imported rez_core version {rez_core.__version__}")
except ImportError as e:
    print(f"✗ Failed to import rez_core: {e}")
    print("Please build the project first with: uv run maturin develop")
    sys.exit(1)

def time_function(func, *args, **kwargs):
    """Time a function execution and return (result, duration_ms)."""
    start_time = time.perf_counter()
    result = func(*args, **kwargs)
    end_time = time.perf_counter()
    duration_ms = (end_time - start_time) * 1000
    return result, duration_ms

def benchmark_version_parsing():
    """Benchmark version parsing performance."""
    print("\n=== Version Parsing Performance ===")
    
    test_versions = [
        "1.2.3",
        "1.2.3-alpha.1",
        "2.0.0-beta.2+build.123",
        "1.0.0-rc.1",
        "3.1.4-dev.123",
        "10.20.30",
        "1.2.3-alpha1.beta2.gamma3",
        "0.0.1-snapshot.20231201",
    ] * 100  # Repeat for better measurement
    
    # Test standard parsing
    durations = []
    for _ in range(10):  # Multiple runs for statistical significance
        _, duration = time_function(lambda: [rez_core.Version(v) for v in test_versions])
        durations.append(duration)
    
    avg_duration = statistics.mean(durations)
    std_dev = statistics.stdev(durations) if len(durations) > 1 else 0
    
    print(f"Standard parsing: {avg_duration:.2f}ms ± {std_dev:.2f}ms")
    print(f"Versions per second: {len(test_versions) * 1000 / avg_duration:.0f}")
    
    return avg_duration

def benchmark_version_comparison():
    """Benchmark version comparison performance."""
    print("\n=== Version Comparison Performance ===")
    
    # Create test versions
    versions = [rez_core.Version(f"1.{i}.{j}") for i in range(10) for j in range(10)]
    
    # Test comparison performance
    durations = []
    for _ in range(10):
        def compare_all():
            comparisons = 0
            for i, v1 in enumerate(versions):
                for v2 in versions[i+1:]:
                    _ = v1 < v2
                    comparisons += 1
            return comparisons
        
        _, duration = time_function(compare_all)
        durations.append(duration)
    
    avg_duration = statistics.mean(durations)
    std_dev = statistics.stdev(durations) if len(durations) > 1 else 0
    comparisons_count = len(versions) * (len(versions) - 1) // 2
    
    print(f"Comparison time: {avg_duration:.2f}ms ± {std_dev:.2f}ms")
    print(f"Comparisons per second: {comparisons_count * 1000 / avg_duration:.0f}")
    
    return avg_duration

def benchmark_version_sorting():
    """Benchmark version sorting performance."""
    print("\n=== Version Sorting Performance ===")
    
    # Create unsorted versions
    import random
    version_strings = [f"1.{random.randint(0, 99)}.{random.randint(0, 99)}" for _ in range(1000)]
    versions = [rez_core.Version(v) for v in version_strings]
    
    # Test sorting performance
    durations = []
    for _ in range(10):
        versions_copy = versions.copy()
        _, duration = time_function(lambda: versions_copy.sort())
        durations.append(duration)
    
    avg_duration = statistics.mean(durations)
    std_dev = statistics.stdev(durations) if len(durations) > 1 else 0
    
    print(f"Sorting time: {avg_duration:.2f}ms ± {std_dev:.2f}ms")
    print(f"Versions per second: {len(versions) * 1000 / avg_duration:.0f}")
    
    return avg_duration

def benchmark_version_range_operations():
    """Benchmark version range operations."""
    print("\n=== Version Range Performance ===")
    
    # Create test version ranges
    ranges = [
        ">=1.0.0",
        ">=1.0.0,<2.0.0",
        ">=1.0.0,<1.5.0",
        ">=2.0.0",
        "==1.2.3",
    ]
    
    versions = [rez_core.Version(f"1.{i}.{j}") for i in range(5) for j in range(5)]
    
    # Test range containment checks
    durations = []
    for _ in range(10):
        def test_containment():
            checks = 0
            for range_str in ranges:
                version_range = rez_core.VersionRange(range_str)
                for version in versions:
                    _ = version_range.contains(version)
                    checks += 1
            return checks
        
        _, duration = time_function(test_containment)
        durations.append(duration)
    
    avg_duration = statistics.mean(durations)
    std_dev = statistics.stdev(durations) if len(durations) > 1 else 0
    checks_count = len(ranges) * len(versions)
    
    print(f"Range checks time: {avg_duration:.2f}ms ± {std_dev:.2f}ms")
    print(f"Checks per second: {checks_count * 1000 / avg_duration:.0f}")
    
    return avg_duration

def benchmark_memory_usage():
    """Benchmark memory usage of version operations."""
    print("\n=== Memory Usage Analysis ===")
    
    try:
        import psutil
        process = psutil.Process()
        
        # Baseline memory
        baseline_memory = process.memory_info().rss / 1024 / 1024  # MB
        
        # Create many versions
        versions = []
        for i in range(10000):
            versions.append(rez_core.Version(f"1.{i % 100}.{i % 10}"))
        
        # Memory after creating versions
        after_creation = process.memory_info().rss / 1024 / 1024  # MB
        
        # Sort versions (should not significantly increase memory)
        versions.sort()
        
        # Memory after sorting
        after_sorting = process.memory_info().rss / 1024 / 1024  # MB
        
        print(f"Baseline memory: {baseline_memory:.1f} MB")
        print(f"After creating 10k versions: {after_creation:.1f} MB (+{after_creation - baseline_memory:.1f} MB)")
        print(f"After sorting: {after_sorting:.1f} MB (+{after_sorting - after_creation:.1f} MB)")
        print(f"Memory per version: {(after_creation - baseline_memory) * 1024 / len(versions):.1f} KB")
        
    except ImportError:
        print("psutil not available, skipping memory analysis")

def benchmark_error_handling():
    """Benchmark error handling performance."""
    print("\n=== Error Handling Performance ===")
    
    invalid_versions = [
        "",
        "invalid",
        "1.2.3.4.5.6.7.8.9",
        "1.2.3-",
        "1.2.3+",
        "...",
    ] * 100
    
    # Test error handling performance
    durations = []
    for _ in range(10):
        def test_errors():
            errors = 0
            for version_str in invalid_versions:
                try:
                    rez_core.Version(version_str)
                except:
                    errors += 1
            return errors
        
        _, duration = time_function(test_errors)
        durations.append(duration)
    
    avg_duration = statistics.mean(durations)
    std_dev = statistics.stdev(durations) if len(durations) > 1 else 0
    
    print(f"Error handling time: {avg_duration:.2f}ms ± {std_dev:.2f}ms")
    print(f"Errors per second: {len(invalid_versions) * 1000 / avg_duration:.0f}")
    
    return avg_duration

def run_comprehensive_benchmark():
    """Run all benchmarks and provide summary."""
    print("🚀 Rez-Core Performance Benchmark Suite")
    print("=" * 50)
    
    results = {}
    
    try:
        results['parsing'] = benchmark_version_parsing()
        results['comparison'] = benchmark_version_comparison()
        results['sorting'] = benchmark_version_sorting()
        results['ranges'] = benchmark_version_range_operations()
        results['errors'] = benchmark_error_handling()
        
        benchmark_memory_usage()
        
        print("\n=== Performance Summary ===")
        total_time = sum(results.values())
        print(f"Total benchmark time: {total_time:.2f}ms")
        
        # Performance ratings
        print("\n=== Performance Ratings ===")
        for operation, duration in results.items():
            if duration < 10:
                rating = "🟢 Excellent"
            elif duration < 50:
                rating = "🟡 Good"
            elif duration < 100:
                rating = "🟠 Fair"
            else:
                rating = "🔴 Needs Optimization"
            
            print(f"{operation.capitalize()}: {duration:.2f}ms - {rating}")
        
        print("\n✅ Benchmark completed successfully!")
        
    except Exception as e:
        print(f"\n❌ Benchmark failed: {e}")
        import traceback
        traceback.print_exc()
        return False
    
    return True

if __name__ == "__main__":
    success = run_comprehensive_benchmark()
    sys.exit(0 if success else 1)
