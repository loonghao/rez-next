# SPDX-License-Identifier: Apache-2.0
# Copyright Contributors to the Rez Project


"""Tests for rez_next.utils.loggging_ module."""

import logging
import pytest


from rez_next.utils.loggging_ import (
    print_debug,
    print_info,
    print_warning,
    print_error,
    print_critical,
    get_debug_printer,
    get_info_printer,
    get_warning_printer,
    get_error_printer,
    get_critical_printer,
    _Printer,
    log_duration,
)


class TestPrintFunctions:
    """Test the print_* functions."""

    def test_print_debug(self, caplog):
        """Test print_debug function."""
        with caplog.at_level(logging.DEBUG, logger="rez_next.utils.loggging_"):
            print_debug("test message %s", "arg1")

        assert "test message arg1" in caplog.text

    def test_print_debug_no_args(self, caplog):
        """Test print_debug with no extra args."""
        with caplog.at_level(logging.DEBUG, logger="rez_next.utils.loggging_"):
            print_debug("test message")

        assert "test message" in caplog.text

    def test_print_info(self, caplog):
        """Test print_info function."""
        with caplog.at_level(logging.INFO, logger="rez_next.utils.loggging_"):
            print_info("info message %s", "arg1")

        assert "info message arg1" in caplog.text

    def test_print_warning(self, caplog):
        """Test print_warning function."""
        with caplog.at_level(logging.WARNING, logger="rez_next.utils.loggging_"):
            print_warning("warning message %s", "arg1")

        assert "warning message arg1" in caplog.text

    def test_print_error(self, caplog):
        """Test print_error function."""
        with caplog.at_level(logging.ERROR, logger="rez_next.utils.loggging_"):
            print_error("error message %s", "arg1")

        assert "error message arg1" in caplog.text

    def test_print_critical(self, caplog):
        """Test print_critical function."""
        with caplog.at_level(logging.CRITICAL, logger="rez_next.utils.loggging_"):
            print_critical("critical message %s", "arg1")

        assert "critical message arg1" in caplog.text


class TestGetPrinterFunctions:
    """Test the get_*_printer functions."""

    def test_get_debug_printer_enabled(self, caplog):
        """Test get_debug_printer with enabled=True."""
        printer = get_debug_printer(enabled=True)
        assert printer

        with caplog.at_level(logging.DEBUG, logger="rez_next.utils.loggging_"):
            printer("test %s", "msg")

        assert "test msg" in caplog.text

    def test_get_debug_printer_disabled(self, caplog):
        """Test get_debug_printer with enabled=False."""
        printer = get_debug_printer(enabled=False)
        assert not printer

        with caplog.at_level(logging.DEBUG, logger="rez_next.utils.loggging_"):
            printer("test %s", "msg")

        assert "test msg" not in caplog.text

    def test_get_info_printer_enabled(self, caplog):
        """Test get_info_printer with enabled=True."""
        printer = get_info_printer(enabled=True)
        assert printer

        with caplog.at_level(logging.INFO, logger="rez_next.utils.loggging_"):
            printer("test %s", "msg")

        assert "test msg" in caplog.text

    def test_get_warning_printer_enabled(self, caplog):
        """Test get_warning_printer with enabled=True."""
        printer = get_warning_printer(enabled=True)
        assert printer

        with caplog.at_level(logging.WARNING, logger="rez_next.utils.loggging_"):
            printer("test %s", "msg")

        assert "test msg" in caplog.text

    def test_get_error_printer_enabled(self, caplog):
        """Test get_error_printer with enabled=True."""
        printer = get_error_printer(enabled=True)
        assert printer

        with caplog.at_level(logging.ERROR, logger="rez_next.utils.loggging_"):
            printer("test %s", "msg")

        assert "test msg" in caplog.text

    def test_get_critical_printer_enabled(self, caplog):
        """Test get_critical_printer with enabled=True."""
        printer = get_critical_printer(enabled=True)
        assert printer

        with caplog.at_level(logging.CRITICAL, logger="rez_next.utils.loggging_"):
            printer("test %s", "msg")

        assert "test msg" in caplog.text


class TestPrinter:
    """Test the _Printer class."""

    def test_printer_callable_enabled(self, caplog):
        """Test _Printer.__call__ when enabled."""
        printer = _Printer(enabled=True, printer_function=logging.getLogger().debug)
        with caplog.at_level(logging.DEBUG):
            printer("test %s", "msg")

        assert "test msg" in caplog.text

    def test_printer_callable_disabled(self, caplog):
        """Test _Printer.__call__ when disabled."""
        printer = _Printer(enabled=False, printer_function=logging.getLogger().debug)
        with caplog.at_level(logging.DEBUG):
            printer("test %s", "msg")

        assert "test msg" not in caplog.text

    def test_printer_bool_enabled(self):
        """Test _Printer.__bool__ when enabled."""
        printer = _Printer(enabled=True, printer_function=logging.getLogger().debug)
        assert printer

    def test_printer_bool_disabled(self):
        """Test _Printer.__bool__ when disabled."""
        printer = _Printer(enabled=False, printer_function=logging.getLogger().debug)
        assert not printer

    def test_printer_format_string(self, caplog):
        """Test that _Printer formats %-style strings."""
        printer = _Printer(enabled=True, printer_function=logging.getLogger().debug)
        with caplog.at_level(logging.DEBUG):
            printer("value=%d", 42)

        assert "value=42" in caplog.text


class TestLogDuration:
    """Test the log_duration context manager."""

    def test_log_duration(self, caplog):
        """Test that log_duration logs the duration."""
        printer = get_debug_printer(enabled=True)

        with caplog.at_level(logging.DEBUG, logger="rez_next.utils.loggging_"):
            with log_duration(printer, "duration: %s"):
                pass

        # Should have logged something with "duration:"
        assert "duration:" in caplog.text


class TestImportFromSolver:
    """Test that print_debug can be imported from rez_next.solver."""

    def test_import_print_debug_from_solver(self):
        """Test importing print_debug from rez_next.solver."""
        from rez_next.solver import print_debug as solver_print_debug

        assert solver_print_debug is not None

    def test_print_debug_works_from_solver_import(self, caplog):
        """Test that print_debug works when imported from rez_next.solver."""
        from rez_next.solver import print_debug as solver_print_debug

        with caplog.at_level(logging.DEBUG, logger="rez_next.utils.loggging_"):
            solver_print_debug("test from solver %s", "import")

        assert "test from solver import" in caplog.text
