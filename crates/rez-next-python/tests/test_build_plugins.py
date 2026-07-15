"""Production contracts for reusable Python build plugins."""

import os
import zipfile

import pytest
from rez_next.build_plugins import copy_tree_contents, extract_archive


@pytest.mark.skipif(os.name == "nt", reason="Unix executable permissions only")
def test_zip_install_preserves_executable_permissions(tmp_path):
    archive_path = tmp_path / "tools.zip"
    with zipfile.ZipFile(archive_path, "w") as archive:
        executable = zipfile.ZipInfo("bin/tool")
        executable.create_system = 3
        executable.external_attr = 0o100755 << 16
        archive.writestr(executable, "#!/bin/sh\nexit 0\n")

    extracted = tmp_path / "extracted"
    installed = tmp_path / "installed"
    extract_archive(archive_path, extracted)
    copy_tree_contents(extracted, installed)

    assert installed.joinpath("bin", "tool").stat().st_mode & 0o111


def test_zip_rejects_members_outside_destination(tmp_path):
    archive_path = tmp_path / "unsafe.zip"
    with zipfile.ZipFile(archive_path, "w") as archive:
        archive.writestr("../outside.txt", "unsafe")

    with pytest.raises(ValueError, match="escapes destination"):
        extract_archive(archive_path, tmp_path / "extracted")

    assert not (tmp_path / "outside.txt").exists()
