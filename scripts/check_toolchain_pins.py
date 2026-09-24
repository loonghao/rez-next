#!/usr/bin/env python3
"""Verify that the Rust toolchain pin is identical in every location.

`rust-toolchain.toml` is the single source of truth. Everywhere else the same
version appears - the declared MSRV in `Cargo.toml`, the `msrv` used by clippy,
and every `dtolnay/rust-toolchain@<version>` / `RUSTUP_TOOLCHAIN: <version>`
entry in the GitHub Actions workflows - it has to match.

The check exists because the pins were allowed to drift: `rust-toolchain.toml`
was bumped to 1.97.1 while CI still pinned 1.95.0, and vx additionally
installed the `stable` channel at runtime, so CI jobs silently ran on a
toolchain nobody had pinned.
"""

from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# `rustsec/audit-check` needs current advisory tooling, so the Security Audit
# job deliberately opts out of the pinned toolchain.
NON_VERSION_ENV_VALUES = {"stable"}

TOOLCHAIN_RE = re.compile(r"uses:\s*dtolnay/rust-toolchain@(?P<version>\S+)")
RUSTUP_ENV_RE = re.compile(r"RUSTUP_TOOLCHAIN:\s*(?P<version>\S+)")
VERSION_RE = re.compile(r"^\d+\.\d+\.\d+$")


def workflow_pins() -> list[tuple[str, int, str, str]]:
    """Return (kind, line, value, location) for every pin in the workflows."""
    found = []
    for path in sorted((ROOT / ".github" / "workflows").glob("*.yml")):
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            for kind, pattern in (("dtolnay/rust-toolchain@", TOOLCHAIN_RE), ("RUSTUP_TOOLCHAIN", RUSTUP_ENV_RE)):
                match = pattern.search(line)
                if match:
                    found.append((kind, number, match.group("version"), path.relative_to(ROOT).as_posix()))
    return found


def main() -> int:
    with (ROOT / "rust-toolchain.toml").open("rb") as stream:
        expected = tomllib.load(stream)["toolchain"]["channel"]
    with (ROOT / "Cargo.toml").open("rb") as stream:
        cargo = tomllib.load(stream)
    with (ROOT / "clippy.toml").open("rb") as stream:
        clippy = tomllib.load(stream)
    with (ROOT / "vx.toml").open("rb") as stream:
        vx = tomllib.load(stream)

    errors: list[str] = []
    checked: list[str] = []

    def compare(label: str, actual: str | None) -> None:
        checked.append(f"{label}: {actual}")
        if actual != expected:
            errors.append(f"{label} is {actual!r}, expected {expected!r} from rust-toolchain.toml")

    compare("rust-toolchain.toml [toolchain].channel", expected)
    compare("Cargo.toml [workspace.package].rust-version", cargo.get("workspace", {}).get("package", {}).get("rust-version"))
    compare("clippy.toml msrv", clippy.get("msrv"))

    for kind, number, value, location in workflow_pins():
        label = f"{location}:{number} {kind}"
        if not VERSION_RE.match(value):
            if value in NON_VERSION_ENV_VALUES:
                checked.append(f"{label}: {value} (intentional override, not checked)")
            else:
                errors.append(f"{label}: {value!r} is not an explicit version pin")
            continue
        compare(label, value)

    vx_rust = vx.get("tools", {}).get("rust")
    if vx_rust is not None:
        errors.append(
            f"vx.toml [tools].rust is pinned to {vx_rust!r}; remove it so rustup and "
            "rust-toolchain.toml stay the only source of truth for the Rust toolchain"
        )
    else:
        checked.append("vx.toml [tools].rust: not pinned (rust comes from rustup)")

    for line in checked:
        print(f"ok   {line}")

    if errors:
        print("\nToolchain pins are inconsistent:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        print(
            "\nAlign every location on the value in rust-toolchain.toml, then re-run this check.",
            file=sys.stderr,
        )
        return 1

    print(f"\nAll Rust toolchain pins agree on {expected}.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
