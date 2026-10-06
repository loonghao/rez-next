#!/usr/bin/env python3
"""vx extension entrypoint driving rez-next's release workflow.

Invoked by vx as `vx x rez-release release [PATH] [options]`. vx runs this
script with the extension directory as the working directory, so the package
to release is resolved from the `path` argument, falling back to
`VX_PROJECT_DIR` (the directory vx was invoked from).

Exit codes: 0 success, 1 release reported errors, 2 usage error,
3 rez_next is not importable, 4 unexpected failure.
"""

from __future__ import annotations

import argparse
import json
import os
import sys

EXIT_OK = 0
EXIT_RELEASE_FAILED = 1
EXIT_USAGE = 2
EXIT_MISSING_REZ_NEXT = 3
EXIT_UNEXPECTED = 4


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="vx x rez-release",
        description="Release a rez-next package from a source directory.",
    )
    parser.add_argument(
        "path",
        nargs="?",
        default=None,
        help="Package source directory (defaults to the directory vx was invoked from)",
    )
    parser.add_argument(
        "-n",
        "--dry-run",
        action="store_true",
        help="Validate only: do not build, install, or touch the VCS",
    )
    parser.add_argument(
        "-l",
        "--local",
        action="store_true",
        help="Release into local_packages_path instead of release_packages_path",
    )
    parser.add_argument(
        "-m", "--message", default=None, help="Release message for the VCS tag"
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit the raw release result as JSON on stdout",
    )
    return parser


def resolve_source_dir(path: str | None) -> tuple[str, str | None]:
    """Resolve the package directory and report how it was determined."""
    if path:
        return path, "argument"

    from_env = os.environ.get("VX_PROJECT_DIR")
    if from_env:
        return from_env, "VX_PROJECT_DIR"
    return os.getcwd(), "current directory"


def main(argv: list[str]) -> int:
    args = build_parser().parse_args(argv)

    source_dir, origin = resolve_source_dir(args.path)

    if not os.path.isdir(source_dir):
        print(
            f"error: package directory not found: {source_dir} (resolved from {origin})",
            file=sys.stderr,
        )
        return EXIT_USAGE

    try:
        from rez_next.release import release_package
    except ImportError as exc:
        print(
            "error: the `rez-next` Python package is not importable.\n"
            "       Install it with `pip install rez-next`, or run `just py-build`\n"
            f"       from a rez-next checkout. (import failed with: {exc})",
            file=sys.stderr,
        )
        return EXIT_MISSING_REZ_NEXT

    print(f"Releasing from {source_dir} (resolved from {origin})")
    if args.dry_run:
        print("Mode: dry-run (no build, no install, no VCS changes)")
    elif args.local:
        print("Mode: local (install into local_packages_path)")
    else:
        print("Mode: release (install into release_packages_path)")

    try:
        result = release_package(source_dir, args.local, args.dry_run, args.message)
    except RuntimeError as exc:  # rez-next raises hard failures as RuntimeError
        print(f"error: release failed: {exc}", file=sys.stderr)
        return EXIT_UNEXPECTED

    if args.json:
        print(
            json.dumps(
                {
                    "success": result.success,
                    "package_name": result.package_name,
                    "version": result.version,
                    "install_path": result.install_path,
                    "vcs_metadata": result.vcs_metadata,
                    "changelog": result.changelog,
                    "errors": list(result.errors),
                    "warnings": list(result.warnings),
                },
                indent=2,
            )
        )
        return EXIT_OK if result.success else EXIT_RELEASE_FAILED

    for warning in result.warnings:
        print(f"warning: {warning}", file=sys.stderr)

    if not result.success:
        for error in result.errors:
            print(f"error: {error}", file=sys.stderr)
        print(
            f"error: release of {result.package_name or '<unknown>'} failed",
            file=sys.stderr,
        )
        return EXIT_RELEASE_FAILED

    label = f"{result.package_name}-{result.version}"
    print(f"Released {label} -> {result.install_path}")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
