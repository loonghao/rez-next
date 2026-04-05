import rez_next._native  # ensure extension module is initialized  # noqa: F401
from rez_next._native.system import *  # noqa: F401,F403

# Expose system singleton's properties as module-level attributes,
# matching rez.system module API: `import rez.system; rez.system.platform`
platform: str = system.platform  # type: ignore[name-defined]  # noqa: F821
arch: str = system.arch  # type: ignore[name-defined]  # noqa: F821
os: str = system.os  # type: ignore[name-defined]  # noqa: F821
