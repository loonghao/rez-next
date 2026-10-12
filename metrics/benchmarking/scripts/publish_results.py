"""Publish a measured commit's saved results only while main still matches it.

Stale runs retain their workflow artifacts without changing main. A failed
publication can be retried independently of the expensive measurement job.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

REPORT = Path("metrics/benchmarking/RESULTS.md")
ARTIFACTS = Path("metrics/benchmarking/artifacts")


class PublishError(RuntimeError):
    """A real publication error, rather than an obsolete measurement."""


def git(*args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        ["git", *args],
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    if check and result.returncode:
        raise PublishError(f"git {args[0]} failed: {result.stderr.strip()}")
    return result


def current_main() -> str:
    git("fetch", "--no-tags", "origin", "main")
    return git("rev-parse", "FETCH_HEAD").stdout.strip()


def result_paths(measured_sha: str) -> list[str]:
    match = re.search(
        r"^<!-- Commit: ([0-9a-f]{7,40}) -->$",
        REPORT.read_text(encoding="utf-8"),
        re.MULTILINE,
    )
    if not match:
        raise PublishError("RESULTS.md is missing its measured commit marker")
    report_sha = match[1]
    if (
        git("rev-parse", "--verify", f"{report_sha}^{{commit}}").stdout.strip()
        != measured_sha
    ):
        raise PublishError("RESULTS.md describes a different measured commit")

    summaries = []
    for path in sorted(ARTIFACTS.glob("*/summary.json")):
        if not re.fullmatch(
            rf"\d{{4}}\.\d{{2}}\.\d{{2}}-{report_sha}", path.parent.name
        ):
            continue
        if json.loads(path.read_text(encoding="utf-8")).get("commit") != report_sha:
            raise PublishError(f"Measured commit mismatch in {path}")
        summaries.append(str(path))
    if not summaries:
        raise PublishError("No matching measured-commit summary was downloaded")
    return [str(REPORT), *summaries]


def publish(measured_sha: str) -> None:
    if not re.fullmatch(r"[0-9a-f]{40}", measured_sha):
        raise PublishError("--measured-sha must be a full commit SHA")
    if git("rev-parse", "HEAD").stdout.strip() != measured_sha:
        raise PublishError("Checkout HEAD does not match the measured commit")
    if git("diff", "--cached", "--quiet", check=False).returncode:
        raise PublishError("Refusing to publish with pre-existing staged changes")
    paths = result_paths(measured_sha)

    tip = current_main()
    if tip != measured_sha:
        print(f"Stale measurement {measured_sha}: main is {tip}; artifacts retained.")
        return

    git("add", "--", *paths)
    if git("diff", "--cached", "--quiet", check=False).returncode == 0:
        print(f"No result changes for measured commit {measured_sha}.")
        return
    git(
        "-c",
        "user.name=github-actions[bot]",
        "-c",
        "user.email=github-actions[bot]@users.noreply.github.com",
        "commit",
        "-m",
        f"chore(bench): update performance results [{measured_sha}] [skip ci]",
    )
    result = git("push", "origin", "HEAD:refs/heads/main", check=False)
    if result.returncode == 0:
        print(f"Published results measured at {measured_sha}.")
        return

    # A concurrent main update is expected and must never be overwritten or
    # rebased underneath results measured from an older tree.
    tip = current_main()
    if tip == git("rev-parse", "HEAD").stdout.strip():
        print(f"Confirmed publication of results measured at {measured_sha}.")
    elif tip != measured_sha:
        print(
            f"Publication became stale: main is {tip}; {measured_sha} artifacts retained."
        )
    else:
        raise PublishError(f"Result push failed: {result.stderr.strip()}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--measured-sha", required=True)
    args = parser.parse_args()
    try:
        publish(args.measured_sha)
    except (PublishError, OSError, ValueError) as exc:
        print(f"[ERROR] {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
