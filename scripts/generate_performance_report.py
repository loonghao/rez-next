#!/usr/bin/env python3
"""
Performance Report Generator

Generates comprehensive performance reports from benchmark results in multiple formats.
"""

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Dict, List, Optional
import statistics
from datetime import datetime
import html

class PerformanceReportGenerator:
    """Generates performance reports from benchmark data"""
    
    def __init__(self):
        self.modules = [
            "version", "solver", "context", "rex", "build-cache"
        ]
    
    def load_benchmark_data(self, input_dir: Path) -> Dict:
        """Load all benchmark data from input directory"""
        data = {}
        
        for file_path in input_dir.glob("*.json"):
            module_name = file_path.stem.replace("-quick", "").replace("-comprehensive", "").replace("-validation", "")
            
            try:
                with open(file_path, 'r') as f:
                    content = json.load(f)
                
                if module_name not in data:
                    data[module_name] = {}
                
                # Determine benchmark type from filename
                if "quick" in file_path.stem:
                    data[module_name]["quick"] = content
                elif "comprehensive" in file_path.stem:
                    data[module_name]["comprehensive"] = content
                elif "validation" in file_path.stem:
                    data[module_name]["validation"] = content
                elif "regression" in file_path.stem:
                    data[module_name]["regression"] = content
                else:
                    data[module_name]["default"] = content
                    
            except Exception as e:
                print(f"Warning: Failed to load {file_path}: {e}")
        
        return data
    
    def load_baseline_data(self, baseline_dir: Path) -> Dict:
        """Load baseline data for comparison"""
        if not baseline_dir.exists():
            return {}
        
        return self.load_benchmark_data(baseline_dir)
    
    def calculate_summary_stats(self, data: Dict) -> Dict:
        """Calculate summary statistics across all modules"""
        stats = {
            "total_benchmarks": 0,
            "total_modules": len([k for k in data.keys() if k in self.modules]),
            "avg_performance_score": 0.0,
            "modules": {}
        }
        
        total_scores = []
        
        for module_name, module_data in data.items():
            if module_name not in self.modules:
                continue
            
            module_stats = {
                "benchmark_count": 0,
                "avg_time_ns": 0.0,
                "total_time_ns": 0.0,
                "performance_score": 0.0
            }
            
            times = []
            
            for benchmark_type, benchmarks in module_data.items():
                if isinstance(benchmarks, dict):
                    if "results" in benchmarks:
                        # Criterion format
                        for result in benchmarks["results"]:
                            mean_time = result.get("mean", {}).get("estimate", 0)
                            times.append(mean_time)
                            module_stats["benchmark_count"] += 1
                    else:
                        # Custom format
                        for bench_name, bench_data in benchmarks.items():
                            if isinstance(bench_data, dict) and "mean_time_ns" in bench_data:
                                times.append(bench_data["mean_time_ns"])
                                module_stats["benchmark_count"] += 1
            
            if times:
                module_stats["avg_time_ns"] = statistics.mean(times)
                module_stats["total_time_ns"] = sum(times)
                # Simple performance score (lower time = higher score)
                module_stats["performance_score"] = max(0, 100 - (module_stats["avg_time_ns"] / 1000000))  # Convert to ms
                total_scores.append(module_stats["performance_score"])
            
            stats["modules"][module_name] = module_stats
            stats["total_benchmarks"] += module_stats["benchmark_count"]
        
        if total_scores:
            stats["avg_performance_score"] = statistics.mean(total_scores)
        
        return stats
    
    def generate_markdown_report(self, data: Dict, baseline_data: Dict, stats: Dict) -> str:
        """Generate Markdown format report"""
        report = []
        
        # Header
        report.append("# rez-core Performance Report")
        report.append(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S UTC')}")
        report.append("")
        
        # Summary
        report.append("## Summary")
        report.append(f"- **Total Modules**: {stats['total_modules']}")
        report.append(f"- **Total Benchmarks**: {stats['total_benchmarks']}")
        report.append(f"- **Average Performance Score**: {stats['avg_performance_score']:.1f}/100")
        report.append("")
        
        # Module Details
        report.append("## Module Performance")
        report.append("")
        
        for module_name in self.modules:
            if module_name in stats["modules"]:
                module_stats = stats["modules"][module_name]
                report.append(f"### {module_name.title()} Module")
                report.append(f"- **Benchmarks**: {module_stats['benchmark_count']}")
                report.append(f"- **Average Time**: {module_stats['avg_time_ns']/1000000:.2f}ms")
                report.append(f"- **Performance Score**: {module_stats['performance_score']:.1f}/100")
                
                # Baseline comparison if available
                if baseline_data and module_name in baseline_data:
                    baseline_stats = self.calculate_module_baseline_comparison(
                        data.get(module_name, {}), 
                        baseline_data.get(module_name, {})
                    )
                    if baseline_stats:
                        change = baseline_stats.get("change_percent", 0)
                        if change > 5:
                            report.append(f"- **vs Baseline**: 🔴 {change:+.1f}% regression")
                        elif change < -5:
                            report.append(f"- **vs Baseline**: 🟢 {abs(change):.1f}% improvement")
                        else:
                            report.append(f"- **vs Baseline**: ⚪ {change:+.1f}% (no significant change)")
                
                report.append("")
        
        # Performance Targets
        report.append("## Performance Targets")
        report.append("")
        report.append("| Module | Target | Current Status |")
        report.append("|--------|--------|----------------|")
        
        targets = {
            "solver": "3-5x improvement for complex scenarios",
            "context": "Sub-millisecond creation for simple contexts",
            "rex": "75x performance improvement for complex operations",
            "build-cache": ">90% cache hit rate for repeated operations"
        }
        
        for module_name, target in targets.items():
            if module_name in stats["modules"]:
                score = stats["modules"][module_name]["performance_score"]
                status = "✅ Met" if score >= 70 else "⚠️ Needs Improvement" if score >= 50 else "❌ Below Target"
                report.append(f"| {module_name.title()} | {target} | {status} |")
        
        report.append("")
        
        # Recommendations
        report.append("## Recommendations")
        report.append("")
        
        low_performing = [name for name, stats in stats["modules"].items() 
                         if stats["performance_score"] < 70]
        
        if low_performing:
            report.append("### Performance Improvements Needed")
            for module in low_performing:
                score = stats["modules"][module]["performance_score"]
                report.append(f"- **{module.title()}**: Score {score:.1f}/100 - Consider optimization")
        else:
            report.append("### All Modules Performing Well")
            report.append("All modules are meeting performance targets.")
        
        return "\n".join(report)
    
    def calculate_module_baseline_comparison(self, current: Dict, baseline: Dict) -> Optional[Dict]:
        """Calculate comparison between current and baseline for a module"""
        # Simplified comparison - in practice, this would be more sophisticated
        current_times = []
        baseline_times = []
        
        # Extract times from both datasets
        for benchmark_type, benchmarks in current.items():
            if isinstance(benchmarks, dict) and "results" in benchmarks:
                for result in benchmarks["results"]:
                    current_times.append(result.get("mean", {}).get("estimate", 0))
        
        for benchmark_type, benchmarks in baseline.items():
            if isinstance(benchmarks, dict) and "results" in benchmarks:
                for result in benchmarks["results"]:
                    baseline_times.append(result.get("mean", {}).get("estimate", 0))
        
        if not current_times or not baseline_times:
            return None
        
        current_avg = statistics.mean(current_times)
        baseline_avg = statistics.mean(baseline_times)
        
        if baseline_avg == 0:
            return None
        
        change_percent = ((current_avg - baseline_avg) / baseline_avg) * 100
        
        return {
            "current_avg": current_avg,
            "baseline_avg": baseline_avg,
            "change_percent": change_percent
        }
    
    def generate_html_report(self, data: Dict, baseline_data: Dict, stats: Dict) -> str:
        """Generate HTML format report"""
        markdown_content = self.generate_markdown_report(data, baseline_data, stats)
        
        # Simple Markdown to HTML conversion
        html_content = markdown_content.replace("\n", "<br>\n")
        html_content = html_content.replace("# ", "<h1>").replace("</h1><br>", "</h1>")
        html_content = html_content.replace("## ", "<h2>").replace("</h2><br>", "</h2>")
        html_content = html_content.replace("### ", "<h3>").replace("</h3><br>", "</h3>")
        html_content = html_content.replace("- ", "<li>").replace("</li><br>", "</li>")
        
        # Wrap in basic HTML structure
        html_template = f"""
<!DOCTYPE html>
<html>
<head>
    <title>rez-core Performance Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; }}
        h1 {{ color: #2c3e50; }}
        h2 {{ color: #34495e; border-bottom: 2px solid #ecf0f1; }}
        h3 {{ color: #7f8c8d; }}
        table {{ border-collapse: collapse; width: 100%; }}
        th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; }}
        th {{ background-color: #f2f2f2; }}
        .score-high {{ color: #27ae60; }}
        .score-medium {{ color: #f39c12; }}
        .score-low {{ color: #e74c3c; }}
    </style>
</head>
<body>
    {html_content}
</body>
</html>
        """
        
        return html_template
    
    def generate_json_report(self, data: Dict, baseline_data: Dict, stats: Dict) -> Dict:
        """Generate JSON format report"""
        return {
            "metadata": {
                "generated_at": datetime.now().isoformat(),
                "generator": "rez-core-performance-reporter",
                "version": "1.0.0"
            },
            "summary": stats,
            "raw_data": data,
            "baseline_data": baseline_data
        }

def main():
    parser = argparse.ArgumentParser(description="Generate performance reports")
    parser.add_argument("--input-dir", type=Path, required=True,
                       help="Directory containing benchmark results")
    parser.add_argument("--output-dir", type=Path, required=True,
                       help="Directory to save reports")
    parser.add_argument("--baseline-dir", type=Path,
                       help="Directory containing baseline results")
    parser.add_argument("--format", default="html,json,markdown",
                       help="Output formats (comma-separated): html,json,markdown")
    
    args = parser.parse_args()
    
    if not args.input_dir.exists():
        print(f"Error: Input directory {args.input_dir} does not exist")
        sys.exit(1)
    
    args.output_dir.mkdir(parents=True, exist_ok=True)
    
    # Initialize generator
    generator = PerformanceReportGenerator()
    
    # Load data
    print("Loading benchmark data...")
    data = generator.load_benchmark_data(args.input_dir)
    
    baseline_data = {}
    if args.baseline_dir and args.baseline_dir.exists():
        print("Loading baseline data...")
        baseline_data = generator.load_baseline_data(args.baseline_dir)
    
    # Calculate statistics
    print("Calculating statistics...")
    stats = generator.calculate_summary_stats(data)
    
    # Generate reports
    formats = [f.strip().lower() for f in args.format.split(",")]
    
    if "markdown" in formats:
        print("Generating Markdown report...")
        markdown_report = generator.generate_markdown_report(data, baseline_data, stats)
        with open(args.output_dir / "summary.md", "w") as f:
            f.write(markdown_report)
    
    if "html" in formats:
        print("Generating HTML report...")
        html_report = generator.generate_html_report(data, baseline_data, stats)
        with open(args.output_dir / "report.html", "w") as f:
            f.write(html_report)
    
    if "json" in formats:
        print("Generating JSON report...")
        json_report = generator.generate_json_report(data, baseline_data, stats)
        with open(args.output_dir / "report.json", "w") as f:
            json.dump(json_report, f, indent=2)
    
    print(f"Reports generated in {args.output_dir}")
    print(f"Summary: {stats['total_benchmarks']} benchmarks across {stats['total_modules']} modules")
    print(f"Average performance score: {stats['avg_performance_score']:.1f}/100")

if __name__ == "__main__":
    main()
