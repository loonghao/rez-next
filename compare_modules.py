#!/usr/bin/env python
"""Detailed comparison between rez and rez_next modules."""

import os
import re
import importlib
import inspect

def get_all_attrs(module_name):
    """Get all attributes (including submodules) of a module."""
    try:
        mod = importlib.import_module(module_name)
        attrs = {}
        for attr_name in dir(mod):
            if attr_name.startswith('_'):
                continue
            try:
                attr = getattr(mod, attr_name)
                attr_type = type(attr).__name__
                # Check if it's a submodule
                if inspect.ismodule(attr):
                    attr_type = 'module'
                elif callable(attr):
                    attr_type = 'callable'
                attrs[attr_name] = attr_type
            except Exception:
                attrs[attr_name] = 'unknown'
        return attrs
    except Exception as e:
        print(f"Error importing {module_name}: {e}")
        return {}

def compare_detailed():
    """Detailed comparison of rez vs rez_next."""
    
    # Get top-level attributes
    rez_attrs = get_all_attrs('rez')
    rez_next_attrs = get_all_attrs('rez_next')
    
    print("=" * 60)
    print("DETAILED COMPARISON: rez vs rez_next")
    print("=" * 60)
    
    # Find missing in rez_next
    missing = set(rez_attrs.keys()) - set(rez_next_attrs.keys())
    if missing:
        print("\n### Missing from rez_next (top-level) ###")
        for attr in sorted(missing):
            print(f"  - {attr} ({rez_attrs[attr]})")
    
    # Find extra in rez_next
    extra = set(rez_next_attrs.keys()) - set(rez_attrs.keys())
    if extra:
        print("\n### Extra in rez_next (top-level) ###")
        for attr in sorted(extra):
            print(f"  + {attr} ({rez_next_attrs[attr]})")
    
    # Compare submodules in detail
    print("\n" + "=" * 60)
    print("SUBMODULE COMPARISON")
    print("=" * 60)
    
    # Check key submodules
    key_submodules = [
        'rez.packages', 'rez.package_search', 'rez.solver',
        'rez.resolved_context', 'rez.build_system', 'rez.release',
        'rez.suite', 'rez.config', 'rez.utils'
    ]
    
    for submodule in key_submodules:
        rez_sub_attrs = get_all_attrs(submodule)
        rez_next_submodule = submodule.replace('rez.', 'rez_next.')
        rez_next_sub_attrs = get_all_attrs(rez_next_submodule)
        
        if rez_sub_attrs and rez_next_sub_attrs:
            sub_missing = set(rez_sub_attrs.keys()) - set(rez_next_sub_attrs.keys())
            if sub_missing:
                print(f"\n### {submodule} -> missing in rez_next ###")
                for attr in sorted(sub_missing)[:20]:  # Limit output
                    print(f"  - {attr} ({rez_sub_attrs[attr]})")
                if len(sub_missing) > 20:
                    print(f"  ... and {len(sub_missing) - 20} more")
    
    # Check for functions/classes that might be implemented differently
    print("\n" + "=" * 60)
    print("CHECKING KEY FUNCTIONS")
    print("=" * 60)
    
    # Try to find functions in rez that should be in rez_next
    key_funcs = [
        'rez.resolve',
        'rez.packages.get_latest_package',
        'rez.packages.iter_packages',
        'rez.resolved_context.ResolvedContext',
    ]
    
    for func_path in key_funcs:
        try:
            parts = func_path.split('.')
            mod_path = '.'.join(parts[:-1])
            func_name = parts[-1]
            
            rez_func = getattr(importlib.import_module(mod_path), func_name, None)
            rez_next_mod_path = mod_path.replace('rez.', 'rez_next.')
            rez_next_func = getattr(importlib.import_module(rez_next_mod_path), func_name, None)
            
            if rez_func and not rez_next_func:
                print(f"\n⚠️  {func_path} exists in rez but NOT in rez_next")
            elif rez_func and rez_next_func:
                print(f"\n✓ {func_path} exists in both")
        except Exception as e:
            pass
    
    print("\n" + "=" * 60)
    print("SUMMARY")
    print("=" * 60)
    print(f"rez top-level attributes: {len(rez_attrs)}")
    print(f"rez_next top-level attributes: {len(rez_next_attrs)}")
    print(f"Missing from rez_next: {len(missing)}")
    print(f"Extra in rez_next: {len(extra)}")

if __name__ == '__main__':
    compare_detailed()
