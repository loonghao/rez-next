"""Example Rez package using rez_next.build_plugins.PipFromDownloadBuilder."""

name = "pypi_sample"
version = "1.0.0"
description = "Example package installed from a pip-compatible wheel"

requires = ["python-3+"]
build_requires = ["python-3+"]

tests = {
    "import": "python -c \"import pypi_sample; print(pypi_sample.VALUE)\"",
}


def commands():
    env.PYTHONPATH.prepend("{root}/python")
