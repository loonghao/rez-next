"""Tests for rez_next.utils.logging_ module."""

from __future__ import annotations

import logging
import io

import pytest
from rez_next.utils.logging_ import (
    _Printer,
    get_debug_printer,
    get_info_printer,
    get_warning_printer,
    get_error_printer,
    get_critical_printer,
    log_duration,
    print_debug,
    print_info,
    print_warning,
    print_error,
    print_critical,
)


class TestPrinter:
    """Tests for _Printer class."""

    def test_enabled_printer_calls_function(self):
        """Enabled printer should call the underlying function."""
        calls = []
        def fake_log(msg):
            calls.append(msg)

        p = _Printer(enabled=True, printer_function=fake_log)
        p("hello %s", "world")
        assert calls == ["hello world"]

    def test_enabled_printer_without_args(self):
        """Enabled printer should call without formatting."""
        calls = []
        def fake_log(msg):
            calls.append(msg)

        p = _Printer(enabled=True, printer_function=fake_log)
        p("hello")
        assert calls == ["hello"]

    def test_disabled_printer_noop(self):
        """Disabled printer should not call the function."""
        calls = []
        def fake_log(msg):
            calls.append(msg)

        p = _Printer(enabled=False, printer_function=fake_log)
        p("hello")
        assert calls == []

    def test_bool_enabled(self):
        """__bool__ should return True when enabled."""
        p = _Printer(enabled=True, printer_function=lambda msg: None)
        assert bool(p) is True

    def test_bool_disabled(self):
        """__bool__ should return False when disabled."""
        p = _Printer(enabled=False, printer_function=lambda msg: None)
        assert bool(p) is False

    def test_with_none_printer_function(self):
        """Enabled=True but no function should not crash."""
        p = _Printer(enabled=True, printer_function=None)
        bool(p) is False
        p("msg")


class TestGetPrinters:
    """Tests for get_*_printer factory functions."""

    def test_get_debug_printer_default(self):
        printer = get_debug_printer()
        assert bool(printer) is True

    def test_get_info_printer_disabled(self):
        printer = get_info_printer(enabled=False)
        assert bool(printer) is False

    def test_get_warning_printer_calls(self):
        calls = []
        original = logging.getLogger("rez_next.utils.logging_").warning
        try:
            logging.getLogger("rez_next.utils.logging_").warning = lambda msg: calls.append(msg)
            p = get_warning_printer(enabled=True)
            p("warn %d", 42)
            assert len(calls) == 1
            assert "42" in calls[0]
        finally:
            logging.getLogger("rez_next.utils.logging_").warning = original

    def test_get_error_printer_exists(self):
        p = get_error_printer()
        assert callable(p)

    def test_get_critical_printer_exists(self):
        p = get_critical_printer()
        assert callable(p)


class TestConvenienceFunctions:
    """Tests for print_* convenience functions."""

    def test_print_debug(self, caplog):
        with caplog.at_level(logging.DEBUG):
            print_debug("test debug %d", 1)
        assert len(caplog.records) >= 1

    def test_print_info(self, caplog):
        with caplog.at_level(logging.INFO):
            print_info("test info")
        assert any("test info" in rec.message for rec in caplog.records)

    def test_print_warning(self, caplog):
        with caplog.at_level(logging.WARNING):
            print_warning("test warning")
        assert any("test warning" in rec.message for rec in caplog.records)

    def test_print_error(self, caplog):
        with caplog.at_level(logging.ERROR):
            print_error("test error")
        assert any("test error" in rec.message for rec in caplog.records)

    def test_print_critical(self, caplog):
        with caplog.at_level(logging.CRITICAL):
            print_critical("test critical")
        assert any("test critical" in rec.message for rec in caplog.records)


class TestLogDuration:
    """Tests for log_duration context manager."""

    def test_log_duration_emits_message(self):
        calls = []
        p = _Printer(enabled=True, printer_function=calls.append)
        with log_duration(p, "took %s sec"):
            pass
        assert len(calls) == 1
        assert "took " in calls[0]

    def test_log_duration_disabled(self):
        calls = []
        p = _Printer(enabled=False, printer_function=calls.append)
        with log_duration(p, "took %s sec"):
            pass
        assert len(calls) == 0
