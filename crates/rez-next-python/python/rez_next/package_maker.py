"""
Package maker module for creating Rez packages programmatically.

Provides PackageMaker class and make_package context manager
for creating and installing Rez packages.

Compatible with Rez's `package_maker` module API.
"""

import os
import uuid as _uuid
from contextlib import contextmanager
from pathlib import Path

from ._native.packages import Package


class PackageMaker:
    """Container for package metadata used to create Package objects.

    Stores package metadata fields and provides a get_package() method
    that creates a validated Package instance.

    Compatible with Rez's PackageMaker class.

    Example:
        maker = PackageMaker("mypkg")
        maker.version = "1.0.0"
        maker.requires = ["python-3.9"]
        pkg = maker.get_package()
    """

    def __init__(self, name):
        self._data = {"name": name}
        self.installed_variants = []
        self.skipped_variants = []

    def __getattr__(self, name):
        if name.startswith("_"):
            raise AttributeError(name)
        # Delegate to internal data dictionary for arbitrary fields
        if name in self._data:
            return self._data[name]
        raise AttributeError(
            f"'{type(self).__name__}' object has no attribute '{name}'"
        )

    def __setattr__(self, name, value):
        if name.startswith("_"):
            super().__setattr__(name, value)
        else:
            self._data[name] = value

    def get_package(self):
        """Create a Package from the stored metadata.

        Returns:
            Package: A validated Package instance populated with stored metadata.

        Raises:
            ValueError: If the package data is invalid (e.g., empty name).
        """
        data = dict(self._data)
        name = data.pop("name")
        pkg = Package(name)

        for key, value in data.items():
            if key == "version":
                pkg.set_version(str(value))
            elif key in (
                "description", "uuid", "build_command", "build_system",
                "pre_commands", "post_commands", "pre_test_commands",
                "pre_build_commands", "commands", "requires_rez_version",
            ):
                if value is not None:
                    setattr(pkg, key, value)
            elif key in (
                "requires", "build_requires", "private_build_requires",
                "variants", "tools", "authors",
            ):
                if value:
                    setattr(pkg, key, list(value))
            elif key in ("relocatable", "cachable", "is_dev_package"):
                if value is not None:
                    setattr(pkg, key, bool(value))
            elif key == "tests":
                if value:
                    setattr(pkg, key, dict(value))
            elif key == "config":
                if value:
                    setattr(pkg, key, dict(value))
            elif key == "timestamp":
                if value is not None:
                    pkg.timestamp = int(value)

        # Validate before returning
        pkg.validate()
        return pkg


@contextmanager
def make_package(
    name,
    path=None,
    make_base=None,
    make_root=None,
    skip_existing=True,
    warn_on_skip=True,
):
    """Context manager for creating and installing a Rez package.

    Creates a PackageMaker, yields it for metadata population, then
    writes the package definition to disk on context exit.

    Args:
        name: Package name (required).
        path: Installation root directory. If None, package is created
            but not written to disk. If a path is given, the package
            is installed to ``path/<name>/<version>/``.
        make_base: Optional callable invoked with the base directory path
            after installation. Used to place payload files.
        make_root: Optional callable invoked with the root directory path
            after installation. Used to place payload files.
        skip_existing: If True (default), skip install of variants whose
            package.py already exists.
        warn_on_skip: If True (default), emit a warning when skipping
            an existing variant.

    Yields:
        PackageMaker: A metadata container for setting package fields.

    Example:
        with make_package("mypkg", path="/packages") as pkg:
            pkg.version = "1.0.0"
            pkg.requires = ["python-3.9"]
            pkg.commands = '    env.PATH.prepend("{root}/bin")'

    Note:
        Inside the ``with`` block, set package metadata on the yielded
        PackageMaker. On exit, the package.py is generated and written.
    """
    maker = PackageMaker(name)
    yield maker

    # Create the Package object from collected metadata
    pkg = maker.get_package()

    if path is None:
        return

    # Resolve install directory
    path = Path(path).resolve()
    version_str = str(pkg.version) if pkg.version else "1.0.0"
    install_path = path / pkg.name / version_str
    base_path = install_path / "base"
    root_path = install_path / "root"

    # Create directories
    base_path.mkdir(parents=True, exist_ok=True)
    root_path.mkdir(parents=True, exist_ok=True)

    # Write package.py
    py_content = pkg.to_package_py()
    pkg_file = install_path / "package.py"
    pkg_file.write_text(py_content, encoding="utf-8")

    # Install variants
    maker.installed_variants = []
    maker.skipped_variants = []

    if pkg.variants:
        for variant_idx in range(len(pkg.variants)):
            variant_install_path = install_path / str(variant_idx)
            variant_pkg_path = variant_install_path / "package.py"

            if skip_existing and variant_pkg_path.exists():
                maker.skipped_variants.append(variant_idx)
                if warn_on_skip:
                    import warnings

                    warnings.warn(
                        f"Variant {variant_idx} already exists at "
                        f"{variant_install_path}, skipping"
                    )
                continue

            variant_install_path.mkdir(parents=True, exist_ok=True)
            variant_pkg_path.write_text(py_content, encoding="utf-8")
            maker.installed_variants.append(variant_idx)

    # Invoke callbacks with base/root directories
    if make_base:
        make_base(str(base_path))
    if make_root:
        make_root(str(root_path))


def install_package(pkg, path, make_root=None, make_base=None):
    """Install a package to a given path.

    Writes the package.py file and creates base/root directories,
    then optionally invokes callbacks for payload placement.

    Args:
        pkg: Package instance to install.
        path: Installation root directory.
        make_root: Optional callback called with root directory path.
        make_base: Optional callback called with base directory path.

    Returns:
        Path: The installation directory.

    Example:
        pkg = Package("mypkg")
        pkg.set_version("1.0.0")
        install_package(pkg, "/packages",
                        make_root=lambda r: Path(r).mkdir(parents=True))
    """
    path = Path(path).resolve()
    version_str = str(pkg.version) if pkg.version else "1.0.0"
    install_path = path / pkg.name / version_str
    base_path = install_path / "base"
    root_path = install_path / "root"

    base_path.mkdir(parents=True, exist_ok=True)
    root_path.mkdir(parents=True, exist_ok=True)

    # Write package.py
    py_content = pkg.to_package_py()
    (install_path / "package.py").write_text(py_content, encoding="utf-8")

    if make_base:
        make_base(str(base_path))
    if make_root:
        make_root(str(root_path))

    return install_path


def create_package(data):
    """Create a Package from a dictionary.

    Convenience wrapper around Package.from_dict().

    Args:
        data: Dictionary with package metadata. Must contain "name".
            Optional keys: version, description, requires, variants, etc.

    Returns:
        Package: A new Package instance.

    Example:
        pkg = create_package({
            "name": "mypkg",
            "version": "1.0.0",
            "requires": ["python-3.9"],
        })
    """
    from ._native.packages import Package

    return Package.from_dict(data)
