"""
Wrapper module — aligns with rez.wrapper.

Provides the `Wrapper` class, which wraps a suite tool as an executable
that can be invoked from the command line.
"""

from __future__ import annotations

import os
import sys
from typing import Optional

from rez_next._native.suite import Suite
from rez_next.resolved_context import ResolvedContext


class Wrapper:
    """A wrapper for a suite tool.

    Wrappers are executable files created by a Suite, stored in the suite's
    ``./bin`` directory. When executed, the wrapper loads the matching context
    from the suite and runs the wrapped tool within it.

    Rez API: ``rez.wrapper.Wrapper``
    """

    def __init__(self, filepath: str) -> None:
        """Create a Wrapper from its executable file.

        Args:
            filepath: Path to the wrapper executable (YAML file).
        """
        self._filepath = filepath
        self._suite_path: Optional[str] = None
        self._context_name: Optional[str] = None
        self._context: Optional[ResolvedContext] = None
        self._tool_name: Optional[str] = None
        self._prefix_char: Optional[str] = None
        self._suite_instance: Optional[Suite] = None
        self._parse_wrapper()

    def _parse_wrapper(self) -> None:
        """Parse the wrapper YAML file to extract suite and tool metadata."""
        import yaml  # using rez vendor yaml

        if not os.path.isfile(self._filepath):
            from rez_next.exceptions import RezSystemError

            raise RezSystemError(f"Wrapper file not found: {self._filepath}")

        try:
            with open(self._filepath, "r") as f:
                data = yaml.safe_load(f)
        except Exception as e:
            from rez_next.exceptions import RezSystemError

            raise RezSystemError(f"Failed to parse wrapper '{self._filepath}': {e}")

        if not isinstance(data, dict):
            from rez_next.exceptions import RezSystemError

            raise RezSystemError(f"Invalid wrapper format in '{self._filepath}'")

        suite_path = data.get("suite_path") or data.get("suite")
        if not suite_path:
            from rez_next.exceptions import RezSystemError

            raise RezSystemError(
                f"Wrapper '{self._filepath}' missing 'suite_path'"
            )

        self._suite_path = suite_path
        self._context_name = data.get("context_name")
        self._tool_name = data.get("tool_name")
        self._prefix_char = data.get("prefix_char")

        # Load context if context file path is provided
        context_path = data.get("context_path") or data.get("context_rxt")
        if context_path and os.path.isfile(context_path):
            try:
                self._context = ResolvedContext.load(context_path)
            except Exception:
                self._context = None

    @property
    def suite(self) -> Suite:
        """The Suite that owns this wrapper (cached)."""
        if self._suite_instance is None and self._suite_path is not None:
            try:
                self._suite_instance = Suite.load(self._suite_path)
            except (OSError, Exception) as e:
                from rez_next.exceptions import RezSystemError

                raise RezSystemError(
                    f"Failed to load suite from '{self._suite_path}': {e}"
                ) from e
        return self._suite_instance

    @property
    def filepath(self) -> str:
        """Path to the wrapper executable file."""
        return self._filepath

    @property
    def tool_name(self) -> Optional[str]:
        """Name of the tool wrapped by this wrapper."""
        return self._tool_name

    @property
    def context_name(self) -> Optional[str]:
        """Name of the context in the suite."""
        return self._context_name

    def run(self, *args: str) -> int:
        """Invoke the wrapped tool within its context.

        Args:
            *args: Arguments to pass to the tool.

        Returns:
            Process exit code.
        """
        if self._prefix_char:
            return self._run(self._prefix_char, args)
        return self._run_no_args(args)

    def _run_no_args(self, args: tuple[str, ...]) -> int:
        """Run the tool directly without prefix argument parsing."""
        if self._context is None:
            from rez_next.exceptions import RezSystemError

            raise RezSystemError("No context loaded for wrapper")
        return self._context.execute(self._tool_name, *args)

    def _run(self, prefix_char: str, args: tuple[str, ...]) -> int:
        """Run the tool with prefix argument parsing (e.g., ``+about``)."""
        import argparse

        parser = argparse.ArgumentParser(prog=self._tool_name or "wrapper")
        parser.add_argument(f"{prefix_char}about", action="store_true", help="Print tool info")
        parser.add_argument(
            f"{prefix_char}interactive", action="store_true", help="Start interactive shell"
        )
        parser.add_argument(
            f"{prefix_char}versions", action="store_true", help="Print package versions"
        )
        parser.add_argument(
            f"{prefix_char}peek", action="store_true", help="Check context staleness"
        )

        known, unknown = parser.parse_known_args(list(args))

        if getattr(known, f"{prefix_char}about", False):
            return self.print_about()
        if getattr(known, f"{prefix_char}versions", False):
            return self.print_package_versions()
        if getattr(known, f"{prefix_char}peek", False):
            return self.peek()
        if getattr(known, f"{prefix_char}interactive", False):
            return self._run_interactive()

        # Forward remaining args to tool
        if self._context is not None:
            return self._context.execute(self._tool_name, *unknown)
        return 1

    def _run_interactive(self) -> int:
        """Start an interactive shell in the wrapper's context."""
        from rez_next.env import create_env

        if self._context is None:
            return 1
        shell = create_env(str(self._context))
        return shell.execute_shell()

    def print_about(self) -> int:
        """Print information about this wrapper and its suite.

        Returns:
            Exit code (0).
        """
        print(f"Tool:     {self._tool_name}")
        print(f"Suite:    {self._suite_path}")
        print(f"Context:  {self._context_name}")
        if self._context is not None:
            print(f"Packages: {len(self._context.resolved_packages)}")

        return 0

    def print_package_versions(self) -> int:
        """Print all versions of packages providing this tool.

        Returns:
            Exit code (0 if no conflicts, 1 if conflicts).
        """
        if self._context is None:
            print("No context loaded")
            return 1

        from rez_next.packages_ import iter_packages

        current_versions: dict[str, str] = {}
        for pkg in self._context.resolved_packages:
            name = getattr(pkg, "name", "")
            ver = getattr(pkg, "version", "") or getattr(pkg, "version_str", "")
            if name:
                current_versions[name] = str(ver)

        output_lines: list[list[str]] = []
        has_conflicts = False

        for pkg_name in sorted(current_versions):
            current_ver = current_versions[pkg_name]
            line = [pkg_name, current_ver]

            # Check latest available version
            try:
                latest = next(iter_packages(pkg_name), None)
                if latest is not None:
                    latest_ver = str(
                        getattr(latest, "version", "") or getattr(latest, "version_str", "")
                    )
                    if latest_ver and latest_ver != current_ver:
                        line.append(f"(latest: {latest_ver})")
                        has_conflicts = True
                    else:
                        line.append("(up-to-date)")
                else:
                    line.append("(external)")
            except Exception:
                line.append("(error)")

            output_lines.append(line)

        # Print table
        for line in output_lines:
            print("  ".join(f"{col:<20}" for col in line))

        return 1 if has_conflicts else 0

    def peek(self) -> int:
        """Compare current context with a re-resolved version.

        Reports any staleness (e.g., newer package versions available).

        Returns:
            Exit code (0).
        """
        if self._context is None:
            print("No context loaded — cannot peek")
            return 0

        try:
            from rez_next.diff import diff_contexts

            package_requests = getattr(self._context, "package_requests", None) or []
            request_strs = [str(r) for r in package_requests]

            if not request_strs:
                print("Context has no package requests — nothing to compare")
                return 0

            # Re-resolve the context
            from rez_next.resolved_context import ResolvedContext as RC

            fresh = RC.resolve_packages(request_strs)

            result = diff_contexts(
                [str(p) for p in self._context.resolved_packages],
                [str(p) for p in fresh.resolved_packages],
            )
            print(f"Context staleness analysis:")
            print(f"  Added packages:   {result.added_count}")
            print(f"  Removed packages: {result.removed_count}")
            print(f"  Changed packages: {result.changed_count}")
        except Exception as e:
            print(f"Peek failed: {e}")

        return 0

    def __repr__(self) -> str:
        return (
            f"Wrapper(filepath={self._filepath!r}, "
            f"tool={self._tool_name!r}, context={self._context_name!r})"
        )
