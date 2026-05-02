"""Compare rez and rez_next APIs."""
import rez
import rez_next as rez_next_mod
import inspect

# Get all public attributes from both modules
rez_attrs = set([f for f in dir(rez) if not f.startswith('_')])
rez_next_attrs = set([f for f in dir(rez_next_mod) if not f.startswith('_')])

# Find what's in rez but not in rez_next
missing = rez_attrs - rez_next_attrs
extra = rez_next_attrs - rez_attrs
common = rez_attrs & rez_next_attrs

print('=== MISSING from rez_next (in rez but not rez_next) ===')
for m in sorted(missing):
    print(f'  {m}')

print(f'\n=== EXTRA in rez_next (not in rez) ===')
for e in sorted(extra):
    print(f'  {e}')

print(f'\n=== Summary ===')
print(f'  rez attributes: {len(rez_attrs)}')
print(f'  rez_next attributes: {len(rez_next_attrs)}')
print(f'  missing from rez_next: {len(missing)}')
print(f'  extra in rez_next: {len(extra)}')
print(f'  common: {len(common)}')

# Check submodules
print('\n=== Checking submodules ===')
rez_submodules = [f for f in rez_attrs if inspect.ismodule(getattr(rez, f))]
rez_next_submodules = [f for f in rez_next_attrs if inspect.ismodule(getattr(rez_next_mod, f))]

print(f'rez submodules: {rez_submodules}')
print(f'rez_next submodules: {rez_next_submodules}')
