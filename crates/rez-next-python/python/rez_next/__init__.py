import rez_next._native as _native  # noqa: F401
from rez_next._native import *  # noqa: F401,F403

__version__: str = _native.__version__
__author__: str = _native.__author__

# Top-level singletons — compatible with `from rez import config` and `from rez import system`
config = _native.Config()
system = _native.System()
