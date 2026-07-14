"""Tests for rez_next.utils.formatting module."""

from __future__ import annotations

import pytest
from rez_next.utils.formatting import (
    columnise,
    header_line,
    truncate,
    format_table,
)


class TestColumnise:
    """Tests for columnise function."""

    def test_empty(self):
        assert columnise([]) == ""

    def test_single_item(self):
        result = columnise(["hello"], width=80)
        assert result == "hello"

    def test_multiple_items_single_column(self):
        result = columnise(["a", "b", "c"], width=5, padding=1)
        assert "a" in result
        assert "b" in result
        assert "c" in result

    def test_multiple_items_multi_column(self):
        items = ["a", "b", "c", "d"]
        result = columnise(items, width=80, padding=2)
        # All items appear in output
        for item in items:
            assert item in result


class TestHeaderLine:
    """Tests for header_line function."""

    def test_with_label(self):
        result = header_line("test", char="-", width=20)
        assert " test " in result
        assert len(result) == 20

    def test_without_label(self):
        result = header_line("", char="-", width=10)
        assert result == "----------"

    def test_custom_char(self):
        result = header_line("x", char="=", width=10)
        assert " x " in result


class TestTruncate:
    """Tests for truncate function."""

    def test_short_string(self):
        assert truncate("hello", 10) == "hello"

    def test_exact_fit(self):
        assert truncate("hello", 5) == "hello"

    def test_truncated_with_default_suffix(self):
        result = truncate("hello world", 8)
        assert result == "hello..."

    def test_truncated_with_custom_suffix(self):
        result = truncate("hello world", 8, suffix="!!")
        assert result == "hello !!"

    def test_max_len_zero(self):
        result = truncate("hello", 0)
        assert result == ""

    def test_max_len_smaller_than_suffix(self):
        result = truncate("hello", 2)
        assert result == "he"


class TestFormatTable:
    """Tests for format_table function."""

    def test_empty(self):
        assert format_table([]) == ""

    def test_no_headers(self):
        result = format_table([["a", "1"], ["b", "2"]], col_sep="  ")
        assert "a  1" in result
        assert "b  2" in result

    def test_with_headers(self):
        result = format_table(
            [["a", "1"], ["b", "2"]],
            headers=["Name", "Val"],
        )
        assert "Name" in result
        assert "Val" in result
        # Should have a separator under header
        assert "----" in result

    def test_varying_lengths(self):
        result = format_table(
            [["short", "x"], ["very_long_name", "y"]],
        )
        assert "short" in result
        assert "very_long_name" in result
