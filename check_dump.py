#!/usr/bin/env python
"""Check if dump_package_data is in packages_ submodule."""

import rez_next._native.packages_ as m

print("Functions in packages_:")
for name in sorted(dir(m)):
    if not name.startswith('_'):
        print(f"  {name}")

result = 'dump_package_data' in dir(m)
print(f"\ndump_package_data available: {result}")
