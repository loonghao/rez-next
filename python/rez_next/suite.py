"""Bridge to rez_next suite module (suite management).

Aligns with rez.suite API:
- ``Suite`` — suite management class
- ``SuiteTool`` — individual tool within a suite
- ``get_suites()`` — list available suites
- ``get_tools()`` — list tools in a suite
- ``get_suite_from_tool()`` — find suite containing a tool
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "suite.py"
)
globals().update(runpy.run_path(str(_IMPL)))
