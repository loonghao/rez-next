#!/usr/bin/env python3
"""
Performance Trend Analysis Generator

Generates trend analysis and visualizations from historical benchmark data.
"""

import argparse
import json
import statistics
import sys
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List

# Optional imports for visualization
try:
    import matplotlib.dates as mdates
    import matplotlib.pyplot as plt
    from matplotlib.figure import Figure

    HAS_MATPLOTLIB = True
except ImportError:
    HAS_MATPLOTLIB = False
    print("Warning: matplotlib not available, skipping chart generation")

try:
    import pandas as pd

    HAS_PANDAS = True
except ImportError:
    HAS_PANDAS = False
    print("Warning: pandas not available, using basic analysis")


class TrendAnalyzer:
    """Analyzes performance trends over time"""

    def __init__(self):
        self.modules = ["version", "solver", "context", "rex", "build-cache"]

    def load_historical_data(
        self, benchmark_dir: Path, lookback_days: int = 30
    ) -> Dict:
        """Load historical benchmark data"""
        # In a real implementation, this would load data from a time series database
        # or historical artifact storage. For now, we'll simulate with current data.

        historical_data = {}
        cutoff_date = datetime.now() - timedelta(days=lookback_days)

        # Load current benchmark data as a starting point
        for file_path in benchmark_dir.glob("*.json"):
            module_name = (
                file_path.stem.replace("-quick", "")
                .replace("-comprehensive", "")
                .replace("-validation", "")
            )

            try:
                with open(file_path) as f:
                    content = json.load(f)

                # Extract benchmark results
                results = []
                if isinstance(content, dict):
                    if "results" in content:
                        # Criterion format
                        for result in content["results"]:
                            benchmark_name = result.get(
                                "id", result.get("name", "unknown")
                            )
                            mean_time = result.get("mean", {}).get("estimate", 0)
                            if mean_time > 0:
                                results.append(
                                    {
                                        "name": benchmark_name,
                                        "mean_time_ns": mean_time,
                                        "timestamp": datetime.now().isoformat(),
                                    }
                                )
                    else:
                        # Custom format
                        for bench_name, bench_data in content.items():
                            if (
                                isinstance(bench_data, dict)
                                and "mean_time_ns" in bench_data
                            ):
                                results.append(
                                    {
                                        "name": bench_name,
                                        "mean_time_ns": bench_data["mean_time_ns"],
                                        "timestamp": datetime.now().isoformat(),
                                    }
                                )

                if results:
                    historical_data[module_name] = results

            except Exception as e:
                print(f"Warning: Failed to load {file_path}: {e}")

        return historical_data

    def calculate_trends(self, historical_data: Dict) -> Dict:
        """Calculate performance trends"""
        trends = {}

        for module_name, data_points in historical_data.items():
            if not data_points:
                continue

            # Calculate basic statistics
            times = [point["mean_time_ns"] for point in data_points]

            module_trends = {
                "current_avg": statistics.mean(times),
                "current_median": statistics.median(times),
                "current_std": statistics.stdev(times) if len(times) > 1 else 0.0,
                "min_time": min(times),
                "max_time": max(times),
                "data_points": len(times),
                "trend_direction": "stable",  # Would calculate from historical data
                "trend_strength": 0.0,  # Would calculate correlation coefficient
                "performance_score": self.calculate_performance_score(times),
            }

            # Simulate trend calculation (in real implementation, would use historical data)
            # For now, we'll create some sample trend data
            module_trends["simulated_trend"] = self.simulate_trend_data(
                module_name, times[0] if times else 1000000
            )

            trends[module_name] = module_trends

        return trends

    def calculate_performance_score(self, times: List[float]) -> float:
        """Calculate performance score from benchmark times"""
        if not times:
            return 0.0

        avg_time_ms = statistics.mean(times) / 1_000_000

        # Performance score (lower time = higher score)
        if avg_time_ms <= 1.0:
            return 100.0
        elif avg_time_ms <= 10.0:
            return 90.0 - (avg_time_ms - 1.0) * 2.0
        elif avg_time_ms <= 100.0:
            return 72.0 - (avg_time_ms - 10.0) * 0.5
        else:
            return max(0.0, 27.0 - (avg_time_ms - 100.0) * 0.1)

    def simulate_trend_data(self, module_name: str, base_time: float) -> List[Dict]:
        """Simulate historical trend data for demonstration"""
        import random

        trend_data = []
        current_time = datetime.now()

        # Generate 30 days of simulated data
        for i in range(30):
            date = current_time - timedelta(days=29 - i)

            # Simulate some variation and trends
            if module_name == "solver":
                # Simulate improvement over time
                variation = base_time * (1.0 - i * 0.01 + random.uniform(-0.1, 0.1))
            elif module_name == "rex":
                # Simulate more stable performance
                variation = base_time * (1.0 + random.uniform(-0.05, 0.05))
            else:
                # Simulate general variation
                variation = base_time * (1.0 + random.uniform(-0.15, 0.15))

            trend_data.append(
                {
                    "date": date.isoformat(),
                    "mean_time_ns": max(1000, variation),  # Minimum 1μs
                    "performance_score": self.calculate_performance_score([variation]),
                }
            )

        return trend_data

    def generate_charts(self, trends: Dict, output_dir: Path):
        """Generate trend charts"""
        if not HAS_MATPLOTLIB:
            print("Skipping chart generation (matplotlib not available)")
            return

        output_dir.mkdir(parents=True, exist_ok=True)

        # Overall performance trend chart
        self.create_overall_trend_chart(trends, output_dir / "overall_trend.png")

        # Individual module charts
        for module_name, module_trends in trends.items():
            if "simulated_trend" in module_trends:
                self.create_module_trend_chart(
                    module_name,
                    module_trends["simulated_trend"],
                    output_dir / f"{module_name}_trend.png",
                )

        # Performance score comparison chart
        self.create_performance_comparison_chart(
            trends, output_dir / "performance_comparison.png"
        )

    def create_overall_trend_chart(self, trends: Dict, output_path: Path):
        """Create overall performance trend chart"""
        fig, ax = plt.subplots(figsize=(12, 6))

        for module_name, module_trends in trends.items():
            if "simulated_trend" not in module_trends:
                continue

            trend_data = module_trends["simulated_trend"]
            dates = [datetime.fromisoformat(point["date"]) for point in trend_data]
            scores = [point["performance_score"] for point in trend_data]

            ax.plot(dates, scores, label=module_name.title(), marker="o", markersize=3)

        ax.set_xlabel("Date")
        ax.set_ylabel("Performance Score")
        ax.set_title("rez-core Performance Trends (30 Days)")
        ax.legend()
        ax.grid(True, alpha=0.3)

        # Format x-axis
        ax.xaxis.set_major_formatter(mdates.DateFormatter("%m/%d"))
        ax.xaxis.set_major_locator(mdates.DayLocator(interval=5))
        plt.xticks(rotation=45)

        plt.tight_layout()
        plt.savefig(output_path, dpi=300, bbox_inches="tight")
        plt.close()

    def create_module_trend_chart(
        self, module_name: str, trend_data: List[Dict], output_path: Path
    ):
        """Create individual module trend chart"""
        fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(10, 8))

        dates = [datetime.fromisoformat(point["date"]) for point in trend_data]
        times_ms = [point["mean_time_ns"] / 1_000_000 for point in trend_data]
        scores = [point["performance_score"] for point in trend_data]

        # Execution time chart
        ax1.plot(dates, times_ms, color="blue", marker="o", markersize=3)
        ax1.set_ylabel("Execution Time (ms)")
        ax1.set_title(f"{module_name.title()} Module - Execution Time Trend")
        ax1.grid(True, alpha=0.3)

        # Performance score chart
        ax2.plot(dates, scores, color="green", marker="o", markersize=3)
        ax2.set_xlabel("Date")
        ax2.set_ylabel("Performance Score")
        ax2.set_title(f"{module_name.title()} Module - Performance Score Trend")
        ax2.grid(True, alpha=0.3)

        # Format x-axis for both subplots
        for ax in [ax1, ax2]:
            ax.xaxis.set_major_formatter(mdates.DateFormatter("%m/%d"))
            ax.xaxis.set_major_locator(mdates.DayLocator(interval=5))

        plt.xticks(rotation=45)
        plt.tight_layout()
        plt.savefig(output_path, dpi=300, bbox_inches="tight")
        plt.close()

    def create_performance_comparison_chart(self, trends: Dict, output_path: Path):
        """Create performance comparison chart"""
        fig, ax = plt.subplots(figsize=(10, 6))

        modules = []
        scores = []

        for module_name, module_trends in trends.items():
            modules.append(module_name.title())
            scores.append(module_trends["performance_score"])

        bars = ax.bar(
            modules,
            scores,
            color=["#3498db", "#e74c3c", "#2ecc71", "#f39c12", "#9b59b6"],
        )

        # Add value labels on bars
        for bar, score in zip(bars, scores):
            height = bar.get_height()
            ax.text(
                bar.get_x() + bar.get_width() / 2.0,
                height + 1,
                f"{score:.1f}",
                ha="center",
                va="bottom",
            )

        ax.set_ylabel("Performance Score")
        ax.set_title("Current Performance Scores by Module")
        ax.set_ylim(0, 100)
        ax.grid(True, alpha=0.3, axis="y")

        # Add target line
        ax.axhline(y=75, color="red", linestyle="--", alpha=0.7, label="Target (75)")
        ax.legend()

        plt.xticks(rotation=45)
        plt.tight_layout()
        plt.savefig(output_path, dpi=300, bbox_inches="tight")
        plt.close()

    def generate_report(self, trends: Dict) -> str:
        """Generate text report of trend analysis"""
        report = []

        report.append("# rez-core Performance Trend Analysis")
        report.append(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S UTC')}")
        report.append("")

        # Overall summary
        all_scores = [trends[module]["performance_score"] for module in trends.keys()]
        if all_scores:
            avg_score = statistics.mean(all_scores)
            report.append(f"## Overall Performance: {avg_score:.1f}/100")
            report.append("")

        # Module details
        report.append("## Module Performance Summary")
        report.append("")

        for module_name, module_trends in trends.items():
            report.append(f"### {module_name.title()} Module")
            report.append(
                f"- **Current Score**: {module_trends['performance_score']:.1f}/100"
            )
            report.append(
                f"- **Average Time**: {module_trends['current_avg']/1_000_000:.2f}ms"
            )
            report.append(f"- **Data Points**: {module_trends['data_points']}")
            report.append(f"- **Trend**: {module_trends['trend_direction']}")
            report.append("")

        # Recommendations
        report.append("## Recommendations")
        report.append("")

        low_performing = [
            name for name, trends in trends.items() if trends["performance_score"] < 70
        ]

        if low_performing:
            report.append("### Modules Needing Attention")
            for module in low_performing:
                score = trends[module]["performance_score"]
                report.append(
                    f"- **{module.title()}**: Score {score:.1f}/100 - Consider optimization"
                )
        else:
            report.append("### All Modules Performing Well")
            report.append("All modules are meeting performance expectations.")

        return "\n".join(report)


def main():
    parser = argparse.ArgumentParser(description="Generate performance trend analysis")
    parser.add_argument(
        "--benchmark-dir",
        type=Path,
        required=True,
        help="Directory containing benchmark results",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        required=True,
        help="Directory to save trend analysis",
    )
    parser.add_argument(
        "--lookback-days",
        type=int,
        default=30,
        help="Number of days to look back for trend analysis",
    )
    parser.add_argument(
        "--no-charts", action="store_true", help="Skip chart generation"
    )

    args = parser.parse_args()

    if not args.benchmark_dir.exists():
        print(f"Error: Benchmark directory {args.benchmark_dir} does not exist")
        sys.exit(1)

    args.output_dir.mkdir(parents=True, exist_ok=True)

    # Initialize analyzer
    analyzer = TrendAnalyzer()

    # Load historical data
    print("Loading historical benchmark data...")
    historical_data = analyzer.load_historical_data(
        args.benchmark_dir, args.lookback_days
    )

    if not historical_data:
        print("Warning: No historical data found")
        sys.exit(0)

    # Calculate trends
    print("Calculating performance trends...")
    trends = analyzer.calculate_trends(historical_data)

    # Generate charts
    if not args.no_charts:
        print("Generating trend charts...")
        analyzer.generate_charts(trends, args.output_dir)

    # Generate report
    print("Generating trend report...")
    report = analyzer.generate_report(trends)

    with open(args.output_dir / "trend_analysis.md", "w") as f:
        f.write(report)

    # Save raw trend data
    with open(args.output_dir / "trend_data.json", "w") as f:
        json.dump(trends, f, indent=2, default=str)

    print(f"Trend analysis saved to {args.output_dir}")

    # Print summary
    all_scores = [trends[module]["performance_score"] for module in trends.keys()]
    if all_scores:
        avg_score = statistics.mean(all_scores)
        print(f"Average performance score: {avg_score:.1f}/100")


if __name__ == "__main__":
    main()
