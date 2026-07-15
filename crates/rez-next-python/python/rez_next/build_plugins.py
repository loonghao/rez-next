"""Reusable build helpers for Rez package build scripts.

These helpers are intentionally small and subclass-friendly. They cover common
pipeline package patterns while keeping package-specific policy in `rezbuild.py`
or `build.py`.
"""

from __future__ import annotations

import hashlib
import os
import platform
import shutil
import subprocess
import sys
import tarfile
import urllib.request
import zipfile
from dataclasses import dataclass
from pathlib import Path
from typing import Sequence


@dataclass(frozen=True)
class BuildContext:
    source_path: Path
    build_path: Path
    install_path: Path
    package_name: str
    package_version: str
    variant_index: int
    variant_requires: tuple[str, ...]
    host_platform: str
    host_arch: str
    target_platform: str
    target_arch: str
    target_os: str

    @classmethod
    def from_env(cls) -> BuildContext:
        source_path = Path(os.environ.get("REZ_BUILD_SOURCE_PATH", os.getcwd())).resolve()
        build_path = Path(os.environ.get("REZ_BUILD_PATH", source_path / ".rez_build")).resolve()
        install_path = Path(
            os.environ.get("REZ_BUILD_INSTALL_PATH", build_path / "install")
        ).resolve()
        host_platform = _host_platform()
        target_platform = os.environ.get("REZ_BUILD_TARGET_PLATFORM", host_platform)
        target_arch = os.environ.get("REZ_BUILD_TARGET_ARCH", platform.machine().lower())
        target_os = os.environ.get("REZ_BUILD_TARGET_OS", target_platform.split("-", 1)[0])
        variant_requires = tuple(
            req for req in os.environ.get("REZ_BUILD_VARIANT_REQUIRES", "").split() if req
        )
        return cls(
            source_path=source_path,
            build_path=build_path,
            install_path=install_path,
            package_name=os.environ.get("REZ_BUILD_PACKAGE_NAME", ""),
            package_version=os.environ.get("REZ_BUILD_PACKAGE_VERSION", ""),
            variant_index=int(os.environ.get("REZ_BUILD_VARIANT_INDEX", "0") or "0"),
            variant_requires=variant_requires,
            host_platform=host_platform,
            host_arch=platform.machine().lower(),
            target_platform=target_platform,
            target_arch=target_arch,
            target_os=target_os,
        )


class BuildPlugin:
    """Base class for `rezbuild.py`/`build.py` helpers."""

    def __init__(self, context: BuildContext | None = None) -> None:
        self.context = context or BuildContext.from_env()

    def run(self, command: str | None = None) -> None:
        command = command or (sys.argv[1] if len(sys.argv) > 1 else "install")
        if command == "build":
            self.build()
            return
        if command == "install":
            self.install()
            return
        if command in {"clean", "test"}:
            return
        raise ValueError(f"Unsupported build command: {command}")

    def build(self) -> None:
        print(f"{self.__class__.__name__}: build step ready")

    def install(self) -> None:
        raise NotImplementedError

    def resolve_path(self, value: str | os.PathLike[str]) -> Path:
        path = Path(value)
        if not path.is_absolute():
            path = self.context.source_path / path
        return path.resolve()


class ExtractionBuilder(BuildPlugin):
    """Download or reuse an archive, verify it, extract it, and copy payload."""

    artifact: str | os.PathLike[str] | None = None
    url: str | None = None
    sha256: str | None = None
    sha256_file: str | os.PathLike[str] | None = None
    payload_prefix: str | os.PathLike[str] | None = None

    def __init__(
        self,
        *,
        artifact: str | os.PathLike[str] | None = None,
        url: str | None = None,
        sha256: str | None = None,
        sha256_file: str | os.PathLike[str] | None = None,
        payload_prefix: str | os.PathLike[str] | None = None,
        context: BuildContext | None = None,
    ) -> None:
        super().__init__(context=context)
        self.artifact = artifact if artifact is not None else self.artifact
        self.url = url if url is not None else self.url
        self.sha256 = sha256 if sha256 is not None else self.sha256
        self.sha256_file = sha256_file if sha256_file is not None else self.sha256_file
        self.payload_prefix = payload_prefix if payload_prefix is not None else self.payload_prefix

    def select_artifact(self) -> Path:
        if self.artifact:
            artifact = self.resolve_path(self.artifact)
            if not artifact.is_file():
                raise FileNotFoundError(f"archive artifact does not exist: {artifact}")
            return artifact
        if not self.url:
            raise ValueError("ExtractionBuilder requires artifact or url")

        downloads = self.context.build_path / "downloads"
        downloads.mkdir(parents=True, exist_ok=True)
        filename = self.url.rsplit("/", 1)[-1] or "artifact"
        destination = downloads / filename
        urllib.request.urlretrieve(self.url, destination)
        return destination

    def expected_sha256(self) -> str | None:
        if self.sha256:
            return self.sha256.strip()
        if self.sha256_file:
            return self.resolve_path(self.sha256_file).read_text(encoding="utf-8").strip()
        return None

    def install(self) -> None:
        artifact = self.select_artifact()
        expected = self.expected_sha256()
        if expected:
            verify_sha256(artifact, expected)

        extract_dir = self.context.build_path / "extract"
        if extract_dir.exists():
            shutil.rmtree(extract_dir)
        extract_dir.mkdir(parents=True, exist_ok=True)
        extract_archive(artifact, extract_dir)

        payload = extract_dir / self.payload_prefix if self.payload_prefix else extract_dir
        if not payload.is_dir():
            raise FileNotFoundError(f"archive payload directory does not exist: {payload}")
        copy_tree_contents(payload, self.context.install_path)
        print(f"{self.__class__.__name__}: installed {artifact} -> {self.context.install_path}")


class PipFromDownloadBuilder(BuildPlugin):
    """Install a pip-compatible package artifact or spec into a Rez package."""

    package: str | os.PathLike[str] | None = None
    requirement: str | os.PathLike[str] | None = None
    extra_args: Sequence[str] = ()
    with_deps: bool = False
    target_subdir: str = "python"

    def __init__(
        self,
        *,
        package: str | os.PathLike[str] | None = None,
        requirement: str | os.PathLike[str] | None = None,
        extra_args: Sequence[str] = (),
        with_deps: bool | None = None,
        target_subdir: str | None = None,
        context: BuildContext | None = None,
    ) -> None:
        super().__init__(context=context)
        self.package = package if package is not None else self.package
        self.requirement = requirement if requirement is not None else self.requirement
        self.extra_args = tuple(extra_args or self.extra_args)
        self.with_deps = self.with_deps if with_deps is None else with_deps
        self.target_subdir = target_subdir or self.target_subdir

    def install(self) -> None:
        target = self.context.install_path / self.target_subdir
        target.mkdir(parents=True, exist_ok=True)

        args = [
            sys.executable,
            "-m",
            "pip",
            "install",
            "--disable-pip-version-check",
            "--target",
            str(target),
        ]
        if not self.with_deps:
            args.append("--no-deps")
        args.extend(self.extra_args)
        if self.requirement:
            args.extend(["-r", str(self.resolve_path(self.requirement))])
        if self.package:
            package = (
                self.resolve_path(self.package) if _looks_like_path(self.package) else self.package
            )
            args.append(str(package))
        if not self.package and not self.requirement:
            raise ValueError("PipFromDownloadBuilder requires package or requirement")

        subprocess.run(args, cwd=self.context.source_path, env=_subprocess_env(), check=True)
        print(f"{self.__class__.__name__}: installed pip payload -> {target}")


def verify_sha256(path: Path, expected: str) -> None:
    hasher = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            hasher.update(chunk)
    actual = hasher.hexdigest()
    if actual != expected:
        raise ValueError(f"archive sha256 mismatch: expected {expected}, got {actual}")


def extract_archive(artifact: Path, destination: Path) -> None:
    name = artifact.name.lower()
    if name.endswith(".zip") or name.endswith(".whl"):
        with zipfile.ZipFile(artifact) as archive:
            archive.extractall(destination)
        return
    if name.endswith(".tar.gz") or name.endswith(".tgz"):
        with tarfile.open(artifact, "r:gz") as archive:
            archive.extractall(destination)
        return
    raise ValueError(f"unsupported archive format: {artifact}")


def copy_tree_contents(source: Path, destination: Path) -> None:
    destination.mkdir(parents=True, exist_ok=True)
    for entry in source.iterdir():
        target = destination / entry.name
        if entry.is_dir():
            if target.exists():
                shutil.rmtree(target)
            shutil.copytree(entry, target)
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(entry, target)


def _looks_like_path(value: str | os.PathLike[str]) -> bool:
    text = os.fspath(value)
    return any(sep in text for sep in ("/", "\\")) or text.endswith(
        (".whl", ".zip", ".tar.gz", ".tgz")
    )


def _host_platform() -> str:
    system = platform.system().lower()
    if system == "darwin":
        system = "macos"
    return f"{system}-{platform.machine().lower()}"


def _subprocess_env() -> dict[str, str]:
    env = dict(os.environ)
    if os.name == "nt":
        for key in ("SystemRoot", "WINDIR", "TEMP", "TMP"):
            if key not in env and key in os.environ:
                env[key] = os.environ[key]
    return env
