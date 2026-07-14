"""
Rez-compatible patching utilities for package requests.

Aligns with ``rez.utils.patching`` API.

Provides ``get_patched_request()`` for applying patch args to a
package request list, following the same override rules as upstream Rez.

Rez API: ``rez.utils.patching.get_patched_request()``

Rez issue #... lessons:
- Use ``Requirement`` objects for consistent name matching,
  avoiding brittle string-level comparisons that broke on
  version-range syntax changes.
"""

from __future__ import annotations

from rez_next._native import PackageRequirement as Requirement


def get_patched_request(requires, patchlist):
    """Apply patch args to a request.

    For example, consider::

        >>> get_patched_request(["foo-5", "bah-8.1"], ["foo-6"])
        ["foo-6", "bah-8.1"]
        >>> get_patched_request(["foo-5", "bah-8.1"], ["^bah"])
        ["foo-6"]

    Override rules (from rez docs):

    PATCH  OVERRIDES:  foo  !foo  ~foo
    -----  ----------  ---  ----  -----
    foo                 Y    Y     Y
    !foo                N    N     N
    ~foo                N    N     Y
    ^foo                Y    Y     Y

    Args:
        requires (list[str | Requirement]): Original package request list.
        patchlist (list[str]): List of patch requests (e.g. ``["foo-6"]``,
            ``["^bah"]``, ``["!dep"]``, ``["~weak_dep"]``).

    Returns:
        list[Requirement]: Patched request list.
    """
    rules = {
        "": (True, True, True),
        "!": (False, False, False),
        "~": (False, False, True),
        "^": (True, True, True),
    }

    requires = [Requirement(x) if not isinstance(x, Requirement) else x
                for x in requires]
    appended = []

    for patch in patchlist:
        if patch and patch[0] in ("!", "~", "^"):
            ch = patch[0]
            name = Requirement(patch[1:]).name
        else:
            ch = ""
            name = Requirement(patch).name

        rule = rules[ch]
        replaced = ch == "^"

        for i, req in enumerate(requires):
            if req is None or req.name != name:
                continue

            if not req.conflict:
                replace = rule[0]      # normal request
            elif not req.weak:
                replace = rule[1]      # conflict request (!foo)
            else:
                replace = rule[2]      # weak request (~foo)

            if replace:
                if replaced:
                    requires[i] = None
                else:
                    requires[i] = Requirement(patch)
                    replaced = True

        if not replaced:
            appended.append(Requirement(patch))

    return [x for x in requires if x is not None] + appended
