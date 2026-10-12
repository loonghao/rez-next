"""Both native config entry points obey presence-based environment precedence."""

import os

import pytest
from rez_next._native.config import Config, load_config
from rez_next.config import Config as ConfigFacade


@pytest.fixture(autouse=True)
def isolated_config_environment(monkeypatch, tmp_path):
    for name in list(os.environ):
        if name.startswith("REZ_"):
            monkeypatch.delenv(name)
    monkeypatch.setenv("HOME", str(tmp_path))
    monkeypatch.setenv("USERPROFILE", str(tmp_path))
    monkeypatch.chdir(tmp_path)


@pytest.mark.parametrize("loader", [Config.load, load_config], ids=["Config.load", "load_config"])
@pytest.mark.parametrize(
    "key,plain,json_value,getter,expected",
    [
        ("default_shell", "plain", '"json"', "get_string", "plain"),
        ("default_shell", "", '"json"', "get_string", ""),
        ("use_rust_solver", "false", "true", "get_bool", False),
        ("thread_count", "0", "4", "get_int", 0),
        ("default_shell", "plain", "not-json", "get_string", "plain"),
        ("default_shell", "", "not-json", "get_string", ""),
        ("default_shell", None, '"json"', "get_string", "json"),
        ("use_rust_solver", None, "false", "get_bool", False),
        ("thread_count", None, "0", "get_int", 0),
        ("plugins.shell.default", "plain", "not-json", "get_string", "plain"),
    ],
)
def test_native_environment_precedence(
    monkeypatch, loader, key, plain, json_value, getter, expected
):
    variable = f"REZ_{key.upper()}"
    if plain is not None:
        monkeypatch.setenv(variable, plain)
    monkeypatch.setenv(f"{variable}_JSON", json_value)
    config = loader()
    assert getattr(config, getter)(key) == expected


@pytest.mark.parametrize("loader", [Config.load, load_config], ids=["Config.load", "load_config"])
def test_native_unmasked_invalid_json_still_errors(monkeypatch, loader):
    monkeypatch.setenv("REZ_DEFAULT_SHELL_JSON", "not-json")
    with pytest.raises(ValueError, match="REZ_DEFAULT_SHELL_JSON"):
        loader()


@pytest.mark.parametrize("json_value", ["4", "not-json"])
def test_facade_native_key_uses_plain_override(monkeypatch, json_value):
    monkeypatch.setenv("REZ_THREAD_COUNT", "0")
    monkeypatch.setenv("REZ_THREAD_COUNT_JSON", json_value)
    assert ConfigFacade([]).get("thread_count") == 0
