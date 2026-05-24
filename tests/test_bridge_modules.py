"""
Integration tests for rez-next bridge modules.

Verifies that all bridge modules in python/rez_next/ load correctly
via the runpy bridge pattern.
"""
from __future__ import annotations

import pytest


# ── Bridge module import tests ─────────────────────────────────────

# New bridge modules added in this cycle
NEW_BRIDGE_MODULES = [
    "rez_next.build_",
    "rez_next.bundles",
    "rez_next.cli",
    "rez_next.data",
    "rez_next.depends",
    "rez_next.diff",
    "rez_next.env",
    "rez_next.forward",
    "rez_next.packages",
    "rez_next.pip",
    "rez_next.plugins",
    "rez_next.release",
    "rez_next.resolved_context",
    "rez_next.rex",
    "rez_next.search",
    "rez_next.source",
    "rez_next.suite",
    "rez_next.util",
]

# Existing bridge modules that should still work
EXISTING_BRIDGE_MODULES = [
    "rez_next.bind",
    "rez_next.build_process",
    "rez_next.build_plugins",
    "rez_next.build_system",
    "rez_next.bundle_context",
    "rez_next.command",
    "rez_next.complete",
    "rez_next.deprecations",
    "rez_next.exceptions",
    "rez_next.package_bind",
    "rez_next.package_cache",
    "rez_next.package_copy",
    "rez_next.package_help",
    "rez_next.package_move",
    "rez_next.package_py_utils",
    "rez_next.package_remove",
    "rez_next.package_repository",
    "rez_next.package_search",
    "rez_next.plugin_managers",
    "rez_next.release_hook",
    "rez_next.release_vcs",
    "rez_next.resolver",
    "rez_next.rex_bindings",
    "rez_next.shells",
    "rez_next.solver",
    "rez_next.status",
    "rez_next.system",
    "rez_next.test",
    "rez_next.wrapper",
]


class TestBridgeModules:
    """Verify that all bridge modules can be imported successfully."""

    @pytest.mark.parametrize("module_name", NEW_BRIDGE_MODULES + EXISTING_BRIDGE_MODULES)
    def test_bridge_module_imports(self, module_name: str) -> None:
        """All bridge modules should import without errors."""
        __import__(module_name)


class TestUtilsSubpackage:
    """Verify utils subpackage bridges load correctly."""

    UTILS_SUBMODULES = [
        "rez_next.utils.colorize",
        "rez_next.utils.data_utils",
        "rez_next.utils.filesystem",
        "rez_next.utils.formatting",
        "rez_next.utils.logging_",
        "rez_next.utils.platform_",
        "rez_next.utils.resources",
        "rez_next.utils.yaml",
    ]

    @pytest.mark.parametrize("module_name", UTILS_SUBMODULES)
    def test_utils_submodule_imports(self, module_name: str) -> None:
        """All utils submodules should import without errors."""
        __import__(module_name)


class TestRezNextInitImports:
    """Verify rez_next top-level import works and exposes all submodules."""

    def test_rez_next_import(self) -> None:
        """rez_next should import without errors."""
        import rez_next  # noqa: F811

        assert hasattr(rez_next, "__version__")
        assert hasattr(rez_next, "config")

    def test_rez_next_exceptions_accessible(self) -> None:
        """rez_next.exceptions should be accessible from the package."""
        import rez_next

        assert hasattr(rez_next, "exceptions")

    def test_rez_next_deprecations_accessible(self) -> None:
        """rez_next.deprecations should be accessible from the package."""
        import rez_next

        assert hasattr(rez_next, "deprecations")

    @pytest.mark.parametrize(
        "attr_name",
        [
            "build_",
            "bundles",
            "cli",
            "data",
            "depends",
            "diff",
            "env",
            "forward",
            "packages",
            "pip",
            "plugins",
            "release",
            "resolved_context",
            "rex",
            "search",
            "source",
            "suite",
            "util",
        ],
    )
    def test_new_modules_accessible(self, attr_name: str) -> None:
        """All new bridge modules should be accessible from rez_next."""
        import rez_next

        assert hasattr(rez_next, attr_name), (
            f"rez_next.{attr_name} should be accessible"
        )
