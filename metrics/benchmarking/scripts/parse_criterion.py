#!/usr/bin/env python3
"""Parse Criterion benchmark output and emit structured JSON.

Usage:
    cargo bench 2>&1 | python metrics/benchmarking/scripts/parse_criterion.py
    # or from file:
    python metrics/benchmarking/scripts/parse_criterion.py benchmark-quick.txt
"""

from __future__ import annotations

import json
import re
import sys
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional


# Criterion output line patterns
# Example: "version_parsing    time:   [9.088 µs 9.120 µs 9.157 µs]"
# Example: "version_parsing    change: [-0.44% +0.71% +1.68%] (p = 0.21 > 0.05)"
_TIME_RE = re.compile(
    r"^(?P<name>.+?)\s+time:\s+\[(?P<low>[\d.]+)\s*(?P<low_unit>[nµm]?s)\s+"
    r"(?P<mean>[\d.]+)\s*(?P<mean_unit>[nµm]?s)\s+"
    r"(?P<high>[\d.]+)\s*(?P<high_unit>[nµm]?s)\]"
)
_CHANGE_RE = re.compile(
    r"^\s+change:\s+\[(?P<low_pct>[+-][\d.]+)%\s+(?P<mean_pct>[+-][\d.]+)%\s+"
    r"(?P<high_pct>[+-][\d.]+)%\]\s+\(p\s+=\s+(?P<p>[\d.]+)"
)
_REGRESSION_RE = re.compile(r"Performance has regressed", re.IGNORECASE)
_IMPROVED_RE = re.compile(r"Performance has improved", re.IGNORECASE)


def _to_ns(value: float, unit: str) -> float:
    """Normalize any time unit to nanoseconds."""
    return {
        "ns": value,
        "µs": value * 1_000,
        "ms": value * 1_000_000,
        "s":  value * 1_000_000_000,
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

    for line in lines:
        m = _TIME_RE.match(line.strip())
        if m:
            # Flush previous
            if pending:
                suite.results.append(pending)

            mean_ns = _to_ns(float(m["mean"]), m["mean_unit"])
            low_ns = _to_ns(float(m["low"]), m["low_unit"])
            high_ns = _to_ns(float(m["high"]), m["high_unit"])
            pending = BenchResult(
                name=m["name"].strip(),
                mean_ns=mean_ns,
                low_ns=low_ns,
                high_ns=high_ns,
                mean_human=_fmt_ns(mean_ns),
            )
            continue

        if pending:
            c = _CHANGE_RE.match(line)
            if c:
                pending.change_pct = float(c["mean_pct"])
                continue
            if _REGRESSION_RE.search(line):
                pending.regression = True
                suite.regressions.append(pending.name)
                continue
            if _IMPROVED_RE.search(line):
                pending.improved = True
                continue

    if pending:
        suite.results.append(pending)

    return suite


def main() -> None:
    if len(sys.argv) > 1:
        text = Path(sys.argv[1]).read_text(encoding="utf-8")
    else:
        text = sys.stdin.read()

    suite = parse(text.splitlines())
    print(json.dumps(suite.as_dict(), indent=2, ensure_ascii=False))

    # Exit non-zero if regressions detected (useful in CI)
    if suite.regressions:
        sys.stderr.write(
            f"\n[WARN] {len(suite.regressions)} regression(s): {suite.regressions}\n"
        )
        sys.exit(1)


if __name__ == "__main__":
    main()
