#!/usr/bin/env python3
"""
Performance Regression Analysis Script

Analyzes benchmark results to detect performance regressions by comparing
current results with established baselines.
"""

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Dict, List, Optional, Tuple
import statistics
from dataclasses import dataclass
from enum import Enum

class RegressionSeverity(Enum):
    """Severity levels for performance regressions"""
    MINOR = "minor"      # 5-10% regression
    MODERATE = "moderate"  # 10-20% regression
    MAJOR = "major"      # 20-50% regression
    CRITICAL = "critical"  # >50% regression

@dataclass
class BenchmarkResult:
    """Individual benchmark result"""
    name: str
    mean_time_ns: float
    std_dev_ns: float
    throughput_ops_per_sec: Optional[float] = None
    memory_usage_bytes: Optional[int] = None
    additional_metrics: Optional[Dict[str, float]] = None

@dataclass
class RegressionResult:
    """Result of regression analysis"""
    benchmark: str
    baseline_time: float
    current_time: float
    change_percent: float
    severity: RegressionSeverity
    confidence: float
    details: str

class PerformanceAnalyzer:
    """Analyzes performance benchmarks for regressions"""
    
    def __init__(self, regression_threshold: float = 10.0):
        """
        Initialize analyzer
        
        Args:
            regression_threshold: Percentage threshold for detecting regressions
        """
        self.regression_threshold = regression_threshold
        self.severity_thresholds = {
            RegressionSeverity.MINOR: 5.0,
            RegressionSeverity.MODERATE: 10.0,
            RegressionSeverity.MAJOR: 20.0,
            RegressionSeverity.CRITICAL: 50.0,
        }
    
    def load_benchmark_results(self, file_path: Path) -> Dict[str, BenchmarkResult]:
        """Load benchmark results from JSON file"""
        try:
            with open(file_path, 'r') as f:
                data = json.load(f)
            
            results = {}
            
            # Handle different JSON formats (Criterion output)
            if isinstance(data, dict):
                if 'results' in data:
                    # Criterion JSON format
                    for result in data['results']:
                        name = result.get('id', result.get('name', 'unknown'))
                        mean_time = result.get('mean', {}).get('estimate', 0)
                        std_dev = result.get('std_dev', {}).get('estimate', 0)
                        throughput = result.get('throughput', {}).get('per_sec', None)
                        
                        results[name] = BenchmarkResult(
                            name=name,
                            mean_time_ns=mean_time,
                            std_dev_ns=std_dev,
                            throughput_ops_per_sec=throughput
                        )
                else:
                    # Custom format
                    for name, result in data.items():
                        if isinstance(result, dict):
                            results[name] = BenchmarkResult(
                                name=name,
                                mean_time_ns=result.get('mean_time_ns', 0),
                                std_dev_ns=result.get('std_dev_ns', 0),
                                throughput_ops_per_sec=result.get('throughput_ops_per_sec'),
                                memory_usage_bytes=result.get('memory_usage_bytes'),
                                additional_metrics=result.get('additional_metrics')
                            )
            
            return results
            
        except Exception as e:
            print(f"Error loading benchmark results from {file_path}: {e}")
            return {}
    
    def load_all_results(self, directory: Path) -> Dict[str, Dict[str, BenchmarkResult]]:
        """Load all benchmark result files from directory"""
        all_results = {}
        
        for file_path in directory.glob("*.json"):
            module_name = file_path.stem
            results = self.load_benchmark_results(file_path)
            if results:
                all_results[module_name] = results
        
        return all_results
    
    def calculate_regression(self, baseline: BenchmarkResult, current: BenchmarkResult) -> Optional[RegressionResult]:
        """Calculate regression between baseline and current results"""
        if baseline.mean_time_ns == 0:
            return None
        
        # Calculate percentage change (positive = regression, negative = improvement)
        change_percent = ((current.mean_time_ns - baseline.mean_time_ns) / baseline.mean_time_ns) * 100
        
        # Determine severity
        severity = self._determine_severity(abs(change_percent))
        
        # Calculate confidence based on standard deviations
        confidence = self._calculate_confidence(baseline, current)
        
        # Create detailed description
        details = self._create_details(baseline, current, change_percent)
        
        return RegressionResult(
            benchmark=current.name,
            baseline_time=baseline.mean_time_ns,
            current_time=current.mean_time_ns,
            change_percent=change_percent,
            severity=severity,
            confidence=confidence,
            details=details
        )
    
    def _determine_severity(self, change_percent: float) -> RegressionSeverity:
        """Determine regression severity based on percentage change"""
        if change_percent >= self.severity_thresholds[RegressionSeverity.CRITICAL]:
            return RegressionSeverity.CRITICAL
        elif change_percent >= self.severity_thresholds[RegressionSeverity.MAJOR]:
            return RegressionSeverity.MAJOR
        elif change_percent >= self.severity_thresholds[RegressionSeverity.MODERATE]:
            return RegressionSeverity.MODERATE
        else:
            return RegressionSeverity.MINOR
    
    def _calculate_confidence(self, baseline: BenchmarkResult, current: BenchmarkResult) -> float:
        """Calculate confidence in regression detection"""
        # Simple confidence calculation based on coefficient of variation
        baseline_cv = baseline.std_dev_ns / baseline.mean_time_ns if baseline.mean_time_ns > 0 else 1.0
        current_cv = current.std_dev_ns / current.mean_time_ns if current.mean_time_ns > 0 else 1.0
        
        # Lower coefficient of variation = higher confidence
        avg_cv = (baseline_cv + current_cv) / 2
        confidence = max(0.0, min(1.0, 1.0 - avg_cv))
        
        return confidence
    
    def _create_details(self, baseline: BenchmarkResult, current: BenchmarkResult, change_percent: float) -> str:
        """Create detailed description of the regression"""
        direction = "regression" if change_percent > 0 else "improvement"
        
        details = f"{direction.capitalize()} of {abs(change_percent):.2f}% "
        details += f"(baseline: {baseline.mean_time_ns:.0f}ns, current: {current.mean_time_ns:.0f}ns)"
        
        if baseline.throughput_ops_per_sec and current.throughput_ops_per_sec:
            throughput_change = ((current.throughput_ops_per_sec - baseline.throughput_ops_per_sec) / 
                               baseline.throughput_ops_per_sec) * 100
            details += f", throughput change: {throughput_change:.2f}%"
        
        return details
    
    def analyze_regressions(self, current_results: Dict[str, Dict[str, BenchmarkResult]], 
                          baseline_results: Dict[str, Dict[str, BenchmarkResult]]) -> List[RegressionResult]:
        """Analyze all benchmarks for regressions"""
        regressions = []
        
        for module_name, current_module_results in current_results.items():
            if module_name not in baseline_results:
                print(f"Warning: No baseline found for module {module_name}")
                continue
            
            baseline_module_results = baseline_results[module_name]
            
            for benchmark_name, current_result in current_module_results.items():
                if benchmark_name not in baseline_module_results:
                    print(f"Warning: No baseline found for benchmark {module_name}::{benchmark_name}")
                    continue
                
                baseline_result = baseline_module_results[benchmark_name]
                regression = self.calculate_regression(baseline_result, current_result)
                
                if regression and abs(regression.change_percent) >= self.regression_threshold:
                    regressions.append(regression)
        
        return regressions
    
    def generate_report(self, regressions: List[RegressionResult]) -> Dict:
        """Generate comprehensive regression report"""
        # Sort regressions by severity and change percentage
        severity_order = {
            RegressionSeverity.CRITICAL: 4,
            RegressionSeverity.MAJOR: 3,
            RegressionSeverity.MODERATE: 2,
            RegressionSeverity.MINOR: 1
        }
        
        regressions.sort(key=lambda r: (severity_order[r.severity], abs(r.change_percent)), reverse=True)
        
        # Count by severity
        severity_counts = {}
        for severity in RegressionSeverity:
            severity_counts[severity.value] = len([r for r in regressions if r.severity == severity])
        
        # Calculate statistics
        if regressions:
            change_percentages = [abs(r.change_percent) for r in regressions]
            avg_regression = statistics.mean(change_percentages)
            max_regression = max(change_percentages)
            min_regression = min(change_percentages)
        else:
            avg_regression = max_regression = min_regression = 0.0
        
        report = {
            "summary": {
                "total_regressions": len(regressions),
                "severity_counts": severity_counts,
                "avg_regression_percent": avg_regression,
                "max_regression_percent": max_regression,
                "min_regression_percent": min_regression,
                "threshold_used": self.regression_threshold
            },
            "regressions": [
                {
                    "benchmark": r.benchmark,
                    "change": round(r.change_percent, 2),
                    "severity": r.severity.value,
                    "confidence": round(r.confidence, 3),
                    "baseline_time_ns": r.baseline_time,
                    "current_time_ns": r.current_time,
                    "details": r.details
                }
                for r in regressions
            ]
        }
        
        return report

def main():
    parser = argparse.ArgumentParser(description="Analyze performance regressions")
    parser.add_argument("--current-dir", type=Path, required=True,
                       help="Directory containing current benchmark results")
    parser.add_argument("--baseline-dir", type=Path, required=True,
                       help="Directory containing baseline benchmark results")
    parser.add_argument("--threshold", type=float, default=10.0,
                       help="Regression threshold percentage (default: 10.0)")
    parser.add_argument("--output", type=Path, default="regression-analysis.json",
                       help="Output file for regression analysis")
    parser.add_argument("--verbose", action="store_true",
                       help="Enable verbose output")
    
    args = parser.parse_args()
    
    if not args.current_dir.exists():
        print(f"Error: Current results directory {args.current_dir} does not exist")
        sys.exit(1)
    
    if not args.baseline_dir.exists():
        print(f"Warning: Baseline directory {args.baseline_dir} does not exist")
        # Create empty baseline for first run
        args.baseline_dir.mkdir(parents=True, exist_ok=True)
    
    # Initialize analyzer
    analyzer = PerformanceAnalyzer(regression_threshold=args.threshold)
    
    # Load results
    if args.verbose:
        print("Loading current benchmark results...")
    current_results = analyzer.load_all_results(args.current_dir)
    
    if args.verbose:
        print("Loading baseline benchmark results...")
    baseline_results = analyzer.load_all_results(args.baseline_dir)
    
    if not current_results:
        print("Warning: No current benchmark results found")
        sys.exit(0)
    
    if not baseline_results:
        print("Warning: No baseline benchmark results found")
        # Create empty report for first run
        report = {
            "summary": {
                "total_regressions": 0,
                "severity_counts": {s.value: 0 for s in RegressionSeverity},
                "avg_regression_percent": 0.0,
                "max_regression_percent": 0.0,
                "min_regression_percent": 0.0,
                "threshold_used": args.threshold
            },
            "regressions": []
        }
    else:
        # Analyze regressions
        if args.verbose:
            print("Analyzing performance regressions...")
        regressions = analyzer.analyze_regressions(current_results, baseline_results)
        
        # Generate report
        report = analyzer.generate_report(regressions)
        
        if args.verbose:
            print(f"Found {len(regressions)} regressions above {args.threshold}% threshold")
            for regression in regressions:
                print(f"  {regression.benchmark}: {regression.change_percent:+.2f}% ({regression.severity.value})")
    
    # Save report
    with open(args.output, 'w') as f:
        json.dump(report, f, indent=2)
    
    if args.verbose:
        print(f"Regression analysis saved to {args.output}")
    
    # Exit with error code if critical regressions found
    critical_regressions = len([r for r in report["regressions"] if r["severity"] == "critical"])
    if critical_regressions > 0:
        print(f"CRITICAL: {critical_regressions} critical performance regressions detected!")
        sys.exit(1)

if __name__ == "__main__":
    main()
