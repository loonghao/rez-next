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

import copy
import os
import re
import sys as _sys
from contextlib import contextmanager
from functools import lru_cache
from inspect import ismodule
from typing import Any, TypeVar, TYPE_CHECKING

from .deprecations import warn as _deprecations_warn, RezDeprecationWarning
from .exceptions import ConfigurationError

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
import re
import sys as _sys
from contextlib import contextmanager
from functools import lru_cache
from inspect import ismodule
from typing import Any, TypeVar, TYPE_CHECKING

from .deprecations import warn as _deprecations_warn, RezDeprecationWarning
from .exceptions import ConfigurationError

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


T = TypeVar("T")


# ====================================================================
#   Lightweight schema / validation helpers
#   (replaces rez.vendor.schema.Schema to avoid external deps)
# ====================================================================

class _SchemaError(ValueError):
    pass


class _Schema(object):
    """Minimal replacement for schema.Schema."""
    def __init__(self, *validators):
        self._validators = validators

    def validate(self, data):
        if not self._validators:
            return data
        for v in self._validators:
            if isinstance(v, type):
                if not isinstance(data, v):
                    raise _SchemaError("Expected %s, got %s" % (v.__name__, type(data).__name__))
            elif isinstance(v, (_Or, _And, _Schema, _Use)):
                data = v.validate(data)
            elif callable(v):
                if not v(data):
                    raise _SchemaError("Validation failed for %r" % (data,))
            else:
                if data != v:
                    raise _SchemaError("Expected %r, got %r" % (v, data))
        return data


class _Or(object):
    """At least one alternative must match."""
    def __init__(self, *alternatives):
        self._alternatives = alternatives
    def validate(self, data):
        for alt in self._alternatives:
            try:
                if isinstance(alt, type):
                    if isinstance(data, alt):
                        return data
                elif isinstance(alt, (_Or, _And, _Schema, _Use)):
                    return alt.validate(data)
                elif callable(alt):
                    if alt(data):
                        return data
                else:
                    if data == alt:
                        return data
            except (_SchemaError, TypeError, ValueError):
                continue
        raise _SchemaError("None matched for %r" % (data,))


class _And(object):
    def __init__(self, *validator):
        self._validators = validator
    def validate(self, data):
        r = data
        for v in self._validators:
            if isinstance(v, type):
                if not isinstance(r, v):
                    raise _SchemaError("Expected %s, got %s" % (v.__name__, type(r).__name__))
            elif isinstance(v, (_Or, _And, _Schema, _Use)):
                r = v.validate(r)
            elif callable(v):
                r = v(r)
            else:
                if r != v:
                    raise _SchemaError("Expected %r, got %r" % (v, r))
        return r


class _Use(object):
    def __init__(self, func):
        self._func = func
    def validate(self, data):
        return self._func(data)


# ====================================================================
#   Setting Hierarchy (lazy validators per config key)
# ====================================================================

class _Deprecation(object):
    def __init__(self, removed_in, extra=None):
        self._removed_in = removed_in
        self._extra = extra or ""
    def get_message(self, name, env_var=False):
        if self._removed_in:
            p = ["config setting %r" % name]
            if env_var:
                p.append("(via %s)" % env_var)
            p.append("is deprecated, removed in %s." % self._removed_in)
            p.append(self._extra)
            return " ".join(p).strip()
        return ""


class Setting(object):
    schema = _Schema(object)

    def __init__(self, config, key):
        self.config = config
        self.key = key

    @property
    def _env_var_name(self):
        return "REZ_%s" % self.key.upper()

    def _parse_env_var(self, value):
        raise NotImplementedError

    def _warn_deprecated(self, varname=None):
        if self.key in _deprecated_settings:
            _deprecations_warn(
                _deprecated_settings[self.key].get_message(self.key, env_var=varname or False),
                RezDeprecationWarning, pre_formatted=True, filename=varname or self.key)

    def validate(self, data):
        try:
            data = self._validate(data)
            data = self.schema.validate(data)
            data = expand_system_vars(data)
        except _SchemaError as e:
            raise ConfigurationError("Misconfigured setting %r: %s" % (self.key, str(e)))
        return data

    def _validate(self, data):
        if self.key in self.config.overrides:
            return data
        if not self.config.locked:
            v = os.getenv(self._env_var_name)
            if v is not None:
                self._warn_deprecated(varname=self._env_var_name)
                return self._parse_env_var(v)
            vn = self._env_var_name + "_JSON"
            v = os.getenv(vn)
            if v is not None:
                self._warn_deprecated(varname=vn)
                import json
                try:
                    return json.loads(v)
                except ValueError:
                    raise ConfigurationError("Expected $%s to be JSON" % vn)
        if data is not None:
            return data
        attr = "_get_%s" % self.key
        if hasattr(self.config, attr):
            return getattr(self.config, attr)()
        return None


class Str(Setting):
    schema = _Schema(str)
    def _parse_env_var(self, v): return v


class Char(Setting):
    schema = _Schema(str, lambda x: len(x) == 1)
    def _parse_env_var(self, v): return v


class OptionalStr(Str):
    schema = _Or(None, str)


class StrList(Setting):
    schema = _Schema(list, lambda x: all(isinstance(i, str) for i in x))
    sep = ","
    def _parse_env_var(self, v):
        return [x for x in v.replace(self.sep, " ").split() if x]


class OptionalStrList(StrList):
    schema = _Or(None, _And(_Use(lambda x: x or []), list, lambda x: all(isinstance(i, str) for i in x)))


class PathList(StrList):
    sep = os.pathsep
    def _parse_env_var(self, v):
        return [x for x in v.split(self.sep) if x]


class PipInstallRemaps(Setting):
    PARDIR, SEP = map(re.escape, (os.pardir, os.sep))
    RE_TOKENS = {"sep": SEP, "s": SEP, "pardir": PARDIR, "p": PARDIR}
    TOKENS = {"sep": os.sep, "s": os.sep, "pardir": os.pardir, "p": os.pardir}
    KEYS = ["record_path", "pip_install", "rez_install"]
    schema = _Schema(list)

    def validate(self, data):
        data = super().validate(data)
        result = []
        for remap in data:
            if not isinstance(remap, dict):
                raise ConfigurationError("Expected dict, got %s" % type(remap).__name__)
            for key in self.KEYS:
                if key not in remap:
                    raise ConfigurationError("Missing key %r" % key)
                tokens = self.RE_TOKENS if key == "record_path" else self.TOKENS
                remap[key] = remap[key].format(**tokens)
            result.append(remap)
        return result
    def _parse_env_var(self, v): return v


class Int(Setting):
    schema = _Schema(int)
    def _parse_env_var(self, v):
        try: return int(v)
        except ValueError: raise ConfigurationError("Expected %s to be int" % self._env_var_name)


class Float(Setting):
    schema = _Schema(float)
    def _parse_env_var(self, v):
        try: return float(v)
        except ValueError: raise ConfigurationError("Expected %s to be float" % self._env_var_name)


class Bool(Setting):
    schema = _Schema(bool)
    true_words = frozenset(["1","true","t","yes","y","on"])
    false_words = frozenset(["0","false","f","no","n","off"])
    all_words = true_words | false_words
    def _parse_env_var(self, v):
        v = v.lower()
        if v in self.true_words: return True
        if v in self.false_words: return False
        raise ConfigurationError("Expected $%s to be one of: %s" % (self._env_var_name, ", ".join(sorted(self.all_words))))


class OptionalBool(Bool):
    schema = _Or(None, Bool.schema)


class ForceOrBool(Bool):
    FORCE_STR = "force"
    schema = _Or("force", Bool.schema)
    all_words = Bool.all_words | frozenset(["force"])
    def _parse_env_var(self, v):
        return v if v == self.FORCE_STR else super()._parse_env_var(v)


class Dict(Setting):
    schema = _Schema(dict)
    def _parse_env_var(self, v):
        items, r = v.split(","), {}
        for item in items:
            if ":" not in item:
                raise ConfigurationError("Expected k1:v1,k2:v2: %s" % v)
            k, val = item.split(":", 1)
            try: val = int(val)
            except ValueError:
                try: val = float(val)
                except ValueError: pass
            r[k] = val
        return r


class OptionalDict(Dict):
    schema = _Or(None, dict)


class OptionalDictOrDictList(Setting):
    schema = _Or(None, _And(dict, _Use(lambda x: [x])), list)
    def _parse_env_var(self, v): return v


class SuiteVisibility_(Str):
    schema = _Or("never", "always", "parent", "parent_priority")


class VariantSelectMode_(Str):
    schema = _Or("version_priority", "intersection_priority")


class RezToolsVisibility_(Str):
    schema = _Or("append", "prepend", "remove")


class ExecutableScriptMode_(Str):
    schema = _Or("single", "py", "platform_specific", "both")


class OptionalStrOrFunction(Setting):
    schema = _Or(None, str, callable)
    def _parse_env_var(self, v): return v


class PreprocessMode_(Str):
    schema = _Or("before", "after", "override")


class BuildThreadCount_(Setting):
    schema = _Schema(_Or(_And(int, lambda x: x > 0), "physical_cores", "logical_cores"))
    def _parse_env_var(self, v):
        try: return int(v)
        except ValueError: return v


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


# ====================================================================
#   Config Schema — maps every setting key to its validator class
# ====================================================================

T = TypeVar("T")


# ====================================================================
#   Lightweight schema / validation helpers
#   (replaces rez.vendor.schema.Schema to avoid external deps)
# ====================================================================

class _SchemaError(ValueError):
    pass


class _Schema(object):
    """Minimal replacement for schema.Schema."""
    def __init__(self, *validators):
        self._validators = validators

    def validate(self, data):
        if not self._validators:
            return data
        for v in self._validators:
            if isinstance(v, type):
                if not isinstance(data, v):
                    raise _SchemaError("Expected %s, got %s" % (v.__name__, type(data).__name__))
            elif isinstance(v, (_Or, _And, _Schema, _Use)):
                data = v.validate(data)
            elif callable(v):
                if not v(data):
                    raise _SchemaError("Validation failed for %r" % (data,))
            else:
                if data != v:
                    raise _SchemaError("Expected %r, got %r" % (v, data))
        return data


class _Or(object):
    """At least one alternative must match."""
    def __init__(self, *alternatives):
        self._alternatives = alternatives
    def validate(self, data):
        for alt in self._alternatives:
            try:
                if isinstance(alt, type):
                    if isinstance(data, alt):
                        return data
                elif isinstance(alt, (_Or, _And, _Schema, _Use)):
                    return alt.validate(data)
                elif callable(alt):
                    if alt(data):
                        return data
                else:
                    if data == alt:
                        return data
            except (_SchemaError, TypeError, ValueError):
                continue
        raise _SchemaError("None matched for %r" % (data,))


class _And(object):
    def __init__(self, *validator):
        self._validators = validator
    def validate(self, data):
        r = data
        for v in self._validators:
            if isinstance(v, type):
                if not isinstance(r, v):
                    raise _SchemaError("Expected %s, got %s" % (v.__name__, type(r).__name__))
            elif isinstance(v, (_Or, _And, _Schema, _Use)):
                r = v.validate(r)
            elif callable(v):
                r = v(r)
            else:
                if r != v:
                    raise _SchemaError("Expected %r, got %r" % (v, r))
        return r


class _Use(object):
    def __init__(self, func):
        self._func = func
    def validate(self, data):
        return self._func(data)


# ====================================================================
#   Setting Hierarchy (lazy validators per config key)
# ====================================================================

class _Deprecation(object):
    def __init__(self, removed_in, extra=None):
        self._removed_in = removed_in
        self._extra = extra or ""
    def get_message(self, name, env_var=False):
        if self._removed_in:
            p = ["config setting %r" % name]
            if env_var:
                p.append("(via %s)" % env_var)
            p.append("is deprecated, removed in %s." % self._removed_in)
            p.append(self._extra)
            return " ".join(p).strip()
        return ""


class Setting(object):
    schema = _Schema(object)

    def __init__(self, config, key):
        self.config = config
        self.key = key

    @property
    def _env_var_name(self):
        return "REZ_%s" % self.key.upper()

    def _parse_env_var(self, value):
        raise NotImplementedError

    def _warn_deprecated(self, varname=None):
        if self.key in _deprecated_settings:
            _deprecations_warn(
                _deprecated_settings[self.key].get_message(self.key, env_var=varname or False),
                RezDeprecationWarning, pre_formatted=True, filename=varname or self.key)

    def validate(self, data):
        try:
            data = self._validate(data)
            data = self.schema.validate(data)
            data = expand_system_vars(data)
        except _SchemaError as e:
            raise ConfigurationError("Misconfigured setting %r: %s" % (self.key, str(e)))
        return data

    def _validate(self, data):
        if self.key in self.config.overrides:
            return data
        if not self.config.locked:
            v = os.getenv(self._env_var_name)
            if v is not None:
                self._warn_deprecated(varname=self._env_var_name)
                return self._parse_env_var(v)
            vn = self._env_var_name + "_JSON"
            v = os.getenv(vn)
            if v is not None:
                self._warn_deprecated(varname=vn)
                import json
                try:
                    return json.loads(v)
                except ValueError:
                    raise ConfigurationError("Expected $%s to be JSON" % vn)
        if data is not None:
            return data
        attr = "_get_%s" % self.key
        if hasattr(self.config, attr):
            return getattr(self.config, attr)()
        return None


class Str(Setting):
    schema = _Schema(str)
    def _parse_env_var(self, v): return v


class Char(Setting):
    schema = _Schema(str, lambda x: len(x) == 1)
    def _parse_env_var(self, v): return v


class OptionalStr(Str):
    schema = _Or(None, str)


class StrList(Setting):
    schema = _Schema(list, lambda x: all(isinstance(i, str) for i in x))
    sep = ","
    def _parse_env_var(self, v):
        return [x for x in v.replace(self.sep, " ").split() if x]


class OptionalStrList(StrList):
    schema = _Or(None, _And(_Use(lambda x: x or []), list, lambda x: all(isinstance(i, str) for i in x)))


class PathList(StrList):
    sep = os.pathsep
    def _parse_env_var(self, v):
        return [x for x in v.split(self.sep) if x]


class PipInstallRemaps(Setting):
    PARDIR, SEP = map(re.escape, (os.pardir, os.sep))
    RE_TOKENS = {"sep": SEP, "s": SEP, "pardir": PARDIR, "p": PARDIR}
    TOKENS = {"sep": os.sep, "s": os.sep, "pardir": os.pardir, "p": os.pardir}
    KEYS = ["record_path", "pip_install", "rez_install"]
    schema = _Schema(list)

    def validate(self, data):
        data = super().validate(data)
        result = []
        for remap in data:
            if not isinstance(remap, dict):
                raise ConfigurationError("Expected dict, got %s" % type(remap).__name__)
            for key in self.KEYS:
                if key not in remap:
                    raise ConfigurationError("Missing key %r" % key)
                tokens = self.RE_TOKENS if key == "record_path" else self.TOKENS
                remap[key] = remap[key].format(**tokens)
            result.append(remap)
        return result
    def _parse_env_var(self, v): return v


class Int(Setting):
    schema = _Schema(int)
    def _parse_env_var(self, v):
        try: return int(v)
        except ValueError: raise ConfigurationError("Expected %s to be int" % self._env_var_name)


class Float(Setting):
    schema = _Schema(float)
    def _parse_env_var(self, v):
        try: return float(v)
        except ValueError: raise ConfigurationError("Expected %s to be float" % self._env_var_name)


class Bool(Setting):
    schema = _Schema(bool)
    true_words = frozenset(["1","true","t","yes","y","on"])
    false_words = frozenset(["0","false","f","no","n","off"])
    all_words = true_words | false_words
    def _parse_env_var(self, v):
        v = v.lower()
        if v in self.true_words: return True
        if v in self.false_words: return False
        raise ConfigurationError("Expected $%s to be one of: %s" % (self._env_var_name, ", ".join(sorted(self.all_words))))


class OptionalBool(Bool):
    schema = _Or(None, Bool.schema)


class ForceOrBool(Bool):
    FORCE_STR = "force"
    schema = _Or("force", Bool.schema)
    all_words = Bool.all_words | frozenset(["force"])
    def _parse_env_var(self, v):
        return v if v == self.FORCE_STR else super()._parse_env_var(v)


class Dict(Setting):
    schema = _Schema(dict)
    def _parse_env_var(self, v):
        items, r = v.split(","), {}
        for item in items:
            if ":" not in item:
                raise ConfigurationError("Expected k1:v1,k2:v2: %s" % v)
            k, val = item.split(":", 1)
            try: val = int(val)
            except ValueError:
                try: val = float(val)
                except ValueError: pass
            r[k] = val
        return r


class OptionalDict(Dict):
    schema = _Or(None, dict)


class OptionalDictOrDictList(Setting):
    schema = _Or(None, _And(dict, _Use(lambda x: [x])), list)
    def _parse_env_var(self, v): return v


class SuiteVisibility_(Str):
    schema = _Or("never", "always", "parent", "parent_priority")


class VariantSelectMode_(Str):
    schema = _Or("version_priority", "intersection_priority")


class RezToolsVisibility_(Str):
    schema = _Or("append", "prepend", "remove")


class ExecutableScriptMode_(Str):
    schema = _Or("single", "py", "platform_specific", "both")


class OptionalStrOrFunction(Setting):
    schema = _Or(None, str, callable)
    def _parse_env_var(self, v): return v


class PreprocessMode_(Str):
    schema = _Or("before", "after", "override")


class BuildThreadCount_(Setting):
    schema = _Schema(_Or(_And(int, lambda x: x > 0), "physical_cores", "logical_cores"))
    def _parse_env_var(self, v):
        try: return int(v)
        except ValueError: return v


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


# ====================================================================
#   Config Schema — maps every setting key to its validator class
# ====================================================================

config_schema = {
    "packages_path":                              PathList,
    "plugin_path":                                PathList,
    "bind_module_path":                           PathList,
    "standard_system_paths":                      PathList,
    "package_definition_build_python_paths":       PathList,
    "platform_map":                               OptionalDict,
    "default_relocatable_per_package":            OptionalDict,
    "default_relocatable_per_repository":         OptionalDict,
    "default_cachable_per_package":               OptionalDict,
    "default_cachable_per_repository":            OptionalDict,
    "default_cachable":                           OptionalBool,
    "implicit_packages":                          StrList,
    "parent_variables":                           StrList,
    "resetting_variables":                        StrList,
    "release_hooks":                              StrList,
    "context_tracking_context_fields":            StrList,
    "pathed_env_vars":                            StrList,
    "prompt_release_message":                     Bool,
    "critical_styles":                            OptionalStrList,
    "error_styles":                               OptionalStrList,
    "warning_styles":                             OptionalStrList,
    "info_styles":                                OptionalStrList,
    "debug_styles":                               OptionalStrList,
    "heading_styles":                             OptionalStrList,
    "local_styles":                               OptionalStrList,
    "implicit_styles":                            OptionalStrList,
    "ephemeral_styles":                           OptionalStrList,
    "alias_styles":                               OptionalStrList,
    "memcached_uri":                              OptionalStrList,
    "pip_extra_args":                             OptionalStrList,
    "pip_install_remaps":                         PipInstallRemaps,
    "local_packages_path":                        Str,
    "release_packages_path":                      Str,
    "dot_image_format":                           Str,
    "build_directory":                            Str,
    "default_build_process":                      Str,
    "documentation_url":                          Str,
    "suite_visibility":                           SuiteVisibility_,
    "rez_tools_visibility":                       RezToolsVisibility_,
    "create_executable_script_mode":              ExecutableScriptMode_,
    "suite_alias_prefix_char":                    Char,
    "cache_packages_path":                        OptionalStr,
    "package_definition_python_path":             OptionalStr,
    "tmpdir":                                     OptionalStr,
    "context_tmpdir":                             OptionalStr,
    "default_shell":                              OptionalStr,
    "terminal_emulator_command":                  OptionalStr,
    "editor":                                     OptionalStr,
    "image_viewer":                               OptionalStr,
    "difftool":                                   OptionalStr,
    "browser":                                    OptionalStr,
    "critical_fore":                              OptionalStr,
    "critical_back":                              OptionalStr,
    "error_fore":                                 OptionalStr,
    "error_back":                                 OptionalStr,
    "warning_fore":                               OptionalStr,
    "warning_back":                               OptionalStr,
    "info_fore":                                  OptionalStr,
    "info_back":                                  OptionalStr,
    "debug_fore":                                 OptionalStr,
    "debug_back":                                 OptionalStr,
    "heading_fore":                               OptionalStr,
    "heading_back":                               OptionalStr,
    "local_fore":                                 OptionalStr,
    "local_back":                                 OptionalStr,
    "implicit_fore":                              OptionalStr,
    "implicit_back":                              OptionalStr,
    "ephemeral_fore":                             OptionalStr,
    "ephemeral_back":                             OptionalStr,
    "alias_fore":                                 OptionalStr,
    "alias_back":                                 OptionalStr,
    "package_preprocess_function":                OptionalStrOrFunction,
    "package_preprocess_mode":                    PreprocessMode_,
    "error_on_missing_variant_requires":          Bool,
    "context_tracking_host":                      OptionalStr,
    "variant_shortlinks_dirname":                 OptionalStr,
    "build_thread_count":                         BuildThreadCount_,
    "resource_caching_maxsize":                   Int,
    "max_package_changelog_chars":                Int,
    "max_package_changelog_revisions":            Int,
    "memcached_package_file_min_compress_len":    Int,
    "memcached_context_file_min_compress_len":    Int,
    "memcached_listdir_min_compress_len":         Int,
    "memcached_resolve_min_compress_len":         Int,
    "shell_error_truncate_cap":                   Int,
    "package_cache_log_days":                     Int,
    "package_cache_max_variant_days":             Int,
    "package_cache_space_buffer":                 Int,
    "package_cache_used_threshold":               Int,
    "package_cache_clean_limit":                  Float,
    "allow_unversioned_packages":                 Bool,
    "package_cache_during_build":                 Bool,
    "package_cache_local":                        Bool,
    "package_cache_same_device":                  Bool,
    "package_cache_async":                        Bool,
    "color_enabled":                              ForceOrBool,
    "resolve_caching":                            Bool,
    "cache_package_files":                        Bool,
    "cache_listdir":                              Bool,
    "prune_failed_graph":                         Bool,
    "all_parent_variables":                       Bool,
    "all_resetting_variables":                    Bool,
    "package_commands_sourced_first":             Bool,
    "use_variant_shortlinks":                     Bool,
    "warn_shell_startup":                         Bool,
    "warn_untimestamped":                         Bool,
    "warn_all":                                   Bool,
    "warn_none":                                  Bool,
    "debug_file_loads":                           Bool,
    "debug_plugins":                              Bool,
    "debug_package_release":                      Bool,
    "debug_bind_modules":                         Bool,
    "debug_resources":                            Bool,
    "debug_package_exclusions":                   Bool,
    "debug_memcache":                             Bool,
    "debug_resolve_memcache":                     Bool,
    "debug_context_tracking":                     Bool,
    "debug_shell_startup":                        Bool,
    "debug_all":                                  Bool,
    "debug_none":                                 Bool,
    "quiet":                                      Bool,
    "show_progress":                              Bool,
    "catch_rex_errors":                           Bool,
    "default_relocatable":                        Bool,
    "set_prompt":                                 Bool,
    "prefix_prompt":                              Bool,
    "make_package_temporarily_writable":          Bool,
    "read_package_cache":                         Bool,
    "write_package_cache":                        Bool,
    "env_var_separators":                         Dict,
    "variant_select_mode":                        VariantSelectMode_,
    "package_filter":                             OptionalDictOrDictList,
    "package_orderers":                           OptionalDictOrDictList,
    "new_session_popen_args":                     OptionalDict,
    "context_tracking_amqp":                      OptionalDict,
    "context_tracking_extra_fields":              OptionalDict,
    "optionvars":                                 OptionalDict,
    "use_pyside":                                 Bool,
    "use_pyqt":                                   Bool,
    "gui_threads":                                Bool,
}

_deprecated_settings = {
    "warn_old_commands": _Deprecation("the future"),
    "error_old_commands": _Deprecation("the future"),
    "rez_1_environment_variables": _Deprecation("the future"),
    "disable_rez_1_compatibility": _Deprecation("the future"),
}


# ====================================================================
#   Config File Loading
# ====================================================================

@lru_cache()
def _load_config_py(filepath):
    reserved = dict(
        __name__=os.path.splitext(os.path.basename(filepath))[0],
        __file__=filepath,
    )
    g = reserved.copy()
    with open(filepath) as f:
        try:
            code = compile(f.read(), filepath, 'exec')
            exec(code, g)
        except Exception as e:
            raise ConfigurationError("Error loading config from %s: %s" % (filepath, str(e)))
    return {k: v for k, v in g.items()
            if k != '__builtins__' and not ismodule(v) and k not in reserved}


@lru_cache()
def _load_config_yaml(filepath):
    try:
        import yaml as _yaml
    except ImportError:
        raise ConfigurationError("PyYAML is required to load YAML config: %s" % filepath)
    with open(filepath) as f:
        try:
            doc = _yaml.safe_load(f) or {}
        except Exception as e:
            raise ConfigurationError("Error loading config from %s: %s" % (filepath, str(e)))
    if not isinstance(doc, dict):
        raise ConfigurationError("Expected dict, got %s" % type(doc).__name__)
    return doc


def _load_config_from_filepaths(filepaths):
    data = {}
    sourced = []
    loaders = ((".py", _load_config_py), ("", _load_config_yaml))
    for fp in filepaths:
        for ext, loader in loaders:
            f = (os.path.splitext(fp)[0] + ext) if ext else fp
            if not os.path.isfile(f):
                continue
            d = loader(f)
            if fp != get_module_root_config():
                for k in d:
                    if k in _deprecated_settings:
                        _deprecations_warn(
                            _deprecated_settings[k].get_message(k),
                            RezDeprecationWarning, pre_formatted=True, filename=f)
            deep_update(data, d)
            sourced.append(f)
            break
    return data, sourced


def get_module_root_config():
    return os.path.join(os.path.dirname(os.path.abspath(__file__)), "rezconfig.py")


def deep_update(d, u):
    for k, v in u.items():
        if isinstance(v, dict) and k in d and isinstance(d[k], dict):
            deep_update(d[k], v)
        else:
            d[k] = v


# ====================================================================
#   Config class — used by ``rez_next.__init__``
# ====================================================================

class Config(object):
    """Rez-compatible configuration facade.

    Provides attribute-based access to settings with env var override,
    file-based overrides, and schema validation.
    """
    schema = config_schema

    if TYPE_CHECKING:
        def __getattr__(self, item: str) -> Any:
            pass

    def __init__(self, filepaths=None, overrides=None, locked=False):
        self.filepaths = filepaths or []
        self._sourced_filepaths = None
        self.overrides = overrides or {}
        self.locked = locked
        self._native_cfg = self._init_native()

    @staticmethod
    def _init_native():
        if _rust_load_config is not None:
            try:
                return _rust_load_config()
            except Exception:
                pass
        return None

    def __getattr__(self, key):
        schema_cls = self.schema.get(key)
        if schema_cls is not None:
            validator = schema_cls(self, key)
            try:
                result = validator.validate(self._data.get(key))
            except Exception:
                result = self._module_defaults().get(key)
            object.__setattr__(self, key, result)
            return result
        d = self._module_defaults()
        if key in d:
            return d[key]
        raise AttributeError("No such config setting: %r" % key)

    def get(self, key, default=None):
        if self._native_cfg is not None:
            try:
                if self._native_cfg.contains_key(key):
                    for getter in (self._native_cfg.get_string, self._native_cfg.get_int,
                                   self._native_cfg.get_float, self._native_cfg.get_bool):
                        r = getter(key)
                        if r is not None:
                            return r
            except Exception:
                pass
        try:
            return getattr(self, key)
        except (AttributeError, Exception):
            pass
        return default

    def warn(self, key):
        return (not self.quiet and not self.warn_none
                and (self.warn_all or getattr(self, "warn_%s" % key)))

    def debug(self, key):
        return (not self.quiet and not self.debug_none
                and (self.debug_all or getattr(self, "debug_%s" % key)))

    def debug_printer(self, key):
        return _DebugPrinter(self.debug(key))

    @property
    def sourced_filepaths(self):
        _ = self._data
        return self._sourced_filepaths or []

    @property
    def plugins(self):
        return _PluginConfigs(self._data.get("plugins", {}))

    @property
    def data(self):
        d = {}
        for key in self._data:
            if key == "plugins":
                d[key] = self.plugins.data()
            else:
                try:
                    d[key] = getattr(self, key)
                except AttributeError:
                    pass
        return d

    @property
    def nonlocal_packages_path(self):
        paths = list(self.packages_path)
        lp = self.local_packages_path
        if lp in paths:
            paths.remove(lp)
        return paths

    def copy(self, overrides=None, locked=False):
        other = copy.copy(self)
        if overrides is not None:
            other.overrides = overrides
        other.locked = locked
        other._uncache()
        return other

    def override(self, key, value):
        keys = key.split(".")
        if len(keys) > 1:
            if keys[0] != "plugins":
                raise AttributeError("no such setting: %r" % key)
            self.plugins.override(keys[1:], value)
        else:
            self.overrides[key] = value
            self._uncache(key)

    def is_overridden(self, key):
        return key in self.overrides

    def remove_override(self, key):
        ks = key.split(".")
        if len(ks) > 1:
            raise NotImplementedError
        elif key in self.overrides:
            del self.overrides[key]
            self._uncache(key)

    def get_completions(self, prefix):
        toks = prefix.split(".")
        if len(toks) > 1:
            if toks[0] == "plugins":
                return ["plugins." + x for x in self._get_plugin_completions(".".join(toks[1:]))]
            return []
        keys = [x for x in list(self.schema.keys()) if isinstance(x, str)] + ["plugins"]
        keys = [x for x in keys if x.startswith(prefix)]
        if keys == ["plugins"]:
            keys += self._get_plugin_completions("")
        return keys

    def _get_plugin_completions(self, prefix):
        return []

    def _uncache(self, key=None):
        if key and hasattr(self, key):
            delattr(self, key)
        if hasattr(self, "_data"):
            delattr(self, "_data")
        if hasattr(self, "plugins"):
            delattr(self, "plugins")

    def _swap(self, other):
        self.__dict__, other.__dict__ = other.__dict__, self.__dict__

    @lru_cache(maxsize=None)
    def _data_without_overrides(self):
        data, self._sourced_filepaths = _load_config_from_filepaths(self.filepaths)
        return data

    @property
    def _data(self):
        data = copy.deepcopy(self._data_without_overrides)
        deep_update(data, self.overrides)
        return data

    # Dynamic defaults
    def _get_tmpdir(self): return None
    def _get_context_tmpdir(self): return None
    def _get_image_viewer(self): return None
    def _get_editor(self): return None
    def _get_difftool(self): return None
    def _get_terminal_emulator_command(self): return None
    def _get_new_session_popen_args(self): return None

    @staticmethod
    def _module_defaults():
        m = _sys.modules[__name__]
        return {n: v for n, v in vars(m).items()
                if not n.startswith("_") and not callable(v) and not isinstance(v, type)}

    @classmethod
    def _create_main_config(cls, overrides=None):
        filepaths = [get_module_root_config()]
        fp = os.getenv("REZ_CONFIG_FILE")
        if fp:
            filepaths.extend(fp.split(os.pathsep))
        if os.getenv("REZ_DISABLE_HOME_CONFIG", "").lower() not in ("1", "t", "true"):
            filepaths.append(os.path.expanduser("~/.rezconfig"))
        return cls(filepaths, overrides)

    def __str__(self):
        return "%r" % sorted([k for k in self.schema if isinstance(k, str)] + ["plugins"])

    def __repr__(self):
        return "%s(%s)" % (self.__class__.__name__, str(self))


class _PluginConfigs(object):
    def __init__(self, plugin_data):
        self.__dict__["_data"] = plugin_data

    def __setattr__(self, attr, value):
        raise AttributeError("Read-only")

    def __getattr__(self, attr):
        if attr in self.__dict__:
            return self.__dict__[attr]
        data = self.__dict__["_data"]
        if attr in data:
            d = data[attr]
            self.__dict__[attr] = d
            return d
        raise AttributeError("No such setting: plugins.%s" % attr)

    def __iter__(self):
        return iter(self.__dict__["_data"].keys())

    def override(self, key, value):
        if not key:
            raise AttributeError("no such setting")
        data = {}
        cur = data
        ks = list(key)
        while len(ks) > 1:
            cur[ks[0]] = {}
            cur = cur[ks[0]]
            ks = ks[1:]
        cur[ks[0]] = value
        deep_update(self.__dict__["_data"], data)
        if ks[0] in self.__dict__:
            del self.__dict__[ks[0]]

    def data(self):
        d = self.__dict__.copy()
        del d["_data"]
        return d

    def __str__(self):
        return "%r" % sorted(self.__dict__["_data"].keys())

    def __repr__(self):
        return "%s(%s)" % (self.__class__.__name__, str(self))


class _DebugPrinter(object):
    def __init__(self, enabled):
        self.enabled = enabled
    def __call__(self, msg, *args, **kwargs):
        if self.enabled:
            print("[DEBUG]", msg)


def expand_system_vars(data):
    if isinstance(data, str):
        return os.path.expanduser(os.path.expandvars(data))
    elif isinstance(data, (list, tuple, set)):
        return [expand_system_vars(x) for x in data]
    elif isinstance(data, dict):
        return {k: expand_system_vars(v) for k, v in data.items()}
    return data


def create_config(overrides=None):
    if not overrides:
        return config
    return config.copy(overrides=overrides)


def _create_locked_config(overrides=None):
    return Config([get_module_root_config()], overrides=overrides, locked=True)


@contextmanager
def _replace_config(other):
    config._swap(other)
    try:
        yield
    finally:
        config._swap(other)


# Singleton
config = Config._create_main_config()

if os.getenv("REZ_LOG_DEPRECATION_WARNINGS"):
    config.data


# ====================================================================
#   Module-level convenience
# ====================================================================

def get(key, default=None):
    return config.get(key, default)
