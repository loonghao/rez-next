# Rez-Next: High-performance Rust rewrite of Rez
# Compatible with rez API - users can `import rez_next as rez`

from _native import *
import os

# Version info (matches rez.__version__ API)
__version__ = "0.3.0"

# Module root path (matches rez.module_root_path API)
module_root_path = os.path.dirname(os.path.abspath(__file__))

# Make all public symbols available at rez_next.*
# (rez_next_bindings exports are now re-exported at rez_next.*)
