"""Rez package fixture for vx-style binary extraction builds."""

name = "vx"
version = "0.0.1"
description = "Universal Development Tool Manager test fixture."
build_system = "binary_archive"
tools = ["vx"]
requires = ["python-3+"]
build_requires = ["python-3+"]
private_build_requires = ["rez_builder-0"]
tests = {
    "artifact": "python tests/check_artifact.py",
    "zipfile": "python tests/check_zipfile.py",
}


def commands():
    env.PATH.prepend("{root}/bin")
