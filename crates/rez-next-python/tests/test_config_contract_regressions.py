"""Regression contracts observed against upstream Rez 3.4.0."""

from rez_next.build_process import BuildProcess, create_build_process
from rez_next.config import Config


def test_python_file_values_are_loaded_and_reported(tmp_path):
    path = tmp_path / "custom.py"
    path.write_text(
        "build_directory = 'custom-build'\nrelease_packages_path = '/tmp/release-contract'\n"
    )
    config = Config([str(path)])
    assert config.build_directory == "custom-build"
    assert config.release_packages_path == "/tmp/release-contract"
    assert config.sourced_filepaths == [str(path)]


def test_overrides_are_applied_and_can_be_changed():
    config = Config([], overrides={"build_directory": "first"})
    assert config.build_directory == "first"
    config.override("build_directory", "second")
    assert config.build_directory == "second"
    assert config.sourced_filepaths == []


def test_copied_overrides_do_not_change_original():
    original = Config([], overrides={"build_directory": "original"})
    assert original.build_directory == "original"
    copied = original.copy(overrides={"build_directory": "copy"})
    assert copied.build_directory == "copy"
    assert original.build_directory == "original"


def test_layers_deep_merge_and_later_scalar_wins(tmp_path):
    first = tmp_path / "first.py"
    second = tmp_path / "second.py"
    first.write_text(
        "build_directory = 'first'\nplugins = {'build_system': {'custom': {'one': 1}}}\n"
    )
    second.write_text(
        "build_directory = 'second'\nplugins = {'build_system': {'custom': {'two': 2}}}\n"
    )
    config = Config([str(first), str(second)])
    assert config.build_directory == "second"
    assert config.plugins.data()["build_system"]["custom"] == {"one": 1, "two": 2}
    assert config.sourced_filepaths == [str(first), str(second)]


def test_factory_accepts_upstream_positional_and_keyword_arguments():
    class ContractProcess(BuildProcess):
        name = "contract_test"

        def build(self, **kwargs):
            return 1

        def release(self, **kwargs):
            return 1

    build_system = object()
    try:
        positional = create_build_process("contract_test", "/tmp/contract", build_system, None)
        keyword = create_build_process(
            "contract_test", working_dir="/tmp/contract", build_system=build_system, package=None
        )
        assert positional._build_system is build_system
        assert keyword._build_system is build_system
        assert positional.working_dir == "/tmp/contract"
        assert keyword.working_dir == "/tmp/contract"
    finally:
        BuildProcess._registry.pop("contract_test", None)


def test_bundled_default_file_can_be_sourced():
    from rez_next.config import get_module_root_config

    path = get_module_root_config()
    config = Config([path])
    assert config.sourced_filepaths == [path]
    assert config.build_directory == "build"


def test_get_honors_object_overrides_before_native_global_config():
    class NativeDefaults:
        def contains_key(self, key):
            return True

        def get_string(self, key):
            return "native-global-default"

        get_int = get_float = get_bool = get_string

    config = Config([], overrides={"build_directory": "object-value"})
    config._native_cfg = NativeDefaults()
    assert config.get("build_directory") == "object-value"


def test_configured_package_paths_expand_home(monkeypatch, tmp_path):
    monkeypatch.setenv("HOME", str(tmp_path))
    monkeypatch.setenv("USERPROFILE", str(tmp_path))
    monkeypatch.delenv("REZ_PACKAGES_PATH", raising=False)
    monkeypatch.delenv("REZ_PACKAGES_PATH_JSON", raising=False)
    config = Config([], overrides={"packages_path": ["~/packages"]})
    # expanduser retains the separator after '~', including on Windows.
    assert config.packages_path == [f"{tmp_path}/packages"]


def test_top_level_and_module_share_config_singleton():
    import sys

    import rez_next

    assert rez_next.config is sys.modules["rez_next.config"].config


def test_context_rejects_missing_required_family(tmp_path):
    import pytest
    from rez_next.resolved_context import ResolvedContext

    with pytest.raises(RuntimeError, match="missing_contract_family"):
        ResolvedContext(["missing_contract_family"], paths=[str(tmp_path)])


def test_context_rejects_missing_transitive_dependency(tmp_path):
    import pytest
    from rez_next.resolved_context import ResolvedContext

    package = tmp_path / "tool" / "1.0"
    package.mkdir(parents=True)
    (package / "package.py").write_text(
        "name='tool'\nversion='1.0'\nrequires=['missing_contract_dependency']\n"
    )
    with pytest.raises(RuntimeError, match="missing_contract_dependency"):
        ResolvedContext(["tool"], paths=[str(tmp_path)])
