"""Python standalone package fixture for build-environment resolution tests."""

name = "python"
version = "3.11.0"
build_system = "binary_archive"
tools = ["python"]


def commands():
    env.setenv("PYTHON_SENTINEL", "from-python-fixture")
    env.setenv("PATHEXT", ".COM;.EXE;.BAT;.CMD")
    env.PATH.prepend("{root}/bin")
