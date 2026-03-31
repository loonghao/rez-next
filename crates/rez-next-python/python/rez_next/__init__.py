from rez_next._native import *  # noqa: F401,F403
from rez_next import _native
__version__: str = _native.__version__
__author__: str = _native.__author__
from rez_next._native import Config, System  # noqa: F401
config = Config()
system = System()
