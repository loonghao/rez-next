"""
Tests for rez_next.wrapper module.
"""

import os
import tempfile

import pytest

from rez_next.wrapper import Wrapper


class TestWrapperInit:
    """Test Wrapper creation and parsing."""

    def test_init_raises_on_missing_file(self):
        """Wrapper should raise on non-existent file."""
        from rez_next.exceptions import RezSystemError

        with pytest.raises(RezSystemError):
            Wrapper("/nonexistent/wrapper.yaml")

    def test_init_raises_on_invalid_yaml(self):
        """Wrapper should raise on invalid YAML."""
        from rez_next.exceptions import RezSystemError

        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".yaml", delete=False
        ) as f:
            f.write("not: valid: yaml: [content")
            tmpfile = f.name

        try:
            with pytest.raises(RezSystemError):
                Wrapper(tmpfile)
        finally:
            os.unlink(tmpfile)

    def test_init_parses_valid_wrapper(self):
        """Wrapper should parse a valid wrapper file."""
        from rez_next._native.suite import Suite

        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".yaml", delete=False
        ) as f:
            f.write(
                "suite_path: /tmp/test_suite\n"
                "context_name: test_context\n"
                "tool_name: mytool\n"
                "prefix_char: +\n"
            )
            tmpfile = f.name

        try:
            wrapper = Wrapper(tmpfile)
            assert wrapper.filepath == tmpfile
            assert wrapper.tool_name == "mytool"
            assert wrapper.context_name == "test_context"
        finally:
            os.unlink(tmpfile)


class TestWrapperProperties:
    """Test Wrapper property access."""

    def test_filepath_property(self):
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".yaml", delete=False
        ) as f:
            f.write("suite_path: /tmp/test\n")
            tmpfile = f.name

        try:
            wrapper = Wrapper(tmpfile)
            assert wrapper.filepath == tmpfile
        finally:
            os.unlink(tmpfile)

    def test_tool_name_none(self):
        """Wrapper with missing tool_name should return None."""
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".yaml", delete=False
        ) as f:
            f.write("suite_path: /tmp/test\n")
            tmpfile = f.name

        try:
            wrapper = Wrapper(tmpfile)
            assert wrapper.tool_name is None
        finally:
            os.unlink(tmpfile)

    def test_suite_cached_property(self):
        """Suite property should be cached after first access."""
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".yaml", delete=False
        ) as f:
            f.write("suite_path: /nonexistent_suite\n")
            tmpfile = f.name

        try:
            wrapper = Wrapper(tmpfile)
            # Suite should raise on first access (path doesn't exist)
            from rez_next.exceptions import RezSystemError

            with pytest.raises(RezSystemError):
                _ = wrapper.suite
        finally:
            os.unlink(tmpfile)

    def test_repr(self):
        """Wrapper repr should include filepath and tool name."""
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".yaml", delete=False
        ) as f:
            f.write(
                "suite_path: /tmp/test\n" 'tool_name: "test-tool"\n'
            )
            tmpfile = f.name

        try:
            wrapper = Wrapper(tmpfile)
            repr_str = repr(wrapper)
            assert "test-tool" in repr_str
            assert os.path.basename(tmpfile) in repr_str
        finally:
            os.unlink(tmpfile)


class TestWrapperServiceMethods:
    """Test Wrapper.print_about(), print_package_versions(), peek()."""

    def test_print_about_returns_zero(self):
        """print_about should always return 0."""
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".yaml", delete=False
        ) as f:
            f.write("suite_path: /tmp/test\n")
            tmpfile = f.name

        try:
            wrapper = Wrapper(tmpfile)
            assert wrapper.print_about() == 0
        finally:
            os.unlink(tmpfile)

    def test_print_package_versions_no_context(self):
        """print_package_versions should return 1 with no context."""
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".yaml", delete=False
        ) as f:
            f.write("suite_path: /tmp/test\n")
            tmpfile = f.name

        try:
            wrapper = Wrapper(tmpfile)
            assert wrapper.print_package_versions() == 1
        finally:
            os.unlink(tmpfile)

    def test_peek_no_context(self):
        """peek should still return 0 with no context."""
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".yaml", delete=False
        ) as f:
            f.write("suite_path: /tmp/test\n")
            tmpfile = f.name

        try:
            wrapper = Wrapper(tmpfile)
            assert wrapper.peek() == 0
        finally:
            os.unlink(tmpfile)
