"""
YAML serialisation utilities for Rez-next.

Mirrors `rez.utils.yaml` API:
- dump_yaml(data, ...) — serialise Python data to a YAML string
- load_yaml(filepath)  — load YAML from a file
- save_yaml(filepath, **fields) — write key-value pairs to a YAML file

Design:
  Uses the vendored ``ruamel.yaml`` or falls back to the standard ``yaml``
  (PyYAML) library.  A custom Dumper handles Rez-native types (Version,
  Requirement) and Python function sources so that round-tripping works for
  typical ``package.py`` metadata files.

Resolution order for the YAML library:
  1. ``ruamel.yaml`` (preferred — preserves comments & ordering)
  2. ``yaml`` (PyYAML fallback)

Lessons from Rez issues:
  - Never ``eval()`` or ``exec()`` YAML content (avoid #937-style @late bugs).
  - Always use a ``SafeLoader``/``SafeDumper`` base.
"""
from __future__ import annotations

import inspect
import os
from textwrap import dedent
from types import BuiltinFunctionType, FunctionType
from typing import Any

_A = Any

# ── YAML library resolution ────────────────────────────────────────────────

try:
    import ruamel.yaml as _yaml

    _RUAMEL = True
    _Yaml = _yaml.YAML  # noqa: N816
except ImportError:
    import yaml as _yaml  # type: ignore[no-redef]

    _RUAMEL = False


# ── Custom type support ────────────────────────────────────────────────────

def _try_import_rez_types() -> dict:
    """Lazily import Rez types to avoid circular imports at module level."""
    types = {}
    try:
        import rez_next.version as _v
        types["Version"] = getattr(_v, "Version", None)
        types["Requirement"] = getattr(_v, "Requirement", None)
    except (ImportError, AttributeError):
        pass
    # Direct class-level fallback
    try:
        import rez_next as _r  # noqa: F811
        if types.get("Version") is None:
            types["Version"] = getattr(_r, "Version", None)
        if types.get("Requirement") is None:
            types["Requirement"] = getattr(_r, "Requirement", None)
    except (ImportError, AttributeError):
        pass
    return types


def _represent_rez_type(dumper: _yaml.Dumper, data: Any) -> _yaml.Node:
    """Represent any Rez type (Version, Requirement) as a plain string."""
    return dumper.represent_str(str(data))


def _represent_function(dumper: _yaml.Dumper, data: Any) -> _yaml.Node:
    """Represent a Python function as its source code."""
    try:
        lines = inspect.getsourcelines(data)[0]
        # Strip leading indentation
        code = dedent("".join(lines[1:])) if len(lines) > 1 else ""
        return dumper.represent_str(code)
    except (OSError, TypeError):
        return dumper.represent_str(str(data))


def _represent_builtin(dumper: _yaml.Dumper, data: Any) -> _yaml.Node:
    """Represent a built-in function as its string representation."""
    return dumper.represent_str(str(data))


def _build_dumper(base_dumper: type) -> type:
    """Build a Dumper class with Rez-specific representers."""
    types_map = _try_import_rez_types()

    if not _RUAMEL:
        # PyYAML — subclass SafeDumper
        from yaml.dumper import SafeDumper  # type: ignore[import-untyped]

        class _RezDumper(SafeDumper):  # type: ignore[valid-type]
            pass

        _RezDumper.add_representer(str, SafeDumper.represent_str)  # type: ignore[attr-defined]
        for name in ("Version", "Requirement"):
            cls = types_map.get(name)
            if cls is not None:
                _RezDumper.add_representer(cls, _represent_rez_type)  # type: ignore[attr-defined]
        _RezDumper.add_representer(FunctionType, _represent_function)  # type: ignore[attr-defined]
        _RezDumper.add_representer(BuiltinFunctionType, _represent_builtin)  # type: ignore[attr-defined]
        return _RezDumper  # type: ignore[return-value]
    else:
        # ruamel.yaml — use its RoundTripRepresenter as base
        base_dumper = _yaml.RoundTripRepresenter
        base_dumper.add_representer(str, base_dumper.represent_str)  # type: ignore[attr-defined]
        for name in ("Version", "Requirement"):
            cls = types_map.get(name)
            if cls is not None:
                base_dumper.add_representer(cls, _represent_rez_type)  # type: ignore[attr-defined]
        base_dumper.add_representer(FunctionType, _represent_function)  # type: ignore[attr-defined]
        base_dumper.add_representer(BuiltinFunctionType, _represent_builtin)  # type: ignore[attr-defined]
        return base_dumper  # type: ignore[return-value]


_RezDumper = _build_dumper(type("Dummy", (), {}))


# ── Public API ─────────────────────────────────────────────────────────────

def dump_yaml(
    data: Any,
    default_flow_style: bool = False,
) -> str:
    """Serialise *data* to a YAML string.

    Supports Rez-native types (Version, Requirement) and Python functions.

    Args:
        data: Python object to serialise.
        default_flow_style: If True, use flow style (``{k: v}``) instead of
            block style (``k: v\\n``) for collections.

    Returns:
        YAML-formatted string, stripped of leading/trailing whitespace.
    """
    if _RUAMEL:
        buf = _yaml.compat.StringIO()
        yml = _yaml.YAML()
        yml.default_flow_style = default_flow_style
        yml.dump(data, buf)
        return buf.getvalue().strip()
    else:
        content = _yaml.dump(
            data,
            default_flow_style=default_flow_style,
            Dumper=_RezDumper,  # type: ignore[arg-type]
        )
        return content.strip()


def load_yaml(filepath: str) -> Any:
    """Load YAML data from a file.

    Args:
        filepath: Path to the YAML file.

    Returns:
        Parsed Python object (typically a dict or list).

    Raises:
        FileNotFoundError: If the file does not exist.
        _yaml.YAMLError: If the file contains invalid YAML.
    """
    if not os.path.exists(filepath):
        raise FileNotFoundError(f"YAML file not found: {filepath}")

    with open(filepath, encoding="utf-8") as f:
        if _RUAMEL:
            yml = _yaml.YAML(typ="safe")
            return yml.load(f)
        else:
            return _yaml.load(f, Loader=_yaml.SafeLoader)


def save_yaml(filepath: str, **fields: Any) -> None:
    """Write key-value pairs as YAML to a file.

    Args:
        filepath: Destination file path.
        **fields: Key-value pairs to serialise.
    """
    content = dump_yaml(fields)
    os.makedirs(os.path.dirname(filepath) or ".", exist_ok=True)
    with open(filepath, "w", encoding="utf-8") as f:
        f.write(content + "\n")
