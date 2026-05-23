"""
Rez-Next configuration definitions — rezconfig.py equivalent.

This module serves as the `rezconfig.py` of rez-next, defining ALL default
configuration values with the same structure as the original
``rez/rezconfig.py`` for API compatibility.

Settings priority (higher number = higher precedence):

1) Defaults defined in this file (lowest);
2) Overridden by file(s) pointed at by ``$REZ_CONFIG_FILE`` (multiple allowed,
   separated by ``os.pathsep``);
3) Overridden by ``$HOME/.rezconfig`` (unless ``$REZ_DISABLE_HOME_CONFIG`` is 1);
4) Overridden by environment variable ``$REZ_XXX``;
5) Overridden by environment variable ``$REZ_XXX_JSON``;
6) Special: package ``config`` section during build/release.

Plugins settings (``plugins.*``) do not support env var overrides (rules 4-5).
"""

from __future__ import annotations

import os
import sys as _sys

try:
    from . import _native as _native_module
    from ._native.config import load_config as _rust_load_config  # type: ignore
except ImportError:
    try:
        import _native as _native_module

        _native_module.config
    except (ImportError, AttributeError):
        _native_module = None  # type: ignore[assignment]
    _rust_load_config = None


# ============================================================================
#   Paths
# ============================================================================

# The package search path. Rez uses this to find packages. A package with the
# same name and version in an earlier path takes precedence.
packages_path: list[str] = [
    "~/packages",
    "~/.rez/packages/int",
    "~/.rez/packages/ext",
]

# The path that Rez will locally install packages to when ``rez-build`` is used.
local_packages_path: str = "~/packages"

# The path that Rez will deploy packages to when ``rez-release`` is used.
release_packages_path: str = "~/.rez/packages/int"

# Where temporary files go. Defaults to appropriate path depending on your
# system (e.g. ``/tmp`` on *nix). It is highly recommended to use local storage.
tmpdir: str | None = None

# Where temporary files for contexts go. Separate from `tmpdir` because you
# might want this to be on NFS (e.g. during renders on a farm).
context_tmpdir: str | None = None

# Extra Python paths added to ``sys.path`` **only during a build**.
package_definition_build_python_paths: list[str] = []

# The directory from which installed packages can import shared modules.
package_definition_python_path: str | None = None


# ============================================================================
#   Extensions
# ============================================================================

# Search path for rez plugins.
plugin_path: list[str] = []

# Search path for bind modules (used by ``rez-bind``).
bind_module_path: list[str] = []


# ============================================================================
#   Caching  (memcached-based)
# ============================================================================

# Cache resolves to memcached, if enabled. Entries are invalidated when
# packages change.
resolve_caching: bool = True

# Cache package file reads to memcached. Invalidated on filesystem change.
cache_package_files: bool = True

# Cache directory traversals to memcached. Invalidated on filesystem change.
cache_listdir: bool = True

# The size of the local (in-process) resource cache. ``0`` disables; ``-1``
# means unlimited. Size refers to entry count, not byte count.
resource_caching_maxsize: int = -1

# URIs of running memcached server(s). Must be ``None`` or list of strings.
memcached_uri: list[str] = []

# Bytecount beyond which memcached entries are compressed for package files.
memcached_package_file_min_compress_len: int = 16384

# Bytecount beyond which memcached entries are compressed for context files.
memcached_context_file_min_compress_len: int = 1

# Bytecount beyond which memcached entries are compressed for directory listings.
memcached_listdir_min_compress_len: int = 16384

# Bytecount beyond which memcached entries are compressed for resolves.
memcached_resolve_min_compress_len: int = 1


# ============================================================================
#   Package Copy
# ============================================================================

# Whether a package is relocatable by default.
default_relocatable: bool = True

# Override relocatability per-package name.
default_relocatable_per_package: dict | None = None

# Override relocatability per-package-repository path.
default_relocatable_per_repository: dict | None = None


# ============================================================================
#   Package Caching  (local variant payload cache — *not* memcached)
# ============================================================================

# Whether a package is cachable by default. If ``None``, defaults to
# relocatability (i.e. cachable == relocatable).
default_cachable: bool = False

# Override cachability per-package name.
default_cachable_per_package: dict | None = None

# Override cachability per-package-repository path.
default_cachable_per_repository: dict | None = None

# The path where rez locally caches variants. ``None`` disables caching.
cache_packages_path: str | None = None

# If True, variants in a resolve will use locally cached payloads.
read_package_cache: bool = True

# If True, creating or sourcing a context will cause variants to be cached.
write_package_cache: bool = True

# Delete variants that haven't been used in N days. ``0`` disables.
package_cache_max_variant_days: int = 30

# Enable package caching during a package build.
package_cache_during_build: bool = False

# Asynchronously cache packages. If False, resolves block until cached.
package_cache_async: bool = True

# Allow caching of local packages (for testing only).
package_cache_local: bool = False

# Allow caching if source is on the same physical disk as cache (for testing).
package_cache_same_device: bool = False

# Spend up to N seconds cleaning the cache on each update. ``-1`` disables.
package_cache_clean_limit: float = 0.5

# Number of days of package cache logs to keep.
package_cache_log_days: int = 7

# Minimum free space buffer for the cache (100 MB).
package_cache_space_buffer: int = 104857600

# Maximum cache usage threshold (percent). Throttle when exceeded.
package_cache_used_threshold: int = 80


# ============================================================================
#   Package Resolution
# ============================================================================

# Packages implicitly added to all resolves unless ``--no-implicit`` is used.
implicit_packages: list[str] = [
    "~platform=={system.platform}",
    "~arch=={system.arch}",
    "~os=={system.os}",
]

# Override platform/variant OS/arch values. Supports regex keys.
platform_map: dict = {}

# Prune unrelated packages from failed-resolve graphs.
prune_failed_graph: bool = True

# Variant select mode: ``"version_priority"`` or ``"intersection_priority"``.
variant_select_mode: str = "version_priority"

# One or more filters applied during resolution. See Rez docs for syntax.
package_filter: list[dict] | dict | None = None

# One or more package orderers to affect version selection priority.
package_orderers: list | None = None

# If True, unversioned packages are allowed.
allow_unversioned_packages: bool = True

# If True, fail immediately when a variant requires an unavailable package.
error_on_missing_variant_requires: bool = True


# ============================================================================
#   Environment Resolution
# ============================================================================

# Environment variables that are appended/prepended to, not overwritten.
parent_variables: list[str] = []

# If True, *all* variables are treated as parent variables.
all_parent_variables: bool = False

# Variables where conflicting sets are resolved via ``resetenv`` semantics.
resetting_variables: list[str] = []

# If True, *all* variables are treated as resetting.
all_resetting_variables: bool = False

# The default shell type. Empty/``None`` means auto-detect.
default_shell: str = ""

# Command to launch a new terminal (``--detached``). ``None`` = auto-detect.
terminal_emulator_command: str | None = None

# ``subprocess.Popen`` args for new-session shell execution.
new_session_popen_args: dict | None = None

# Override list separators for specific environment variables.
env_var_separators: dict[str, str] = {
    "CMAKE_MODULE_PATH": ";",
    "DOXYGEN_TAGFILES": " ",
}

# Path-like env vars. Wildcards supported.
pathed_env_vars: list[str] = ["*PATH"]

# Suite visibility in resolved environments.
# ``"never"`` | ``"always"`` | ``"parent"`` | ``"parent_priority"``.
suite_visibility: str = "always"

# How Rez CLI tools are added back to ``PATH`` in resolved envs.
rez_tools_visibility: str = "append"

# If True, package commands are sourced before startup scripts (e.g. ``.bashrc``).
package_commands_sourced_first: bool = True

# Paths to initially set ``PATH`` to. Empty = auto-detect per shell.
standard_system_paths: list[str] = []

# Global package preprocess function (string or callable).
package_preprocess_function: str | None = None

# When the global preprocess runs relative to package-local preprocess.
# ``"before"`` | ``"after"`` | ``"override"``.
package_preprocess_mode: str = "override"


# ============================================================================
#   Context Tracking  (AMQP)
# ============================================================================

# AMQP host for context tracking. Empty disables tracking.
# Set to ``"stdout"`` for debugging.
context_tracking_host: str = ""

# AMQP connection parameters.
context_tracking_amqp: dict = {
    "userid": "",
    "password": "",
    "connect_timeout": 10,
    "exchange_name": "",
    "exchange_routing_key": "REZ.CONTEXT",
    "message_delivery_mode": 1,
}

# Which context fields to include in tracking payload.
context_tracking_context_fields: list[str] = [
    "status",
    "timestamp",
    "solve_time",
    "load_time",
    "from_cache",
    "package_requests",
    "implicit_packages",
    "resolved_packages",
]

# Extra fields added to the tracking payload.
context_tracking_extra_fields: dict = {}


# ============================================================================
#   Debugging / Warnings
# ============================================================================

# Print warnings about shell startup sequence.
warn_shell_startup: bool = False

# Print warning when an untimestamped package is found.
warn_untimestamped: bool = False

# Turn on all warnings.
warn_all: bool = False

# Turn off all warnings (overrides ``warn_all``).
warn_none: bool = False

# Print info on file loads/saves.
debug_file_loads: bool = False

# Print debugging info when loading plugins.
debug_plugins: bool = False

# Print VCS commands during package release (and ``rez-pip``).
debug_package_release: bool = False

# Print debugging info in binding modules.
debug_bind_modules: bool = False

# Print debugging info when searching, loading, and copying resources.
debug_resources: bool = False

# Print packages excluded from resolve, and the filter rule responsible.
debug_package_exclusions: bool = False

# Print debugging info related to memcached during a resolve.
debug_resolve_memcache: bool = False

# Debug memcache usage (verbose).
debug_memcache: bool = False

# Print debugging info for AMQP context tracking.
debug_context_tracking: bool = False

# Print debugging info related to shell startup.
debug_shell_startup: bool = False

# Turn on all debugging messages.
debug_all: bool = False

# Turn off all debugging messages (overrides ``debug_all``).
debug_none: bool = False

# If True, rex errors are caught and processed (removing internal stack info).
catch_rex_errors: bool = True

# Max characters printed from stdout/stderr of failed shell commands.
shell_error_truncate_cap: int = 750


# ============================================================================
#   Package Build / Release
# ============================================================================

# Default working directory for a package build.
build_directory: str = "build"

# Number of build threads. ``"logical_cores"`` and ``"physical_cores"`` are
# recognised string values.
build_thread_count: str = "physical_cores"

# Release hooks to run on release. Plugin names.
release_hooks: list[str] = []

# Prompt for release message using an editor.
prompt_release_message: bool = False

# Temporarily make package writable during mutation processes.
make_package_temporarily_writable: bool = True

# Subdirectory for hashed variant symlinks.
variant_shortlinks_dirname: str = "_v"

# Whether to use variant shortlinks when resolving variant root paths.
use_variant_shortlinks: bool = True

# Default build process. Only ``"local"`` is currently available.
default_build_process: str = "local"


# ============================================================================
#   Suites
# ============================================================================

# Prefix character for rez-specific CLI args in suite alias scripts.
suite_alias_prefix_char: str = "+"


# ============================================================================
#   Appearance
# ============================================================================

# Suppress all extraneous output.
quiet: bool = False

# Show progress bars where applicable.
show_progress: bool = True

# Editor for user input.
editor: str | None = None

# Image viewer (used by ``rez-context -g`` etc.).
image_viewer: str | None = None

# Browser for documentation.
browser: str | None = None

# Diff viewer.
difftool: str | None = None

# Default image format for dot graphs.
dot_image_format: str = "png"

# Update the prompt when entering a resolved shell.
set_prompt: bool = True

# Prefix (vs suffix) the prompt indicator.
prefix_prompt: bool = True


# ============================================================================
#   Plugins  (plugin-specific settings — defined by each plugin)
# ============================================================================

plugins: dict = {}


# ============================================================================
#   Misc
# ============================================================================

# Max characters for package changelog entries. ``0`` = no limit.
max_package_changelog_chars: int = 65536

# Max revisions shown in package changelogs. ``0`` = no limit.
max_package_changelog_revisions: int = 0

# Script creation mode: ``"single"``, ``"py"``, ``"platform_specific"``,
# ``"both"``.
create_executable_script_mode: str = "single"

# Extra arguments passed to ``pip install`` by ``rez-pip``.
pip_extra_args: list[str] = []

# Remap rules for unknown parent paths in pip distribution records.
pip_install_remaps: list[dict] = [
    {
        "record_path": r"^{p}{s}{p}{s}(bin{s}.*)",
        "pip_install": r"\1",
        "rez_install": r"\1",
    },
    {
        "record_path": r"^{p}{s}{p}{s}lib{s}python{s}(.*)",
        "pip_install": r"\1",
        "rez_install": r"python{s}\1",
    },
]

# User-preference dict accessible via ``optionvars()`` in package commands.
optionvars: dict | None = None


# ============================================================================
#   Help / Documentation
# ============================================================================

# Where Rez's documentation is hosted.
documentation_url: str = "https://rez.readthedocs.io"


# ============================================================================
#   Colorization
# ============================================================================

# Enable/disable colorization globally.
color_enabled: bool = os.name == "posix"

# -- Logging colors ----------------------------------------------------------
critical_fore: str = "red"
critical_back: str | None = None
critical_styles: list[str] | None = ["bright"]

error_fore: str = "red"
error_back: str | None = None
error_styles: list[str] | None = None

warning_fore: str = "yellow"
warning_back: str | None = None
warning_styles: list[str] | None = None

info_fore: str = "green"
info_back: str | None = None
info_styles: list[str] | None = None

debug_fore: str = "blue"
debug_back: str | None = None
debug_styles: list[str] | None = None

# -- Context-sensitive colors ------------------------------------------------
# Heading
heading_fore: str | None = None
heading_back: str | None = None
heading_styles: list[str] | None = ["bright"]

# Local packages
local_fore: str = "green"
local_back: str | None = None
local_styles: list[str] | None = None

# Implicit packages
implicit_fore: str = "cyan"
implicit_back: str | None = None
implicit_styles: list[str] | None = None

# Ephemerals
ephemeral_fore: str = "blue"
ephemeral_back: str | None = None
ephemeral_styles: list[str] | None = None

# Tool aliases in suites
alias_fore: str = "cyan"
alias_back: str | None = None
alias_styles: list[str] | None = None


# ============================================================================
#   GUI
# ============================================================================

# Force Qt binding selection.
use_pyside: bool = False
use_pyqt: bool = False

# Turn GUI threading on/off (off only for debugging).
gui_threads: bool = True


# ============================================================================
#   Config class — used by ``rez_next.__init__``
# ============================================================================

class Config:
    """Rez-compatible configuration facade.

    Provides attribute-based access to all configuration defaults.
    When the Rust native extension is available, file/env overrides are
    loaded transparently.
    """

    def __init__(self):
        self._native = self._init_native()

    @staticmethod
    def _init_native():
        if _rust_load_config is not None:
            try:
                return _rust_load_config()
            except Exception:
                pass
        return None

    def get(self, key: str, default=None):
        """Get a config value by dot-separated key path.

        Priority:
        1. Rust native config (file/env overrides) — if available;
        2. Module-level default defined in this file;
        3. Explicit ``default`` fallback.
        """
        # 1) Check Rust native config (file/env overrides)
        if self._native is not None:
            try:
                if self._native.contains_key(key):
                    # Try typed getters in priority order
                    result = self._native.get_string(key)
                    if result is not None:
                        return result
                    result = self._native.get_int(key)
                    if result is not None:
                        return result
                    result = self._native.get_float(key)
                    if result is not None:
                        return result
                    result = self._native.get_bool(key)
                    if result is not None:
                        return result
            except Exception:
                pass
        # 2) Fall back to Python module defaults
        try:
            return self._module_defaults()[key.replace(".", "_")]
        except KeyError:
            pass
        # 3) Fall through to attribute-based lookup
        try:
            return getattr(self, key, default)
        except AttributeError:
            return default

    @staticmethod
    def _module_defaults():
        """Return a dict of all module-level default values."""
        import sys as _sys

        this_module = _sys.modules[__name__]
        return {
            name: value
            for name, value in vars(this_module).items()
            if not name.startswith("_")
            and not callable(value)
            and not isinstance(value, type)
        }


# ============================================================================
#   Module-level convenience
# ============================================================================

def get(key: str, default=None):
    """Top-level convenience function (compatible with ``rez.config.get``)."""
    _config = Config()
    return _config.get(key, default)
