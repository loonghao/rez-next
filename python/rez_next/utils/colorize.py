"""Bridge to rez_next utils/colorize module (terminal color helpers).

Aligns with rez.utils.colorize API:
- ``critical()``, ``error()``, ``warning()``, ``info()``, ``debug()``
- ``heading()``, ``local()``, ``implicit()``, ``ephemeral()``, ``alias()``
- ``inactive()``, ``notset()``
- ``ColorizedStreamHandler`` — logging handler with color support
- ``colorama_wrap()``, ``stream_is_tty()``
"""
from pathlib import Path
import runpy

_IMPL = (
    Path(__file__).resolve().parents[2]
    / "crates"
    / "rez-next-python"
    / "python"
    / "rez_next"
    / "utils"
    / "colorize.py"
)
globals().update(runpy.run_path(str(_IMPL)))
