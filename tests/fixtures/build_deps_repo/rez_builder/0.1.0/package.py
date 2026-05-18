"""Minimal rez_builder-like package fixture used by vx build tests."""

name = "rez_builder"
version = "0.1.0"
requires = ["python-3+"]


def commands():
    env.setenv("REZ_BUILDER_SENTINEL", "from-rez-builder-fixture")
    env.setenv("REZ_BUILDER_MODULE_PATH", "{root}/python")
