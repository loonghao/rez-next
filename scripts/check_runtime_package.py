#!/usr/bin/env python3
"""Check normalized SDK metadata, optionally testing public registry dependencies."""

from __future__ import annotations

import argparse
import json
import subprocess
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def validate_manifest(packaged: dict) -> None:
    if packaged.get("patch") or packaged.get("replace"):
        raise ValueError("Packaged SDK must use registry dependencies without replacements")
    scopes = [packaged, *packaged.get("target", {}).values()]
    for scope in scopes:
        for table in ("dependencies", "dev-dependencies", "build-dependencies"):
            for name, spec in scope.get(table, {}).items():
                if isinstance(spec, dict) and any(
                    key in spec for key in ("path", "git", "registry", "registry-index")
                ):
                    raise ValueError(f"Packaged dependency {name} must use crates.io")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--metadata-only", action="store_true",
        help="Validate a staged workspace package; public registry tests remain a separate gate",
    )
    args = parser.parse_args()
    with (ROOT / "rust-toolchain.toml").open("rb") as stream:
        channel = tomllib.load(stream)["toolchain"]["channel"]
    cargo = ["vx", "cargo", f"+{channel}"]
    metadata = json.loads(subprocess.check_output(
        [*cargo, "metadata", "--format-version=1", "--no-deps", "--locked"],
        cwd=ROOT,
        text=True,
    ))
    with (ROOT / "crates/rez-next-runtime/Cargo.toml").open("rb") as stream:
        version = tomllib.load(stream)["package"]["version"]
    manifest = Path(metadata["target_directory"]) / "package" / f"rez-next-runtime-{version}" / "Cargo.toml"
    with manifest.open("rb") as stream:
        packaged = tomllib.load(stream)
    validate_manifest(packaged)
    if args.metadata_only:
        return 0
    return subprocess.call(
        [*cargo, "test", "--manifest-path", str(manifest), "--tests", "--locked"],
        cwd=ROOT,
    )


if __name__ == "__main__":
    raise SystemExit(main())
