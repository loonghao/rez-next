"""Package filtering for rez-next.

This module provides the same API as rez.package_filter for drop-in compatibility.
It defines rules and filters used during package resolution to include/exclude
packages based on glob, regex, version range, and timestamp criteria.

API Reference: rez.package_filter
  - PackageFilter: manages inclusion/exclusion rule sets
  - GlobRule: match package name/version/description via glob
  - RangeRule: match package version range
  - RegexRule: match package name/description via regex
  - TimestampRule: match packages before/after a timestamp
"""

import fnmatch as _fnmatch
import re as _re
from typing import Any, Dict, Optional, Tuple

import rez_next._native  # noqa: F401
from rez_next._native.package_filter import PackageFilter  # noqa: F401

# ── Rule base class ─────────────────────────────────────────────────────────


class Rule:
    """Base class for filter rules.

    This matches rez.package_filter.Rule. Individual rule instances are
    created and used internally by PackageFilter via its add_inclusion()
    and add_exclusion() text-based parsing.

    Subclasses must implement:
      - matches(package_dict) -> bool
      - cost
      - family
      - to_pod
    """

    name = "base"

    def matches(self, package_dict: Dict[str, Any]) -> bool:
        raise NotImplementedError

    @property
    def cost(self) -> float:
        return 100.0

    @property
    def family(self) -> Optional[str]:
        return None

    def to_pod(self) -> Tuple[str, str]:
        return (self.name, "")


# ── GlobRule ────────────────────────────────────────────────────────────────


class GlobRule(Rule):
    """A rule that matches packages using glob patterns.

    This matches rez.package_filter.GlobRule.

    Args:
        pattern: Glob pattern (e.g., "*.beta" for version, "maya-*" for name).
        field: The field to match against ("name", "version", "description", or
               the GlobField enum value).
        family: Optional package family this rule applies to.
    """

    name = "glob"

    def __init__(self, pattern: str, field: str = "name", family: Optional[str] = None):
        self._pattern = pattern
        self._field = field
        self._family = family

    @property
    def pattern(self) -> str:
        return self._pattern

    @property
    def field(self) -> str:
        return self._field

    @property
    def cost(self) -> float:
        return 1.0

    @property
    def family(self) -> Optional[str]:
        return self._family

    def matches(self, package_dict: Dict[str, Any]) -> bool:
        value = package_dict.get(self._field, "")
        if value is None:
            value = ""
        return _fnmatch.fnmatchcase(str(value), self._pattern)

    def to_pod(self) -> Tuple[str, str]:
        return (self.name, self._pattern)

    def __repr__(self) -> str:
        return f"GlobRule('{self._pattern}', field='{self._field}')"


# ── RegexRule ───────────────────────────────────────────────────────────────


class RegexRule(Rule):
    """A rule that matches packages using regular expressions.

    This matches rez.package_filter.RegexRule.

    Args:
        pattern: Regular expression pattern.
        field: The field to match against ("name", "description").
        family: Optional package family this rule applies to.
    """

    name = "regex"

    def __init__(self, pattern: str, field: str = "name", family: Optional[str] = None):
        self._regex = _re.compile(pattern)
        self._pattern = pattern
        self._field = field
        self._family = family

    @property
    def pattern(self) -> str:
        return self._pattern

    @property
    def field(self) -> str:
        return self._field

    @property
    def cost(self) -> float:
        return 10.0

    @property
    def family(self) -> Optional[str]:
        return self._family

    def matches(self, package_dict: Dict[str, Any]) -> bool:
        value = package_dict.get(self._field, "")
        if value is None:
            value = ""
        return bool(self._regex.search(str(value)))

    def to_pod(self) -> Tuple[str, str]:
        return (self.name, self._pattern)

    def __repr__(self) -> str:
        return f"RegexRule('{self._pattern}', field='{self._field}')"


# ── RangeRule ───────────────────────────────────────────────────────────────


class RangeRule(Rule):
    """A rule that matches packages by version range.

    This matches rez.package_filter.RangeRule.

    Args:
        range_str: Version range string (e.g., ">=3.9,<4.0").
        family: Optional package family this rule applies to.
    """

    name = "range"

    def __init__(self, range_str: str, family: Optional[str] = None):
        self._range_str = range_str
        self._family = family
        # Parse version range from string
        from rez_next.version import VersionRange

        self._range = VersionRange(range_str)

    @property
    def pattern(self) -> str:
        return self._range_str

    @property
    def cost(self) -> float:
        return 5.0

    @property
    def family(self) -> Optional[str]:
        return self._family

    def matches(self, package_dict: Dict[str, Any]) -> bool:
        version_str = package_dict.get("version")
        if not version_str:
            return False
        from rez_next.version import Version

        try:
            version = Version(str(version_str))
            return self._range.contains(version)
        except Exception:
            return False

    def to_pod(self) -> Tuple[str, str]:
        return (self.name, self._range_str)

    def __repr__(self) -> str:
        return f"RangeRule('{self._range_str}')"


# ── TimestampRule ───────────────────────────────────────────────────────────


class TimestampRule(Rule):
    """A rule that matches packages based on timestamp (before/after).

    This matches rez.package_filter.TimestampRule.

    Args:
        operation: "before" or "after".
        timestamp: Unix timestamp (int).
        family: Optional package family this rule applies to.
    """

    name = "timestamp"

    def __init__(self, operation: str, timestamp: int, family: Optional[str] = None):
        if operation not in ("before", "after"):
            raise ValueError(
                f"TimestampRule operation must be 'before' or 'after', got '{operation}'"
            )
        self._operation = operation
        self._timestamp = timestamp
        self._family = family

    @property
    def pattern(self) -> str:
        return f"{self._operation}({self._timestamp})"

    @property
    def operation(self) -> str:
        return self._operation

    @property
    def timestamp(self) -> int:
        return self._timestamp

    @property
    def cost(self) -> float:
        return 50.0

    @property
    def family(self) -> Optional[str]:
        return self._family

    def matches(self, package_dict: Dict[str, Any]) -> bool:
        pkg_ts = package_dict.get("timestamp")
        if pkg_ts is None:
            return False
        try:
            ts = int(pkg_ts)
            if self._operation == "before":
                return ts < self._timestamp
            else:
                return ts >= self._timestamp
        except (ValueError, TypeError):
            return False

    def to_pod(self) -> Tuple[str, str]:
        return (self.name, self.pattern)

    def __repr__(self) -> str:
        return f"TimestampRule('{self._operation}', {self._timestamp})"


# ── Convenience functions ──────────────────────────────────────────────────


def parse_filter_string(text: str) -> PackageFilter:
    """Parse a filter string into a PackageFilter.

    This matches rez.package_filter.parse_filter_string().

    Args:
        text: Filter string in rez format (e.g., "~=*.beta", "~!=python-3").

    Returns:
        A PackageFilter instance with parsed rules.
    """
    flt = PackageFilter()
    text = text.strip()
    if not text:
        return flt

    parts = [p.strip() for p in text.split(",")]
    for part in parts:
        if not part:
            continue
        if part.startswith("~="):
            rule_text = part[2:].strip()
            flt.add_inclusion(rule_text)
        elif part.startswith("~!"):
            rule_text = part[2:].strip()
            flt.add_exclusion(rule_text)
        elif part.startswith("~"):
            rule_text = part[1:].strip()
            flt.add_inclusion(rule_text)
        else:
            flt.add_inclusion(part)

    return flt


# ── Re-export for convenience ──────────────────────────────────────────────

__all__ = [
    "PackageFilter",
    "Rule",
    "GlobRule",
    "RegexRule",
    "RangeRule",
    "TimestampRule",
    "parse_filter_string",
]
