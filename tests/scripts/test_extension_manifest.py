"""Regression guards for `vx-extension.toml`.

The most dangerous failure this file can cause is invisible: declaring
`[[entrypoint.arguments]]` makes vx parse bare-entrypoint flags into
`VX_ARG_*` environment variables instead of forwarding argv. The script then
sees an empty argv, so every argparse flag falls back to its default and
`vx x rez-release --dry-run` silently becomes a real release that builds,
installs, and creates a VCS tag.

Keep these guards in mind before "helpfully" re-adding argument declarations.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]
MANIFEST = REPO_ROOT / "vx-extension.toml"

if sys.version_info >= (3, 11):
    import tomllib
else:
    import tomli as tomllib


def load_manifest() -> dict:
    assert MANIFEST.is_file(), f"missing extension manifest: {MANIFEST}"
    with MANIFEST.open("rb") as handle:
        return tomllib.load(handle)


@pytest.fixture(scope="module")
def manifest() -> dict:
    return load_manifest()


def test_entrypoint_declares_no_arguments(manifest: dict) -> None:
    entrypoint = manifest["extension"]
    assert entrypoint["name"] == "rez-release"
    assert "arguments" not in manifest.get("entrypoint", {}), (
        "declaring [[entrypoint.arguments]] stops vx from forwarding argv to the "
        "bare entrypoint, which turns `--dry-run` into a real release"
    )


def test_no_command_declares_arguments(manifest: dict) -> None:
    for name, command in manifest.get("commands", {}).items():
        assert "arguments" not in command, (
            f"command {name!r} must not declare arguments: vx would parse them "
            "into VX_ARG_* env vars instead of forwarding argv"
        )


def test_manifest_text_has_no_argument_tables() -> None:
    """No argument tables in the live TOML (comments explaining why are fine)."""
    lines = MANIFEST.read_text(encoding="utf-8").splitlines()
    active = [line for line in lines if not line.lstrip().startswith("#")]
    for line in active:
        assert not line.lstrip().startswith("[["), (
            f"unexpected table array on a live line: {line!r}. Argument tables "
            "stop vx from forwarding argv to the entrypoint."
        )


def test_manifest_retains_required_shape(manifest: dict) -> None:
    assert manifest["entrypoint"]["main"] == "scripts/release.py"
    assert set(manifest["commands"]) == {"release", "check"}
    assert manifest["commands"]["check"]["args"] == ["--dry-run"]
