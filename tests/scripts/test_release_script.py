"""Smoke tests for the `rez-release` vx extension script.

`rez_next` is never imported here: `release_package` is monkeypatched into
`sys.modules`, so these tests cover the script's own logic — source-directory
resolution, argument parsing, and exit-code mapping — on any machine.
"""

from __future__ import annotations

import json
import sys
import types
from typing import Any

import pytest
import release


class FakeResult:
    """Stands in for `rez_next`'s `ReleaseResult`."""

    def __init__(
        self,
        success: bool = True,
        package_name: str = "pkg",
        version: str = "1.0.0",
        install_path: str = "/install/pkg/1.0.0",
        vcs_metadata: str | None = None,
        changelog: str | None = None,
        errors: list[str] | None = None,
        warnings: list[str] | None = None,
    ) -> None:
        self.success = success
        self.package_name = package_name
        self.version = version
        self.install_path = install_path
        self.vcs_metadata = vcs_metadata
        self.changelog = changelog
        self.errors = errors or []
        self.warnings = warnings or []


@pytest.fixture
def recorded(monkeypatch: pytest.MonkeyPatch) -> dict[str, Any]:
    """Install a fake `rez_next.release` and record how it was called.

    Returns a dict that is populated with `result`, `raises`, and `calls`.
    The test mutates it before invoking `main`.
    """
    state: dict[str, Any] = {
        "result": FakeResult(),
        "raises": None,
        "calls": [],
    }

    def fake_release_package(*args: Any, **kwargs: Any) -> FakeResult:
        state["calls"].append((args, kwargs))
        if state["raises"] is not None:
            raise state["raises"]
        return state["result"]

    fake_module = types.ModuleType("rez_next")
    fake_release = types.ModuleType("rez_next.release")
    fake_release.release_package = fake_release_package
    fake_module.release = fake_release
    monkeypatch.setitem(sys.modules, "rez_next", fake_module)
    monkeypatch.setitem(sys.modules, "rez_next.release", fake_release)
    return state


# ── resolve_source_dir ──────────────────────────────────────────────────────


def test_resolve_source_dir_prefers_argument(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("VX_PROJECT_DIR", "/from/env")
    assert release.resolve_source_dir("/explicit") == ("/explicit", "argument")


def test_resolve_source_dir_falls_back_to_vx_project_dir(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setenv("VX_PROJECT_DIR", "/from/env")
    assert release.resolve_source_dir(None) == ("/from/env", "VX_PROJECT_DIR")


def test_resolve_source_dir_falls_back_to_cwd(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.delenv("VX_PROJECT_DIR", raising=False)
    _, origin = release.resolve_source_dir(None)
    assert origin == "current directory"


# ── argument parsing ────────────────────────────────────────────────────────


def test_ignore_existing_tag_defaults_to_false() -> None:
    args = release.build_parser().parse_args([])
    assert args.ignore_existing_tag is False


def test_ignore_existing_tag_flag_is_parsed() -> None:
    args = release.build_parser().parse_args(["--ignore-existing-tag"])
    assert args.ignore_existing_tag is True


# ── exit-code mapping ───────────────────────────────────────────────────────


def test_missing_directory_returns_usage_exit(
    recorded: dict[str, Any], capsys: pytest.CaptureFixture[str]
) -> None:
    exit_code = release.main(["/definitely/not/a/directory"])
    assert exit_code == release.EXIT_USAGE
    assert "not found" in capsys.readouterr().err


def test_success_returns_zero(
    recorded: dict[str, Any], tmp_path, capsys: pytest.CaptureFixture[str]
) -> None:
    exit_code = release.main([str(tmp_path)])
    assert exit_code == release.EXIT_OK
    assert "Released" in capsys.readouterr().out


def test_release_errors_return_one(
    recorded: dict[str, Any], tmp_path, capsys: pytest.CaptureFixture[str]
) -> None:
    recorded["result"] = FakeResult(
        success=False, errors=["No package.py or package.yaml found"]
    )
    exit_code = release.main([str(tmp_path)])
    assert exit_code == release.EXIT_RELEASE_FAILED
    assert "No package.py" in capsys.readouterr().err


def test_runtime_error_returns_one_not_four(
    recorded: dict[str, Any], tmp_path, capsys: pytest.CaptureFixture[str]
) -> None:
    """Build/test failures must be exit 1, not the removed "unexpected" 4."""
    recorded["raises"] = RuntimeError("build failed")
    exit_code = release.main([str(tmp_path)])
    assert exit_code == release.EXIT_RELEASE_FAILED
    assert "build failed" in capsys.readouterr().err


def test_exit_code_four_no_longer_exists() -> None:
    assert not hasattr(release, "EXIT_UNEXPECTED")


# ── tag policy is forwarded ─────────────────────────────────────────────────


def test_strict_tag_policy_is_forwarded_by_default(
    recorded: dict[str, Any], tmp_path
) -> None:
    release.main([str(tmp_path)])
    args, _ = recorded["calls"][0]
    # (source_dir, local, dry_run, message, ignore_existing_tag)
    assert args[4] is False


def test_ignore_existing_tag_is_forwarded_when_requested(
    recorded: dict[str, Any], tmp_path
) -> None:
    release.main([str(tmp_path), "--ignore-existing-tag"])
    args, _ = recorded["calls"][0]
    assert args[4] is True


# ── --json output ───────────────────────────────────────────────────────────


def test_json_output_decodes_vcs_metadata(
    recorded: dict[str, Any], tmp_path, capsys: pytest.CaptureFixture[str]
) -> None:
    recorded["result"] = FakeResult(vcs_metadata=json.dumps({"commit_hash": "abc123"}))
    assert release.main([str(tmp_path), "--json"]) == release.EXIT_OK
    payload = json.loads(capsys.readouterr().out)
    assert payload["vcs_metadata"] == {"commit_hash": "abc123"}


def test_json_output_preserves_unparseable_vcs_metadata(
    recorded: dict[str, Any], tmp_path, capsys: pytest.CaptureFixture[str]
) -> None:
    recorded["result"] = FakeResult(vcs_metadata="not json")
    release.main([str(tmp_path), "--json"])
    payload = json.loads(capsys.readouterr().out)
    assert payload["vcs_metadata"] == "not json"


def test_json_output_reports_failure_exit_code(
    recorded: dict[str, Any], tmp_path, capsys: pytest.CaptureFixture[str]
) -> None:
    recorded["result"] = FakeResult(success=False, errors=["boom"])
    assert release.main([str(tmp_path), "--json"]) == release.EXIT_RELEASE_FAILED
    assert json.loads(capsys.readouterr().out)["errors"] == ["boom"]
