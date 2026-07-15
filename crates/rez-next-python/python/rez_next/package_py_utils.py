"""
package_py_utils — aligns with rez.package_py_utils.

Provides utility functions intended to be imported and used inside
``package.py`` files, in functions including the ``preprocess`` function
and early-bound ``@early`` functions.

Rez API: ``rez.package_py_utils``
"""

from __future__ import annotations

import subprocess
from typing import Any


def expand_requirement(request: str, paths: list[str] | None = None) -> str:
    """Expand wildcard version tokens in a requirement string.

    Wildcards (``*`` and ``**``) are expanded to the latest matching version.
    ``*`` replaces a single version token; ``**`` replaces all remaining tokens.

    Examples::

        >>> expand_requirement('python-2.*')
        'python-2.7'
        >>> expand_requirement('python==2.**')
        'python==2.7.12'

    Args:
        request: Requirement string with optional wildcards.
        paths: Package search paths (defaults to ``packages_path``).

    Returns:
        Expanded requirement string without wildcards.
    """
    if "*" not in request:
        return request

    from uuid import uuid4

    from rez_next.packages_ import get_latest_package
    from rez_next.version import Requirement, Version, VersionRange

    wildcard_map: dict[str, str] = {}
    expanded_versions: dict[Version, Version] = {}
    request_ = request

    # Replace wildcards with unique temporary tokens
    while "**" in request_:
        uid = f"_{uuid4().hex}_"
        request_ = request_.replace("**", uid, 1)
        wildcard_map[uid] = "**"

    while "*" in request_:
        uid = f"_{uuid4().hex}_"
        request_ = request_.replace("*", uid, 1)
        wildcard_map[uid] = "*"

    req = Requirement(request_, invalid_bound_error=False)

    def expand_version(version: Version) -> Version | None:
        rank = len(version)
        wildcard_found = False

        while version and str(version[-1]) in wildcard_map:
            token = wildcard_map[str(version[-1])]
            version = version.trim(len(version) - 1)
            if token == "**":
                if wildcard_found:
                    return None
                wildcard_found = True
                rank = 0
                break
            wildcard_found = True

        if not wildcard_found:
            return None

        range_ = VersionRange(str(version))
        package = get_latest_package(name=req.name, range_=range_, paths=paths)
        if package is None:
            return version
        if rank:
            return package.version.trim(rank)
        return package.version

    def visit_version(version: Version) -> Version | None:
        for v, expanded_v in expanded_versions.items():
            if version == next(v):
                return next(expanded_v)
        result = expand_version(version)
        if result is not None:
            expanded_versions[version] = result
        return result

    if req.range_ is not None:
        req.range_.visit_versions(visit_version)

    result = str(req)

    # Restore any remaining uid tokens as wildcards
    for uid, token in wildcard_map.items():
        result = result.replace(uid, token)

    expanded_req = Requirement(result)
    return str(expanded_req)


def expand_requires(*requests: str, paths: list[str] | None = None) -> list[str]:
    """Expand wildcards across multiple requirement strings.

    Args:
        *requests: One or more requirement strings.
        paths: Package search paths.

    Returns:
        List of expanded requirement strings.
    """
    return [expand_requirement(x, paths=paths) for x in requests]


def exec_command(attr: str, cmd: list[str]) -> tuple[str, str]:
    """Run a subprocess to calculate a package attribute.

    Args:
        attr: Package attribute name (used in error messages).
        cmd: Command to run as a list of strings.

    Returns:
        ``(stdout, stderr)`` tuple.

    Raises:
        InvalidPackageError: If the command exits with a non-zero status.
    """
    p = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    out, err = p.communicate()

    if p.returncode:
        from rez_next.exceptions import InvalidPackageError

        raise InvalidPackageError(f"Error determining package attribute '{attr}':\n{err}")

    return out.strip(), err.strip()


def exec_python(
    attr: str,
    src: str | list[str],
    executable: str = "python",
) -> str:
    """Run a Python subprocess to calculate a package attribute.

    Args:
        attr: Package attribute name.
        src: Python code to execute (single string or list of lines).
        executable: Python executable path.

    Returns:
        Striped stdout from the subprocess.

    Raises:
        InvalidPackageError: If the command exits with a non-zero status.
    """
    if isinstance(src, list):
        src = "; ".join(src)

    p = subprocess.Popen(
        [executable, "-c", src],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    out, err = p.communicate()

    if p.returncode:
        from rez_next.exceptions import InvalidPackageError

        raise InvalidPackageError(f"Error determining package attribute '{attr}':\n{err}")

    return out.strip()


def find_site_python(
    module_name: str,
    paths: list[str] | None = None,
) -> Any:
    """Find the rez native Python package containing a given module.

    This is used by Python 'native' rez installers to find the rez package
    that represents the Python installation hosting a given module.

    Args:
        module_name: Target Python module (e.g. ``'PySide2'``).
        paths: Package search paths.

    Returns:
        The matching ``Package`` object.

    Raises:
        InvalidPackageError: If no matching package is found.
    """
    import ast

    from rez_next.exceptions import InvalidPackageError
    from rez_next.packages_ import iter_packages

    py_cmd = f"import {module_name}; print({module_name}.__path__)"
    p = subprocess.Popen(
        ["python", "-c", py_cmd],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    out, err = p.communicate()

    if p.returncode:
        raise InvalidPackageError(f"Failed to find installed python module '{module_name}':\n{err}")

    module_paths = ast.literal_eval(out.strip())

    def issubdir(path: str, parent: str) -> bool:
        return path.startswith(parent + os.sep)

    import os

    for package in iter_packages("python", paths=paths):
        site_paths = getattr(package, "_site_paths", None)
        if not site_paths:
            continue
        contained = all(any(issubdir(mp, sp) for sp in site_paths) for mp in module_paths)
        if contained:
            return package

    raise InvalidPackageError(
        f"Failed to find python installation containing module "
        f"'{module_name}'. Has python been installed as a rez package?"
    )


__all__ = [
    "expand_requirement",
    "expand_requires",
    "exec_command",
    "exec_python",
    "find_site_python",
]
