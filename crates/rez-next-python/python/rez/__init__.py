"""Public ``rez`` package backed by :mod:`rez_next`."""

from __future__ import annotations

import sys

import rez_next as _rez_next

sys.modules[__name__] = _rez_next
