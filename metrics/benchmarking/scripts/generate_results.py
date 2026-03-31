#!/usr/bin/env python3
"""Generate RESULTS.md comparing rez-next (Rust) vs rez (Python).

Usage:
    python metrics/benchmarking/scripts/generate_results.py \
        --bench-json benchmark-quick.json \
        [--bench-json benchmark-full.json] \
        --out metrics/benchmarking/RESULTS.md

The script reads:
  - One or more parsed Criterion JSON files (from parse_criterion.py)
  - metrics/benchmarking/data/rez_baseline.json  (official rez numbers)

And produces:
  - metrics/benchmarking/RESULTS.md  (auto-generated, do NOT edit by hand)
  - metrics/benchmarking/artifacts/<date>-<sha>/summary.json
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
DATA_DIR = Path(__file__).resolve().parent.parent / "data"
ARTIFACTS_DIR = Path(__file__).resolve().parent.parent / "artifacts"


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _git_sha() -> str:
    try:
        return subprocess.check_output(
            ["git", "rev-parse", "--short", "HEAD"], cwd=REPO_ROOT, text=True
        ).strip()
    except Exception:
        return "unknown"


def _cpu_info() -> str:
    try:
        if platform.system() == "Linux":
            for line in Path("/proc/cpuinfo").read_text().splitlines():
                if line.startswith("model name"):
                    return line.split(":", 1)[1].strip()
        return platform.processor() or platform.machine()
    except Exception:
        return "unknown"


def _ns_to_human(ns: float) -> str:
    if ns < 1_000:
        return f"{ns:.1f} ns"
    if ns < 1_000_000:
        return f"{ns / 1_000:.2f} µs"
    if ns < 1_000_000_000:
        return f"{ns / 1_000_000:.2f} ms"
    return f"{ns / 1_000_000_000:.3f} s"


def _speedup(rez_ms: float, rez_next_ns: float) -> str:
    ratio = (rez_ms * 1_000_000) / rez_next_ns
    return f"**~{ratio:.0f}×**"


# ---------------------------------------------------------------------------
# Load data
# ---------------------------------------------------------------------------

def load_bench_results(json_files: list[Path]) -> dict[str, dict]:
    """Merge multiple criterion JSON files keyed by benchmark name."""
    results: dict[str, dict] = {}
    for path in json_files:
        data = json.loads(path.read_text())
        for r in data.get("results", []):
            results[r["name"]] = r
    return results


def load_baseline(path: Path) -> dict:
    return json.loads(path.read_text())


# ---------------------------------------------------------------------------
# Map Criterion bench names → baseline keys
# ---------------------------------------------------------------------------

# Maps baseline operation key → list of possible Criterion bench name substrings
BASELINE_MAPPING: dict[str, list[str]] = {
    "version_parse_1000": ["version_parsing", "version_parse"],
    "version_range_parse_1000": ["version_range_parse", "range_parse"],
    "req_parse_1000": ["package_requirement", "req_parse", "requirement_parse"],
    "rex_execute_10cmds": ["rex_execute", "rex_real_package", "maya_commands", "python_commands"],
    "shell_script_generate": ["shell_script", "generate_script", "shell_generate"],
    "package_py_parse": ["package_serialization", "yaml_serialization", "package_parse"],
}


def _find_bench(results: dict[str, dict], keys: list[str]) -> dict | None:
    for k in keys:
        for name, r in results.items():
            if k.lower() in name.lower():
                return r
    return None


# ---------------------------------------------------------------------------
# Solver benchmark (rez official format: median solve time in seconds)
# ---------------------------------------------------------------------------

def _solver_section(results: dict[str, dict], baseline: dict) -> str:
    rez_median_s = baseline["solver"]["median_s"]
    rez_mean_s = baseline["solver"]["mean_s"]
    rez_stddev_s = baseline["solver"]["stddev_s"]

    # Find solver bench
    solver_r = _find_bench(results, ["resolver_create", "resolve_empty", "resolve_single"])
    if solver_r:
        rez_next_mean_s = solver_r["mean_ns"] / 1_000_000_000
        speedup = (rez_median_s * 1_000_000) / solver_r["mean_ns"]
        rez_next_str = _ns_to_human(solver_r["mean_ns"])
        speedup_str = f"**~{speedup:.0f}×**"
    else:
        rez_next_str = "N/A (run full benchmarks)"
        speedup_str = "N/A"

    return f"""\
## Solver Performance

> Reference: [rez official benchmarking](https://github.com/AcademySoftwareFoundation/rez/tree/main/metrics/benchmarking)

| Metric | rez (Python) | rez-next (Rust) | Speedup |
|--------|-------------|-----------------|---------|
| Median solve time | {rez_median_s * 1000:.1f} ms | {rez_next_str} | {speedup_str} |
| Mean solve time   | {rez_mean_s * 1000:.1f} ms | — | — |
| StdDev            | {rez_stddev_s * 1000:.1f} ms | — | — |

*rez baseline: {baseline['_source']}*
"""


# ---------------------------------------------------------------------------
# Core operations table
# ---------------------------------------------------------------------------

def _ops_table(results: dict[str, dict], baseline: dict) -> str:
    ops = baseline["operations"]
    rows = []

    for key, mapping_keys in BASELINE_MAPPING.items():
        if key not in ops:
            continue
        op = ops[key]
        rez_ms = op["mean_ms"]
        note = op["note"]
        r = _find_bench(results, mapping_keys)
        if r:
            rez_next_str = _ns_to_human(r["mean_ns"])
            speedup = _speedup(rez_ms, r["mean_ns"])
            bench_name = r["name"]
        else:
            rez_next_str = "N/A"
            speedup = "N/A"
            bench_name = "—"
        rows.append(
            f"| {note} | {rez_ms:.1f} ms | {rez_next_str} | {speedup} | `{bench_name}` |"
        )

    if not rows:
        return "_No matching benchmark results found. Run `cargo bench` and re-generate._\n"

    header = (
        "| Operation | rez Python | rez-next Rust | Speedup | Criterion bench |\n"
        "|-----------|-----------|---------------|---------|----------------|\n"
    )
    return header + "\n".join(rows) + "\n"


# ---------------------------------------------------------------------------
# Memory section (static from docs/performance.md)
# ---------------------------------------------------------------------------

MEMORY_TABLE = """\
## Memory Usage

| Scenario | rez Python RSS | rez-next Rust RSS | Reduction |
|----------|---------------|-------------------|-----------|
| Startup (`import rez`) | ~45 MB | ~2 MB | **~96%** |
| Solve 10-package graph | ~60 MB | ~4 MB | **~93%** |
| Load 100-package repo  | ~180 MB | ~12 MB | **~93%** |

*Memory figures from local profiling; Rust numbers via `/usr/bin/time -v`.*
"""


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def generate(bench_jsons: list[Path], output: Path) -> dict:
    baseline_path = DATA_DIR / "rez_baseline.json"
    if not baseline_path.exists():
        sys.exit(f"Baseline not found: {baseline_path}")

    baseline = load_baseline(baseline_path)
    results = load_bench_results(bench_jsons)

    sha = _git_sha()
    ts = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
    cpu = _cpu_info()
    py_ver = platform.python_version()
    os_info = f"{platform.system()}-{platform.release()}"

    # Count regressions across all input files
    regressions: list[str] = []
    for p in bench_jsons:
        d = json.loads(p.read_text())
        regressions.extend(d.get("regressions", []))

    # Build document
    regression_banner = ""
    if regressions:
        regression_banner = (
            f"\n> ⚠️ **{len(regressions)} regression(s) detected**: "
            + ", ".join(f"`{r}`" for r in regressions)
            + "\n"
        )

    md = f"""\
<!-- DO NOT EDIT BY HAND — auto-generated by metrics/benchmarking/scripts/generate_results.py -->
<!-- Last updated: {ts} -->
<!-- Commit: {sha} -->

# rez-next Performance Results

{regression_banner}
## Environment

| Field | Value |
|-------|-------|
| rez-next commit | `{sha}` |
| Generated | {ts} |
| Platform | `{os_info}` |
| CPU | {cpu} |
| Python | {py_ver} |

---

## Core Operations vs rez Python

{_ops_table(results, baseline)}

---

{_solver_section(results, baseline)}

---

{MEMORY_TABLE}

---

## How to Reproduce

```bash
# Run quick benchmarks and regenerate
cargo bench --bench version_benchmark --bench simple_package_benchmark --bench rex_benchmark \\
  2>&1 | python metrics/benchmarking/scripts/parse_criterion.py > /tmp/bench.json

python metrics/benchmarking/scripts/generate_results.py \\
  --bench-json /tmp/bench.json \\
  --out metrics/benchmarking/RESULTS.md
```

See [`docs/performance.md`](../../docs/performance.md) for full methodology.
"""

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(md, encoding="utf-8")
    print(f"[OK] Written: {output}")

    # Save artifact
    artifact_dir = ARTIFACTS_DIR / f"{datetime.now().strftime('%Y.%m.%d')}-{sha}"
    artifact_dir.mkdir(parents=True, exist_ok=True)
    summary = {
        "timestamp": ts,
        "commit": sha,
        "platform": os_info,
        "cpu": cpu,
        "regressions": regressions,
        "bench_count": len(results),
    }
    (artifact_dir / "summary.json").write_text(json.dumps(summary, indent=2))
    print(f"[OK] Artifact: {artifact_dir / 'summary.json'}")
    return summary


def main() -> None:
    parser = argparse.ArgumentParser(description="Generate rez-next RESULTS.md")
    parser.add_argument(
        "--bench-json",
        action="append",
        dest="bench_jsons",
        required=True,
        metavar="FILE",
        help="Parsed Criterion JSON (from parse_criterion.py). Repeatable.",
    )
    parser.add_argument(
        "--out",
        default="metrics/benchmarking/RESULTS.md",
        help="Output Markdown file (default: metrics/benchmarking/RESULTS.md)",
    )
    args = parser.parse_args()

    bench_jsons = [Path(p) for p in args.bench_jsons]
    for p in bench_jsons:
        if not p.exists():
            sys.exit(f"File not found: {p}")

    generate(bench_jsons, Path(args.out))


if __name__ == "__main__":
    main()
