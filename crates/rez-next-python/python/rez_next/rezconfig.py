"""
Rez-compatible rezconfig defaults module.

Aligns with ``rez.rezconfig`` API:
- Central place for all configuration default values
- Follows the same structure as Rez's ``src/rez/rezconfig.py``
- Users can ``from rez import rezconfig`` to access defaults

See https://github.com/AcademySoftwareFoundation/rez/blob/main/src/rez/rezconfig.py
"""

from __future__ import annotations

import os

from .config import Config

# ── Re-export Config for convenience ─────────────────────────────────────────

Config = Config

# ── Paths ────────────────────────────────────────────────────────────────────

packages_path: list[str] = ["~/packages"]
local_packages_path: str = "~/packages"
release_packages_path: str = os.path.join("~", ".rez", "packages", "int")
tmpdir: str | None = None

# ── Extensions ───────────────────────────────────────────────────────────────

plugin_path: list[str] = []
bind_module_path: list[str] = []

# ── Caching ──────────────────────────────────────────────────────────────────

resolve_caching: bool = True
cache_package_files: bool = True
memcached_uri: list[str] = []

# ── Package Copy ─────────────────────────────────────────────────────────────

default_relocatable: bool = True

# ── Package Caching ──────────────────────────────────────────────────────────

default_cachable: bool = False
cache_packages_path: str | None = None
read_package_cache: bool = True
write_package_cache: bool = True
package_cache_max_variant_days: int = 30

# ── Package Resolution ───────────────────────────────────────────────────────

implicit_packages: list[str] = [
    "~platform=={system.platform}",
    "~arch=={system.arch}",
    "~os=={system.os}",
]
variant_select_mode: str = "version_priority"
package_filter: str | None = None
allow_unversioned_packages: bool = True

# ── Environment Resolution ───────────────────────────────────────────────────

parent_variables: list[str] = []
default_shell: str = ""
suite_visibility: str = "always"
rez_tools_visibility: str = "append"
package_commands_sourced_first: bool = True

# ── Context Tracking ─────────────────────────────────────────────────────────

context_tracking_host: str = ""

# ── Debugging ────────────────────────────────────────────────────────────────

debug_rex_print_all: bool = False
debug_rex_print_all_external: bool = False
debug_resolve_package_order: bool = False
debug_resolve_package_selection: bool = False
debug_build: bool = False
debug_resolve: bool = False
debug_memcached: bool = False
warn_untimestamped: bool = False
warn_unrelocatable: bool = False
catch_rex_errors: bool = True
shell_error_truncate_cap: int = 750

# ── Package Build/Release ────────────────────────────────────────────────────

build_directory: str = "build"
build_thread_count: str = "physical_cores"
release_hooks: list[str] = []

# ── Suites ───────────────────────────────────────────────────────────────────

suite_alias_prefix_char: str = "+"

# ── Appearance ───────────────────────────────────────────────────────────────

quiet: bool = False
show_progress: bool = True
set_prompt: bool = True
prefix_prompt: bool = True

# ── Plugins ──────────────────────────────────────────────────────────────────

plugins: dict = {}

# ── Misc ─────────────────────────────────────────────────────────────────────

max_package_changelog_chars: int = 65536
create_executable_script_mode: str = "single"
optionvars: str | None = None

# ── Rez-1 Compatibility ──────────────────────────────────────────────────────

disable_rez_1_compatibility: bool = True

# ── Help ─────────────────────────────────────────────────────────────────────

documentation_url: str = "https://rez.readthedocs.io"

# ── Colorization ─────────────────────────────────────────────────────────────

color_enabled: bool = os.name == "posix"

# ── GUI ──────────────────────────────────────────────────────────────────────

use_pyside: bool = False
use_pyqt: bool = False
gui_threads: bool = True

# ── Public API ───────────────────────────────────────────────────────────────

__all__ = [
    "Config",
    "packages_path",
    "local_packages_path",
    "release_packages_path",
    "tmpdir",
    "plugin_path",
    "bind_module_path",
    "resolve_caching",
    "cache_package_files",
    "memcached_uri",
    "default_relocatable",
    "default_cachable",
    "cache_packages_path",
    "read_package_cache",
    "write_package_cache",
    "package_cache_max_variant_days",
    "implicit_packages",
    "variant_select_mode",
    "package_filter",
    "allow_unversioned_packages",
    "parent_variables",
    "default_shell",
    "suite_visibility",
    "rez_tools_visibility",
    "package_commands_sourced_first",
    "context_tracking_host",
    "debug_rex_print_all",
    "debug_rex_print_all_external",
    "debug_resolve_package_order",
    "debug_resolve_package_selection",
    "debug_build",
    "debug_resolve",
    "debug_memcached",
    "warn_untimestamped",
    "warn_unrelocatable",
    "catch_rex_errors",
    "shell_error_truncate_cap",
    "build_directory",
    "build_thread_count",
    "release_hooks",
    "suite_alias_prefix_char",
    "quiet",
    "show_progress",
    "set_prompt",
    "prefix_prompt",
    "plugins",
    "max_package_changelog_chars",
    "create_executable_script_mode",
    "optionvars",
    "disable_rez_1_compatibility",
    "documentation_url",
    "color_enabled",
    "use_pyside",
    "use_pyqt",
    "gui_threads",
]
