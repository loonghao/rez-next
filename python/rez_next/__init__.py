# Rez-Next: High-performance Rust rewrite of Rez
# Compatible with rez API - users can `import rez_next as rez`

import _native
import os
import sys

# Version info (matches rez.__version__ API)
__version__ = "0.3.0"

# Author info (matches rez.__author__ API)
__author__ = "rez-next contributors"

# License info (matches rez.__license__ API)
__license__ = "Apache-2.0"

# Module root path (matches rez.module_root_path API)
module_root_path = os.path.dirname(os.path.abspath(__file__))

# Make all public symbols available at rez_next.*
# (rez_next_bindings exports are now re-exported at rez_next.*)
from _native import *

# Register submodules in sys.modules for "from rez_next.* import ..." to work
# Process modules from _native
import rez_next._native.package_order
sys.modules["rez_next.package_order"] = rez_next._native.package_order

# Also register other submodules dynamically
for _attr_name in dir(_native):
    _attr = getattr(_native, _attr_name)
    _type_str = str(type(_attr))
    if "module" in _type_str.lower():
        _full_name = f"rez_next._native.{_attr_name}"
        if _full_name in sys.modules:
            sys.modules[f"rez_next.{_attr_name}"] = sys.modules[_full_name]
