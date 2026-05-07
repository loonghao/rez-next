# Solver module - compatible with rez.solver
# This module re-exports from the native _native.solver_ submodule

import rez_next._native  # ensure extension module is initialized  # noqa: F401
from rez_next._native.solver_ import *  # noqa: F401,F403
