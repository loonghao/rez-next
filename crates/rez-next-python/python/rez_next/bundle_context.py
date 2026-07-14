"""
Bundle context module — aligns with rez.bundle_context.

Provides the ``bundle_context()`` function for creating relocatable
context bundles from resolved environments.
"""

from __future__ import annotations

import os
from typing import Optional

from rez_next._native import bundles
from rez_next.resolved_context import ResolvedContext


def bundle_context(
    context: ResolvedContext,
    dest_dir: str,
    force: bool = False,
    skip_non_relocatable: bool = False,
    quiet: bool = False,
    patch_libs: bool = False,
    verbose: bool = False,
) -> None:
    """Bundle a resolved context and its variants into a relocatable directory.

    This creates a self-contained directory with all packages copied into
    a local repository, plus a retargeted context file (``context.rxt``).

    Args:
        context: Resolved context to bundle.
        dest_dir: Destination directory (must not exist).
        force: Force relocation of non-relocatable packages.
        skip_non_relocatable: Skip non-relocatable packages instead of erroring.
        quiet: Suppress all output.
        patch_libs: Patch binaries to use relative library paths (Linux only).
        verbose: Verbose output.

    Rez API: ``rez.bundle_context.bundle_context()``
    """
    # If dest_dir exists, check force
    if os.path.exists(dest_dir):
        if not force:
            raise FileExistsError(f"Destination directory already exists: {dest_dir}")
    else:
        os.makedirs(dest_dir, exist_ok=True)

    # Extract package request strings from context
    package_requests = getattr(context, "package_requests", None) or []
    request_strs = [str(r) for r in package_requests]

    # Delegate to native bundle implementation
    bundle_result = bundles.bundle_context(
        request_strs,
        dest_dir,
        skip_solve=False,
    )

    if not quiet and bundle_result:
        print(f"Bundle created at: {bundle_result}")

    if verbose:
        print(f"Context: {context}")
        print(f"Packages bundled: {len(request_strs)}")
        print(f"Destination: {dest_dir}")
