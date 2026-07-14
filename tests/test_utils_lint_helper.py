"""
Tests for rez_next.utils.lint_helper module.

Aligns with rez's ``utils.lint_helper`` API.
The module is designed so that ANY name imported from it returns None,
satisfying linters that would otherwise flag undefined variables.
"""

import pytest
from types import ModuleType
import sys


class TestLintHelper:
    """Tests for lint_helper NoneModule."""

    def test_module_is_none_module(self):
        """The module object should be a NoneModule instance."""
        import importlib
        mod = importlib.import_module("rez_next.utils.lint_helper")
        assert isinstance(mod, ModuleType)
        # NoneModule.__getattr__ returns None for all names
        assert mod.any_random_name is None

    def test_import_returns_none(self):
        """Importing any name from lint_helper should return None."""
        from rez_next.utils.lint_helper import some_variable  # noqa: F811
        assert some_variable is None

    def test_import_noner_returns_none(self):
        """Even 'noner' itself returns None when imported (NoneModule self-referencing)."""
        from rez_next.utils.lint_helper import noner  # noqa: F811
        # Since the module IS a NoneModule, accessing 'noner' on it returns None
        assert noner is None

    def test_none_module_getattr(self):
        """NoneModule's __getattr__ returns None for any attribute."""
        import importlib
        mod = importlib.import_module("rez_next.utils.lint_helper")
        assert mod.does_not_exist is None
        assert mod.anything_here is None

    def test_module_replaces_itself(self):
        """Verify the module is replaced by NoneModule in sys.modules."""
        mod = sys.modules.get("rez_next.utils.lint_helper")
        assert mod is not None
        # Accessing any attribute returns None
        assert mod.any_name is None

    def test_used_method_exists_and_works(self):
        """The 'used' method is defined on NoneModule and can be called."""
        # 'used' is an actual method on NoneModule, so __getattr__ is NOT triggered.
        # This is by design — can be used to suppress 'unused variable' lints.
        import importlib
        mod = importlib.import_module("rez_next.utils.lint_helper")
        assert hasattr(mod, "used")
        # It should not raise
        mod.used("anything")
