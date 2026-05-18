#! /usr/bin/env python
"""Offline vx-style Rez build fixture.

The production vx Rez package downloads a prebuilt artifact, verifies it,
extracts it, and moves the extracted payload into the Rez install root. This
fixture follows the same shape without network access: tests provide a local
zip path and sha256 via environment variables.
"""

import hashlib
import os
import shutil
import sys
import tempfile
import zipfile
from pathlib import Path


def _sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _copytree_contents(source, destination):
    destination.mkdir(parents=True, exist_ok=True)
    for child in source.iterdir():
        target = destination / child.name
        if child.is_dir():
            if target.exists():
                shutil.rmtree(target)
            shutil.copytree(child, target)
        else:
            shutil.copy2(child, target)


def _require_env(name):
    value = os.environ.get(name)
    if not value:
        raise RuntimeError(f"missing required build dependency environment variable: {name}")
    return value


def _assert_build_dependency_environment():
    _require_env("PYTHON_ROOT")
    _require_env("REZ_BUILDER_ROOT")
    if os.environ.get("PYTHON_SENTINEL") != "from-python-fixture":
        raise RuntimeError("python fixture environment was not applied")
    if os.environ.get("REZ_BUILDER_SENTINEL") != "from-rez-builder-fixture":
        raise RuntimeError("rez_builder fixture environment was not applied")


def build():
    _assert_build_dependency_environment()

    source_root = Path(__file__).resolve().parent
    artifact = os.environ.get("VX_REZ_TEST_ARTIFACT")
    expected_sha256 = os.environ.get("VX_REZ_TEST_SHA256")
    install_path = os.environ.get("REZ_BUILD_INSTALL_PATH")

    if not artifact:
        artifact = str(source_root / "dist" / "vx-artifact.zip")
    if not expected_sha256:
        checksum_file = source_root / "dist" / "vx-artifact.sha256"
        if not checksum_file.is_file():
            raise RuntimeError("VX_REZ_TEST_SHA256 or dist/vx-artifact.sha256 is required")
        expected_sha256 = checksum_file.read_text(encoding="utf-8").strip()
    if not install_path:
        raise RuntimeError("REZ_BUILD_INSTALL_PATH is required")

    artifact_path = Path(artifact)
    if not artifact_path.is_file():
        raise RuntimeError(f"artifact does not exist: {artifact_path}")

    actual_sha256 = _sha256(artifact_path)
    if actual_sha256 != expected_sha256:
        raise RuntimeError(
            f"artifact sha256 mismatch: expected {expected_sha256}, got {actual_sha256}"
        )

    install_root = Path(install_path)
    with tempfile.TemporaryDirectory(prefix="vx-rez-extract-") as temp_root:
        extract_root = Path(temp_root) / "extract"
        extract_root.mkdir()
        with zipfile.ZipFile(artifact_path) as archive:
            archive.extractall(extract_root)
        _copytree_contents(extract_root, install_root)

    print(f"installed vx artifact to {install_root}")


if __name__ == "__main__":
    try:
        build()
    except Exception as exc:
        print(f"vx rezbuild failed: {exc}", file=sys.stderr)
        sys.exit(1)
