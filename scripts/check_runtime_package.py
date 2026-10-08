#!/usr/bin/env python3
"""Run SDK contracts from the packaged crate against registry dependencies."""

from __future__ import annotations

import json
import subprocess
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
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
    for table in ("dependencies", "dev-dependencies", "build-dependencies"):
        for name, spec in packaged.get(table, {}).items():
            if isinstance(spec, dict) and "path" in spec:
                raise ValueError(f"Packaged dependency {name} still uses a local path")
    if packaged.get("patch") or packaged.get("replace"):
        raise ValueError("Packaged SDK must use registry dependencies without replacements")
    return subprocess.call(
        [*cargo, "test", "--manifest-path", str(manifest), "--test", "runtime_api", "--locked"],
        cwd=ROOT,
    )


if __name__ == "__main__":
    raise SystemExit(main())
