#!/usr/bin/env python3
"""
Performance Badge Generator

Creates performance badges for README and documentation.
"""

import argparse
import json
import statistics
import sys
from pathlib import Path
from typing import Dict, List


class PerformanceBadgeGenerator:
    """Generates performance badges from benchmark data"""

    def __init__(self):
        self.modules = ["version", "solver", "context", "rex", "build-cache"]
        self.performance_targets = {
            "solver": 75.0,  # Target performance score
            "context": 80.0,
            "rex": 85.0,  # Higher target due to 75x improvement goal
            "build-cache": 75.0,
            "version": 70.0,
        }

    def load_benchmark_data(self, benchmark_dir: Path) -> Dict:
        """Load benchmark data from directory"""
        data = {}

        for file_path in benchmark_dir.glob("*.json"):
            module_name = (
                file_path.stem.replace("-quick", "")
                .replace("-comprehensive", "")
                .replace("-validation", "")
            )

            try:
                with open(file_path) as f:
                    content = json.load(f)

                if module_name not in data:
                    data[module_name] = []

                # Extract performance metrics
                if isinstance(content, dict):
                    if "results" in content:
                        # Criterion format
                        for result in content["results"]:
                            mean_time = result.get("mean", {}).get("estimate", 0)
                            if mean_time > 0:
                                data[module_name].append(mean_time)
                    else:
                        # Custom format
                        for bench_name, bench_data in content.items():
                            if (
                                isinstance(bench_data, dict)
                                and "mean_time_ns" in bench_data
                            ):
                                data[module_name].append(bench_data["mean_time_ns"])

            except Exception as e:
                print(f"Warning: Failed to load {file_path}: {e}")

        return data

    def calculate_performance_score(self, times: List[float]) -> float:
        """Calculate performance score from benchmark times"""
        if not times:
            return 0.0

        # Convert nanoseconds to milliseconds for scoring
        avg_time_ms = statistics.mean(times) / 1_000_000

        # Performance score calculation (lower time = higher score)
        # Score ranges from 0-100, with diminishing returns for very fast operations
        if avg_time_ms <= 1.0:
            score = 100.0
        elif avg_time_ms <= 10.0:
            score = 90.0 - (avg_time_ms - 1.0) * 2.0  # 90-72
        elif avg_time_ms <= 100.0:
            score = 72.0 - (avg_time_ms - 10.0) * 0.5  # 72-27
        else:
            score = max(0.0, 27.0 - (avg_time_ms - 100.0) * 0.1)

        return min(100.0, max(0.0, score))

    def get_badge_color(self, score: float, target: float) -> str:
        """Get badge color based on performance score"""
        if score >= target:
            return "brightgreen"
        elif score >= target * 0.8:
            return "yellow"
        elif score >= target * 0.6:
            return "orange"
        else:
            return "red"

    def get_overall_status(self, module_scores: Dict[str, float]) -> str:
        """Get overall performance status"""
        if not module_scores:
            return "unknown"

        met_targets = 0
        total_modules = 0

        for module, score in module_scores.items():
            if module in self.performance_targets:
                total_modules += 1
                if score >= self.performance_targets[module]:
                    met_targets += 1

        if total_modules == 0:
            return "unknown"

        percentage = (met_targets / total_modules) * 100

        if percentage >= 80:
            return "excellent"
        elif percentage >= 60:
            return "good"
        elif percentage >= 40:
            return "fair"
        else:
            return "needs-improvement"

    def generate_badge_data(self, data: Dict) -> Dict:
        """Generate badge data from benchmark results"""
        module_scores = {}

        # Calculate scores for each module
        for module_name in self.modules:
            if module_name in data and data[module_name]:
                score = self.calculate_performance_score(data[module_name])
                module_scores[module_name] = score

        # Calculate overall score
        if module_scores:
            overall_score = statistics.mean(module_scores.values())
        else:
            overall_score = 0.0

        # Get overall status
        overall_status = self.get_overall_status(module_scores)

        # Generate badge configurations
        badges = {
            "overall": {
                "label": "performance",
                "message": f"{overall_score:.1f}/100",
                "color": self.get_badge_color(overall_score, 75.0),
                "style": "flat-square",
            }
        }

        # Individual module badges
        for module_name, score in module_scores.items():
            target = self.performance_targets.get(module_name, 70.0)
            badges[module_name] = {
                "label": f"{module_name}-perf",
                "message": f"{score:.1f}/100",
                "color": self.get_badge_color(score, target),
                "style": "flat-square",
            }

        # Status badge
        status_colors = {
            "excellent": "brightgreen",
            "good": "green",
            "fair": "yellow",
            "needs-improvement": "red",
            "unknown": "lightgrey",
        }

        badges["status"] = {
            "label": "status",
            "message": overall_status.replace("-", " "),
            "color": status_colors.get(overall_status, "lightgrey"),
            "style": "flat-square",
        }

        # Performance targets badge
        met_targets = sum(
            1
            for module, score in module_scores.items()
            if module in self.performance_targets
            and score >= self.performance_targets[module]
        )
        total_targets = len(
            [m for m in module_scores.keys() if m in self.performance_targets]
        )

        if total_targets > 0:
            targets_percentage = (met_targets / total_targets) * 100
            badges["targets"] = {
                "label": "targets met",
                "message": f"{met_targets}/{total_targets} ({targets_percentage:.0f}%)",
                "color": "brightgreen"
                if targets_percentage >= 80
                else "yellow"
                if targets_percentage >= 60
                else "red",
                "style": "flat-square",
            }

        return {
            "badges": badges,
            "scores": module_scores,
            "overall_score": overall_score,
            "overall_status": overall_status,
            "targets_met": f"{met_targets}/{total_targets}"
            if total_targets > 0
            else "0/0",
            "generated_at": "2024-01-01T00:00:00Z",  # Would use actual timestamp
        }

    def generate_shield_urls(self, badge_data: Dict) -> Dict[str, str]:
        """Generate shields.io URLs for badges"""
        base_url = "https://img.shields.io/badge"
        urls = {}

        for badge_name, badge_config in badge_data["badges"].items():
            label = badge_config["label"].replace(" ", "%20")
            message = badge_config["message"].replace(" ", "%20").replace("/", "%2F")
            color = badge_config["color"]
            style = badge_config.get("style", "flat-square")

            url = f"{base_url}/{label}-{message}-{color}?style={style}"
            urls[badge_name] = url

        return urls

    def generate_markdown_badges(self, badge_data: Dict) -> str:
        """Generate Markdown for badges"""
        urls = self.generate_shield_urls(badge_data)

        markdown = []
        markdown.append("<!-- Performance Badges -->")
        markdown.append(f"![Performance](${urls['overall']})")
        markdown.append(f"![Status](${urls['status']})")

        if "targets" in urls:
            markdown.append(f"![Targets](${urls['targets']})")

        markdown.append("")
        markdown.append("<!-- Module Performance Badges -->")

        for module in self.modules:
            if module in urls:
                module_title = module.replace("-", " ").title()
                markdown.append(f"![{module_title}](${urls[module]})")

        return "\n".join(markdown)


def main():
    parser = argparse.ArgumentParser(description="Generate performance badges")
    parser.add_argument(
        "--benchmark-dir",
        type=Path,
        required=True,
        help="Directory containing benchmark results",
    )
    parser.add_argument(
        "--output-file",
        type=Path,
        default="performance-badge.json",
        help="Output file for badge data",
    )
    parser.add_argument(
        "--markdown-file", type=Path, help="Output file for Markdown badges"
    )
    parser.add_argument("--verbose", action="store_true", help="Enable verbose output")

    args = parser.parse_args()

    if not args.benchmark_dir.exists():
        print(f"Error: Benchmark directory {args.benchmark_dir} does not exist")
        sys.exit(1)

    # Initialize generator
    generator = PerformanceBadgeGenerator()

    # Load benchmark data
    if args.verbose:
        print("Loading benchmark data...")
    data = generator.load_benchmark_data(args.benchmark_dir)

    if not data:
        print("Warning: No benchmark data found")
        # Create default badge data
        badge_data = {
            "badges": {
                "overall": {
                    "label": "performance",
                    "message": "unknown",
                    "color": "lightgrey",
                    "style": "flat-square",
                }
            },
            "scores": {},
            "overall_score": 0.0,
            "overall_status": "unknown",
            "targets_met": "0/0",
            "generated_at": "2024-01-01T00:00:00Z",
        }
    else:
        # Generate badge data
        if args.verbose:
            print("Generating badge data...")
        badge_data = generator.generate_badge_data(data)

    # Save badge data
    with open(args.output_file, "w") as f:
        json.dump(badge_data, f, indent=2)

    if args.verbose:
        print(f"Badge data saved to {args.output_file}")
        print(f"Overall score: {badge_data['overall_score']:.1f}/100")
        print(f"Status: {badge_data['overall_status']}")
        print(f"Targets met: {badge_data['targets_met']}")

    # Generate Markdown if requested
    if args.markdown_file:
        markdown = generator.generate_markdown_badges(badge_data)
        with open(args.markdown_file, "w") as f:
            f.write(markdown)

        if args.verbose:
            print(f"Markdown badges saved to {args.markdown_file}")


if __name__ == "__main__":
    main()
