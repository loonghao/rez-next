#!/usr/bin/env python3
"""Parse Criterion benchmark output and emit structured JSON.

Usage:
    cargo bench 2>&1 | python metrics/benchmarking/scripts/parse_criterion.py
    # or from file:
    python metrics/benchmarking/scripts/parse_criterion.py benchmark-quick.txt
    # opt into a non-zero exit when regressions are present:
    python metrics/benchmarking/scripts/parse_criterion.py --fail-on-regression benchmark-quick.txt
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional


# Criterion output line patterns. Grouped benchmarks print their name and timing
# interval on separate lines, while standalone benchmarks use a single line.
_TIME_INTERVAL = (
    r"time:\s+\[(?P<low>[\d.]+)\s*(?P<low_unit>ps|ns|µs|us|ms|s)\s+"
    r"(?P<mean>[\d.]+)\s*(?P<mean_unit>ps|ns|µs|us|ms|s)\s+"
    r"(?P<high>[\d.]+)\s*(?P<high_unit>ps|ns|µs|us|ms|s)\]"
)
_TIME_RE = re.compile(rf"^(?P<name>.+?)\s+{_TIME_INTERVAL}")
_TIME_ONLY_RE = re.compile(rf"^\s*{_TIME_INTERVAL}")
_CHANGE_RE = re.compile(
    r"^\s+change:\s+\[(?P<low_pct>[+\-−][\d.]+)%\s+"
    r"(?P<mean_pct>[+\-−][\d.]+)%\s+"
    r"(?P<high_pct>[+\-−][\d.]+)%\]\s+\(p\s+=\s+(?P<p>[\d.]+)"
)
_REGRESSION_RE = re.compile(r"Performance has regressed", re.IGNORECASE)
_IMPROVED_RE = re.compile(r"Performance has improved", re.IGNORECASE)
_ANSI_RE = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]")


def _to_ns(value: float, unit: str) -> float:
    """Normalize any time unit to nanoseconds."""
    return {
        "ps": value / 1_000,
        "ns": value,
        "µs": value * 1_000,
        "us": value * 1_000,
        "ms": value * 1_000_000,
        "s": value * 1_000_000_000,
    }.get(unit, value)


def _fmt_ns(ns: float) -> str:
    """Human-readable from nanoseconds."""
    if ns < 1_000:
        return f"{ns:.1f} ns"
    if ns < 1_000_000:
        return f"{ns / 1_000:.3f} µs"
    if ns < 1_000_000_000:
        return f"{ns / 1_000_000:.3f} ms"
    return f"{ns / 1_000_000_000:.3f} s"


@dataclass
class BenchResult:
    name: str
    mean_ns: float
    low_ns: float
    high_ns: float
    mean_human: str
    change_pct: Optional[float] = None
    regression: bool = False
    improved: bool = False

    def as_dict(self) -> dict:
        return asdict(self)


@dataclass
class BenchSuite:
    timestamp: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    results: list[BenchResult] = field(default_factory=list)
    regressions: list[str] = field(default_factory=list)

    def as_dict(self) -> dict:
        return {
            "timestamp": self.timestamp,
            "results": [r.as_dict() for r in self.results],
            "regressions": self.regressions,
            "summary": {
                "total": len(self.results),
                "regressions": len(self.regressions),
            },
        }


def parse(lines: list[str]) -> BenchSuite:
    suite = BenchSuite()
    pending: Optional[BenchResult] = None
    candidate_name: Optional[str] = None

    for raw_line in lines:
        line = _ANSI_RE.sub("", raw_line).rstrip()
        stripped = line.strip()
        m = _TIME_RE.match(stripped)
        name = m["name"].strip() if m else None
        if not m and candidate_name:
            m = _TIME_ONLY_RE.match(line)
            name = candidate_name if m else None

        if m:
            # Flush previous
            if pending:
                suite.results.append(pending)

            mean_ns = _to_ns(float(m["mean"]), m["mean_unit"])
            low_ns = _to_ns(float(m["low"]), m["low_unit"])
            high_ns = _to_ns(float(m["high"]), m["high_unit"])
            pending = BenchResult(
                name=name or "unknown",
                mean_ns=mean_ns,
                low_ns=low_ns,
                high_ns=high_ns,
                mean_human=_fmt_ns(mean_ns),
            )
            candidate_name = None
            continue

        if pending:
            c = _CHANGE_RE.match(line)
            if c:
                pending.change_pct = float(c["mean_pct"].replace("−", "-"))
                continue
            if _REGRESSION_RE.search(line):
                if not pending.regression:
                    pending.regression = True
                    suite.regressions.append(pending.name)
                continue
            if _IMPROVED_RE.search(line):
                pending.improved = True
                continue

        if stripped and not line[:1].isspace():
            candidate_name = stripped

    if pending:
        suite.results.append(pending)

    return suite


def main(argv: Optional[list[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", nargs="?", type=Path, help="Criterion output file")
    parser.add_argument(
        "--fail-on-regression",
        action="store_true",
        help="Return a non-zero exit code when regressions are detected",
    )
    args = parser.parse_args(argv)

    if args.input:
        text = args.input.read_text(encoding="utf-8")
    else:
        text = sys.stdin.read()

    suite = parse(text.splitlines())
    print(json.dumps(suite.as_dict(), indent=2, ensure_ascii=False))

    if suite.regressions:
        sys.stderr.write(
            f"\n[WARN] {len(suite.regressions)} regression(s): {suite.regressions}\n"
        )
        if args.fail_on_regression:
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
