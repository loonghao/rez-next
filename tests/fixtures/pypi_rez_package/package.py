"""PyPI-backed Rez package fixture for the built-in pypi plugin."""

name = "pypi_sample"
version = "1.0.0"
description = "Fixture package installed from a pip-compatible wheel"
build_system = "pypi"

requires = ["python-3+"]
build_requires = ["python-3+"]

tests = {
    "import": "python -c \"import pypi_sample; print(pypi_sample.VALUE)\"",
}


def commands():
    env.PYTHONPATH.prepend("{root}/python")
