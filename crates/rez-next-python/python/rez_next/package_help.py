"""
Package Help module for rez-next.

Provides PackageHelp class that matches rez's rez.package_help.PackageHelp interface.
"""

import subprocess
import webbrowser
from typing import List, Optional, Tuple

from . import packages_ as _packages


class PackageHelp:
    """
    Object for extracting and viewing help for a package.

    Given a package name and version range, help will be extracted from the latest
    package in the version range that provides it.

    This matches rez's `rez.package_help.PackageHelp` interface.
    """

    def __init__(
        self,
        package_name: str,
        version_range: Optional[str] = None,
        paths=None,
        verbose: bool = False,
    ) -> None:
        """
        Create a PackageHelp object.

        Args:
            package_name: Package to search.
            version_range: Version range (optional).
        """
        self.package = None
        self._sections = []
        self._verbose = verbose

        # Find latest package with a help entry
        if version_range:
            it = _packages.iter_packages(package_name, range_=version_range)
        else:
            it = _packages.iter_packages(package_name)

        # Sort by version descending (latest first)
        packages = sorted(it, key=lambda x: x.version if x.version else "", reverse=True)

        for pkg in packages:
            if self._verbose:
                print(f"Searching for help in {pkg.uri}")

            # Check if package has help (string format)
            if hasattr(pkg, "help") and pkg.help is not None:
                help_val = pkg.help

                if isinstance(help_val, str):
                    # String format: single section
                    self._sections = [["Help", help_val]]
                elif isinstance(help_val, list):
                    # List format: multiple sections
                    self._sections = help_val
                else:
                    # Unknown format
                    self._sections = [["Help", str(help_val)]]

                self.package = pkg

                if self._verbose:
                    print(f"Found {len(self._sections)} help entries in {pkg.uri}")
                break

    @property
    def success(self) -> bool:
        """Return True if help was found, False otherwise."""
        return len(self._sections) > 0

    @property
    def sections(self) -> List[Tuple[str, str]]:
        """Returns a list of (name, uri) 2-tuples."""
        return self._sections

    def open(self, section_index: int = 0) -> None:
        """Launch a help section."""
        if not self.success:
            raise RuntimeError("No help found for package")

        if section_index >= len(self._sections):
            raise IndexError(
                f"Section index {section_index} out of range (0-{len(self._sections) - 1})"
            )

        uri = self._sections[section_index][1]

        # Split by whitespace and check if it's a single token (URL or path)
        parts = uri.split()
        if len(parts) == 1:
            # Single token: treat as URL or file path
            if self._verbose:
                print(f"Opening URL/file: {uri}")
            webbrowser.open_new(uri)
        else:
            # Multiple parts: treat as command to execute
            if self._verbose:
                print(f"Running command: {uri}")
            subprocess.Popen(uri, shell=True)

    def print_info(self, buf=None) -> None:
        """Print help sections."""
        if buf is None:
            import sys

            buf = sys.stdout

        buf.write("Sections:\n")
        for i, (name, uri) in enumerate(self._sections):
            buf.write(f"\t{i + 1}:\t{name} ({uri})\n")

    @classmethod
    def open_rez_manual(cls) -> None:
        """Open the Rez user manual."""
        # This should read from rez config, but for now use a default URL
        manual_url = "https://rez.readthedocs.io/en/stable/"
        webbrowser.open_new(manual_url)
