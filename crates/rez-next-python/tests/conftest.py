"""
Shared pytest helpers for rez_next Python test suite.

Import in test modules with::

    from conftest import write_package_py
"""


def write_package_py(path, name, version, requires=None, commands=None):
    """Write a minimal package.py to *path* directory.

    Parameters
    ----------
    path : pathlib.Path
        Directory in which ``package.py`` will be written (created if needed).
    name : str
        Package name field.
    version : str
        Package version field.
    requires : list[str], optional
        List of requirement strings (e.g. ``["python-3+"]``).
    commands : str, optional
        Raw Rex command block to embed verbatim.
    """
    path.mkdir(parents=True, exist_ok=True)
    lines = [f'name = "{name}"', f'version = "{version}"']
    if requires:
        req_list = ", ".join(f'"{r}"' for r in requires)
        lines.append(f"requires = [{req_list}]")
    if commands:
        lines.append(f'commands = """\n{commands}\n"""')
    (path / "package.py").write_text("\n".join(lines) + "\n")
