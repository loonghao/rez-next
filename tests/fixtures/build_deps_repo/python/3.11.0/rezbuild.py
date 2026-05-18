#! /usr/bin/env python
"""Build a Rez Python package from a python-build-standalone artifact.

Tests pass a local artifact path via PYTHON_STANDALONE_ARTIFACT. Outside tests,
this script can download an artifact URL selected for the current platform or
provided via PYTHON_STANDALONE_URL.
"""

import hashlib
import os
import pathlib
import shutil
import sys
import tarfile
import tempfile
import urllib.request
import zipfile


DEFAULT_RELEASE = "20260510"
DEFAULT_VERSION = "3.11.14"


def _platform_tag():
    if sys.platform.startswith("win"):
        return "x86_64-pc-windows-msvc"
    if sys.platform == "darwin":
        return "aarch64-apple-darwin" if os.uname().machine == "arm64" else "x86_64-apple-darwin"
    return "x86_64-unknown-linux-gnu"


def _default_url():
    release = os.environ.get("PYTHON_STANDALONE_RELEASE", DEFAULT_RELEASE)
    version = os.environ.get("PYTHON_STANDALONE_VERSION", DEFAULT_VERSION)
    filename = (
        f"cpython-{version}+{release}-{_platform_tag()}-install_only_stripped.tar.gz"
    )
    return (
        "https://github.com/astral-sh/python-build-standalone/releases/download/"
        f"{release}/{filename}"
    )


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


def _download(url, destination):
    with urllib.request.urlopen(url) as response:
        with open(destination, "wb") as stream:
            shutil.copyfileobj(response, stream)


def _artifact_path(work_dir):
    local_artifact = os.environ.get("PYTHON_STANDALONE_ARTIFACT")
    if local_artifact:
        path = pathlib.Path(local_artifact)
        if not path.is_file():
            raise RuntimeError(f"python standalone artifact does not exist: {path}")
        return path

    url = os.environ.get("PYTHON_STANDALONE_URL", _default_url())
    destination = work_dir / pathlib.PurePosixPath(url).name
    print(f"downloading python standalone artifact: {url}")
    _download(url, destination)
    return destination


def _extract(artifact, destination):
    if zipfile.is_zipfile(artifact):
        with zipfile.ZipFile(artifact) as archive:
            archive.extractall(destination)
        return

    if tarfile.is_tarfile(artifact):
        with tarfile.open(artifact) as archive:
            archive.extractall(destination)
        return

    raise RuntimeError(f"unsupported python standalone artifact: {artifact}")


def build():
    install_path = os.environ.get("REZ_BUILD_INSTALL_PATH")
    if not install_path:
        raise RuntimeError("REZ_BUILD_INSTALL_PATH is required")

    install_root = pathlib.Path(install_path)
    expected_sha256 = os.environ.get("PYTHON_STANDALONE_SHA256")

    with tempfile.TemporaryDirectory(prefix="python-standalone-rez-") as temp_root:
        work_dir = pathlib.Path(temp_root)
        artifact = _artifact_path(work_dir)
        if expected_sha256 and _sha256(artifact) != expected_sha256:
            raise RuntimeError("python standalone artifact sha256 mismatch")

        extract_root = work_dir / "extract"
        extract_root.mkdir()
        _extract(artifact, extract_root)

        payload = extract_root / "python" / "install"
        if not payload.is_dir():
            raise RuntimeError("artifact missing python/install payload")
        _copytree_contents(payload, install_root)

    print(f"installed python standalone to {install_root}")


if __name__ == "__main__":
    try:
        build()
    except Exception as exc:
        print(f"python rezbuild failed: {exc}", file=sys.stderr)
        sys.exit(1)
