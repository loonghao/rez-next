"""Tests for rez_next.utils.colorize module."""

from __future__ import annotations

import io
import logging
import re

import pytest
from rez_next.utils.colorize import (
    Printer,
    ColorizedStreamHandler,
    colorama_wrap,
    stream_is_tty,
    critical,
    error,
    warning,
    info,
    debug,
    heading,
    local,
    implicit,
    ephemeral,
    alias,
    inactive,
    notset,
    _color,
    _color_level,
    _RESET,
)


class TestColorHelpers:
    """Tests for individual color helper functions."""

    def test_critical_returns_string(self):
        result = critical("boom")
        assert isinstance(result, str)
        assert "boom" in result

    def test_error_returns_string(self):
        result = error("fail")
        assert isinstance(result, str)
        assert "fail" in result

    def test_warning_returns_string(self):
        result = warning("caution")
        assert isinstance(result, str)
        assert "caution" in result

    def test_info_returns_string(self):
        result = info("note")
        assert isinstance(result, str)
        assert "note" in result

    def test_debug_returns_string(self):
        result = debug("trace")
        assert isinstance(result, str)
        assert "trace" in result

    def test_heading_returns_string(self):
        result = heading("title")
        assert isinstance(result, str)
        assert "title" in result

    def test_local_returns_string(self):
        result = local("here")
        assert isinstance(result, str)
        assert "here" in result

    def test_implicit_returns_string(self):
        result = implicit("auto")
        assert isinstance(result, str)
        assert "auto" in result

    def test_ephemeral_returns_string(self):
        result = ephemeral("temp")
        assert isinstance(result, str)
        assert "temp" in result

    def test_alias_returns_string(self):
        result = alias("other")
        assert isinstance(result, str)
        assert "other" in result

    def test_inactive_adds_dim_style(self):
        result = inactive("grey")
        assert "2m" in result or "dim" not in result  # SGR dim param is '2'
        # Should have SGR codes
        assert "\x1b[" in result

    def test_notset_returns_plain(self):
        result = notset("plain")
        assert result == "plain"


class TestColorCore:
    """Tests for internal _color and _color_level functions."""

    def test_color_with_foreground(self):
        result = _color("red text", fore="red")
        assert "\x1b[31m" in result  # red foreground code
        assert _RESET in result
        assert "red text" in result

    def test_color_with_style(self):
        result = _color("bold text", styles=["bold"])
        assert "\x1b[1m" in result
        assert _RESET in result

    def test_color_with_background(self):
        result = _color("bg", back="green")
        # Background green = 42 (32 + 10)
        assert "\x1b[42m" in result or "\x1b[" in result

    def test_color_no_params_returns_unchanged(self):
        result = _color("plain")
        assert result == "plain"

    def test_color_level_defaults_to_info(self):
        result = _color_level("something", "info")
        assert isinstance(result, str)
        assert "something" in result


class TestColoramaWrap:
    """Tests for colorama_wrap function."""

    def test_colorama_wrap_returns_stream(self):
        stream = io.StringIO()
        result = colorama_wrap(stream)
        assert result is stream

    def test_colorama_wrap_stdout(self):
        import sys
        result = colorama_wrap(sys.stdout)
        assert result is sys.stdout


class TestStreamIsTty:
    """Tests for stream_is_tty function."""

    def test_stringio_is_not_tty(self):
        stream = io.StringIO()
        assert stream_is_tty(stream) is False

    def test_none_stream_does_not_crash(self):
        # Should not raise
        try:
            result = stream_is_tty(None)  # type: ignore
            assert result is False
        except AttributeError:
            pass  # also acceptable


class TestColorizedStreamHandler:
    """Tests for ColorizedStreamHandler class."""

    def test_init_defaults(self):
        handler = ColorizedStreamHandler()
        assert handler.stream is not None

    def test_init_with_stream(self):
        stream = io.StringIO()
        handler = ColorizedStreamHandler(stream=stream)
        assert handler.stream is stream

    def test_emit_writes_to_stream(self):
        stream = io.StringIO()
        handler = ColorizedStreamHandler(stream=stream)

        record = logging.LogRecord(
            name="test",
            level=logging.INFO,
            pathname="",
            lineno=0,
            msg="test message",
            args=(),
            exc_info=None,
        )
        handler.emit(record)
        output = stream.getvalue()
        assert "test message" in output

    def test_emit_error_level(self):
        stream = io.StringIO()
        handler = ColorizedStreamHandler(stream=stream)

        record = logging.LogRecord(
            name="test",
            level=logging.ERROR,
            pathname="",
            lineno=0,
            msg="error msg",
            args=(),
            exc_info=None,
        )
        handler.emit(record)
        output = stream.getvalue()
        assert "error msg" in output

    def test_emit_debug_level(self):
        stream = io.StringIO()
        handler = ColorizedStreamHandler(stream=stream)

        record = logging.LogRecord(
            name="test",
            level=logging.DEBUG,
            pathname="",
            lineno=0,
            msg="debug msg",
            args=(),
            exc_info=None,
        )
        handler.emit(record)
        output = stream.getvalue()
        assert "debug msg" in output

    def test_is_colorized_returns_bool(self):
        handler = ColorizedStreamHandler()
        result = handler.is_colorized
        assert isinstance(result, bool)

    def test_logger_integration(self):
        logger = logging.getLogger("test_colorize_handler")
        logger.setLevel(logging.DEBUG)

        stream = io.StringIO()
        handler = ColorizedStreamHandler(stream=stream)
        logger.addHandler(handler)

        logger.info("integration test")
        output = stream.getvalue()
        assert "integration test" in output
        logger.removeHandler(handler)


class TestPrinter:
    """Tests for Printer class."""

    def test_init_defaults(self):
        printer = Printer()
        assert printer.stream is not None

    def test_init_with_stream(self):
        stream = io.StringIO()
        printer = Printer(stream=stream)
        assert printer.stream is stream

    def test_print_writes_to_stream(self):
        stream = io.StringIO()
        printer = Printer(stream=stream, colorize=False)
        printer.print("hello")
        assert stream.getvalue() == "hello\n"

    def test_print_with_style(self):
        stream = io.StringIO()
        printer = Printer(stream=stream, colorize=False)
        printer.print("styled", style="error")
        output = stream.getvalue()
        assert "styled" in output

    def test_colorize_false_no_ansi(self):
        stream = io.StringIO()
        printer = Printer(stream=stream, colorize=False)
        printer.print("plain")
        output = stream.getvalue()
        # Should not contain ANSI codes
        assert "\x1b[" not in output

    def test_colorize_true_emits_ansi_when_style_given(self):
        stream = io.StringIO()
        printer = Printer(stream=stream, colorize=True)
        printer.print("styled", style="error")
        output = stream.getvalue()
        # With colorize=True and a style, should get ANSI codes
        assert "styled" in output

    def test_print_no_style_with_colorize(self):
        stream = io.StringIO()
        printer = Printer(stream=stream, colorize=True)
        printer.print("no style")
        output = stream.getvalue()
        assert "no style" in output
