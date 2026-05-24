"""Bridge to rez_next utils/data_utils module (data manipulation).

Aligns with rez.utils.data_utils API:
- ``ModifyList`` — list modifiers for config merging
- ``DelayLoad`` — delayed file loading
- ``deep_update()``, ``deep_del()`` — dict manipulation
- ``get_dict_diff()``, ``get_dict_diff_str()`` — dict comparison
- ``cached_property``, ``cached_class_property`` — caching descriptors
- ``LazySingleton`` — thread-safe singleton
- ``AttrDictWrapper``, ``RO_AttrDictWrapper`` — attribute-based dict access
- ``convert_dicts()``, ``convert_json_safe()`` — data conversion
- ``get_object_completions()`` — tab completion helper
- ``AttributeForwardMeta``, ``LazyAttributeMeta`` — metaclasses
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
    / "data_utils.py"
)
globals().update(runpy.run_path(str(_IMPL)))
