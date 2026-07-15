#!/usr/bin/env python3
"""Validate every release-managed version against the release manifest."""

from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MARKER = "x-release-please-version"
VERSION_PATTERN = re.compile(r"(?<!\d)\d+\.\d+\.\d+(?!\d)")


def release_files() -> list[Path]:
    config = json.loads((ROOT / "release-please-config.json").read_text(encoding="utf-8"))
    entries = config["packages"]["."]["extra-files"]
    return [ROOT / (entry["path"] if isinstance(entry, dict) else entry) for entry in entries]


def marked_versions(path: Path) -> list[tuple[int, str]]:
    versions = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if MARKER not in line:
            continue
        versions_on_line = VERSION_PATTERN.findall(line[: line.index(MARKER)])
        if not versions_on_line:
            raise ValueError(f"{path.relative_to(ROOT)}:{line_number}: marker has no version")
        versions.append((line_number, versions_on_line[-1]))
    return versions


def dependency_tables(manifest: dict):
    for name in ("dependencies", "dev-dependencies", "build-dependencies"):
        yield manifest.get(name, {})
    for target in manifest.get("target", {}).values():
        for name in ("dependencies", "dev-dependencies", "build-dependencies"):
            yield target.get(name, {})


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--expected", help="Expected version or release tag")
    args = parser.parse_args()

    manifest = json.loads(
        (ROOT / ".release-please-manifest.json").read_text(encoding="utf-8")
    )["."]
    expected = args.expected.removeprefix("v") if args.expected else manifest
    errors = []

    if manifest != expected:
        errors.append(f"release manifest is {manifest}, expected {expected}")

    with (ROOT / "Cargo.toml").open("rb") as stream:
        root_manifest = tomllib.load(stream)
    cargo_version = root_manifest["package"].get("version")
    if not isinstance(cargo_version, str):
        errors.append(
            "root Cargo package.version must be a literal string for release-please"
        )
    elif cargo_version != expected:
        errors.append(f"root Cargo package is {cargo_version!r}, expected {expected}")

    with (ROOT / "crates/rez-next-python/pyproject.toml").open("rb") as stream:
        python_version = tomllib.load(stream)["project"]["version"]
    if python_version != expected:
        errors.append(f"Python package is {python_version}, expected {expected}")

    if json.loads((ROOT / "release-please-config.json").read_text(encoding="utf-8"))[
        "packages"
    ]["."]["release-type"] != "rust":
        errors.append("release-please must use the rust strategy so Cargo.lock is updated")

    manifest_paths = [ROOT / "Cargo.toml"] + [
        ROOT / member / "Cargo.toml" for member in root_manifest["workspace"]["members"]
    ]
    workspace_manifests = {path.resolve() for path in manifest_paths}
    workspace_names = set()
    parsed_manifests = []
    for path in manifest_paths:
        with path.open("rb") as stream:
            manifest_data = tomllib.load(stream)
        parsed_manifests.append((path, manifest_data))
        package = manifest_data.get("package")
        if not package:
            continue
        workspace_names.add(package["name"])
        version = package.get("version")
        if not isinstance(version, str):
            errors.append(
                f"{path.relative_to(ROOT)} package.version must be a literal string "
                "for release-please"
            )
        elif version != expected:
            errors.append(
                f"{path.relative_to(ROOT)} package is {version!r}, expected {expected}"
            )

    for path, manifest_data in parsed_manifests:
        for dependencies in dependency_tables(manifest_data):
            for name, spec in dependencies.items():
                if not isinstance(spec, dict) or "path" not in spec:
                    continue
                dependency_manifest = (path.parent / spec["path"] / "Cargo.toml").resolve()
                if dependency_manifest not in workspace_manifests:
                    continue
                if spec.get("version") != expected:
                    errors.append(
                        f"{path.relative_to(ROOT)} dependency {name} has version "
                        f"{spec.get('version')!r}, expected {expected}"
                    )

    with (ROOT / "Cargo.lock").open("rb") as stream:
        lockfile = tomllib.load(stream)
    locked_workspace = {
        package["name"]: package["version"]
        for package in lockfile["package"]
        if package["name"] in workspace_names
    }
    for name in sorted(workspace_names):
        if locked_workspace.get(name) != expected:
            errors.append(
                f"Cargo.lock package {name} is {locked_workspace.get(name)!r}, expected {expected}"
            )

    for path in release_files():
        if not path.is_file():
            errors.append(f"release-managed file is missing: {path.relative_to(ROOT)}")
            continue
        versions = marked_versions(path)
        if not versions:
            errors.append(f"release-managed file has no {MARKER} marker: {path.relative_to(ROOT)}")
        for line_number, version in versions:
            if version != expected:
                errors.append(
                    f"{path.relative_to(ROOT)}:{line_number} is {version}, expected {expected}"
                )

    if errors:
        print("Release version validation failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print(f"Release versions are consistent: {expected}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
