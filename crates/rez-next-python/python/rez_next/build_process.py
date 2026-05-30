"""
build_process - Build and release process framework.

Provides the abstract base classes and factory functions
for managing package build and release workflows.

This module is a pure Python implementation aligned with rez's
build_process.py API, without deprecated parameters or legacy
compatibility code.
"""

import os
import sys
import abc
import enum
import logging
import shutil
import stat
import time
import inspect
from collections import OrderedDict
from typing import Any

logger = logging.getLogger(__name__)


class BuildType(enum.IntEnum):
    """Build type: local (developer) vs central (release)."""
    local = 0
    central = 1


# --- Error hierarchy ---

class BuildProcessError(Exception):
    """Base error for build process operations."""
    pass


class BuildContextResolveError(BuildProcessError):
    """Build environment resolution failed."""
    pass


class ReleaseError(BuildProcessError):
    """Fatal error during release."""
    pass


class ReleaseHookCancellingError(BuildProcessError):
    """A release hook cancelled the release."""
    pass


class ReleaseVCSError(BuildProcessError):
    """VCS operation failed (can be silenced with skip_repo_errors)."""

    def __init__(self, msg, cause=None):
        super().__init__(msg)
        self.cause = cause


# --- BuildProcess ABC ---

class BuildProcess(abc.ABC):
    """Abstract base for build/release process implementations.

    Subclasses auto-register via __init_subclass__ for factory discovery.
    """

    _registry = OrderedDict()

    def __init_subclass__(cls, **kwargs):
        super().__init_subclass__(**kwargs)
        if (not abc.ABC in getattr(cls, "__bases__", []) and
                not getattr(cls, "__abstractmethods__", frozenset())):
            name = getattr(cls, "name", cls.__name__)
            BuildProcess._registry[name] = cls

    def __init__(self, working_dir="", build_system=None, vcs=None,
                 ensure_latest=True, skip_repo_errors=False,
                 ignore_existing_tag=False, verbose=False, quiet=False):
        self._working_dir = working_dir or ""
        self._build_system = build_system
        self._vcs = vcs
        self._ensure_latest = ensure_latest
        self._skip_repo_errors = skip_repo_errors
        self._ignore_existing_tag = ignore_existing_tag
        self._verbose = verbose
        self._quiet = quiet

    @property
    def package(self):
        if self._build_system is not None:
            return getattr(self._build_system, "package", None)
        return None

    @property
    def working_dir(self):
        return self._working_dir or (
            getattr(self._build_system, "working_dir", None)
            if self._build_system is not None
            else None
        )

    @abc.abstractmethod
    def build(self, install_path=None, clean=False, install=False, variants=None):
        """Execute the build."""
        raise NotImplementedError

    @abc.abstractmethod
    def release(self, release_message=None, variants=None):
        """Execute the release (build + publish)."""
        raise NotImplementedError

    def get_changelog(self, max_revisions=None):
        """Get changelog since the last release."""
        if self._vcs is not None:
            return self._vcs.get_changelog(max_revisions=max_revisions)
        return None

    def _print(self, msg):
        if not self._quiet:
            print(msg)

    def _print_header(self, title):
        self._print("\n" + "=" * 60)
        self._print(f"  {title}")
        self._print("=" * 60)

    def _n_of_m(self, n, m):
        return f"[{n}/{m}]"

# --- BuildProcessHelper ---

class BuildProcessHelper(BuildProcess):
    """Concrete helper with common build/release logic.

    Extends BuildProcess with variant iteration, build context creation,
    pre-release validation, post-release tagging, and changelog gathering.
    """

    def repo_operation(self, label=""):
        """Context manager for VCS operations with error suppression."""
        class _RepoOperation:
            def __init__(self, outer, label_):
                self.outer = outer
                self.label = label_
            def __enter__(self):
                return self
            def __exit__(self, exc_type, exc_val, exc_tb):
                if exc_type is ReleaseVCSError and self.outer._skip_repo_errors:
                    logger.warning("Skipping repo operation %s: %s", self.label, exc_val)
                    return True
                return False
        return _RepoOperation(self, label)

    def visit_variants(self, func, variants=None, **kwargs):
        """Iterate over variants, calling func for each."""
        package = self.package
        if package is None:
            return 0, []
        all_variants = getattr(package, "variants", None) or []
        if not all_variants:
            result = func(0, [], **kwargs)
            return 1, [result]
        indices = range(len(all_variants))
        if variants is not None:
            indices = [i for i in indices if i in variants]
        results = []
        for i in indices:
            var_requires = all_variants[i] if i < len(all_variants) else []
            results.append(func(i, var_requires, **kwargs))
        return len(results), results

    def get_package_install_path(self, path):
        """Get the install directory for a package."""
        pkg = self.package
        if pkg is None:
            return path
        name = getattr(pkg, "name", "unknown")
        version = getattr(pkg, "version", "unknown")
        return os.path.join(path, name, str(version))

    def create_build_context(self, variant, build_type, build_path=None):
        """Create a ResolvedContext for a build variant."""
        from rez_next.resolved_context import ResolvedContext
        index, requires = variant
        pkg = self.package
        if pkg is None:
            raise BuildProcessError("No package available for build context")
        name = getattr(pkg, "name", "unknown")
        version = getattr(pkg, "version", "unknown")
        working_dir = self.working_dir or os.getcwd()
        request = [f"{name}-{version}"]
        if requires:
            request.extend(str(r) for r in requires)
        build_requires = getattr(pkg, "build_requires", None) or []
        request.extend(str(r) for r in build_requires)
        try:
            ctx = ResolvedContext(request)
        except Exception as e:
            raise BuildContextResolveError(
                f"Failed to resolve build context for {name}-{version}: {e}"
            ) from e
        rxt_path = None
        if build_path:
            rxt_path = os.path.join(build_path, f"build_{name}_{version}.rxt")
            os.makedirs(build_path, exist_ok=True)
            try:
                with open(rxt_path, "w") as f:
                    f.write(str(ctx))
            except (IOError, OSError):
                rxt_path = None
        return ctx, rxt_path

    def pre_release(self):
        """Validate pre-release conditions."""
        pkg = self.package
        if pkg is None:
            raise ReleaseError("No package available for release")
        name = getattr(pkg, "name", "unknown")
        version = getattr(pkg, "version", "unknown")
        self._print_header(f"Pre-release validation: {name}-{version}")
        if self._vcs is None:
            raise ReleaseError("VCS is required for release. Provide a VCS instance.")
        with self.repo_operation("status check"):
            status = self._vcs.status() if hasattr(self._vcs, "status") else None
            if status:
                logger.debug("VCS status: %s", status)
        tag_name = self.get_current_tag_name()
        with self.repo_operation("tag check"):
            if hasattr(self._vcs, "tag_exists"):
                if self._vcs.tag_exists(tag_name):
                    if self._ignore_existing_tag:
                        logger.warning("Tag %s exists, ignoring", tag_name)
                    else:
                        raise ReleaseError(
                            f"Release tag {tag_name} already exists. "
                            "Set ignore_existing_tag=True to bypass."
                        )
        if self._ensure_latest:
            previous = self.get_previous_release()
            if previous is not None:
                prev_version = getattr(previous, "version", None)
                if prev_version is not None and str(prev_version) > str(version):
                    raise ReleaseError(
                        f"Newer published version exists ({prev_version} > {version}). "
                        "Set ensure_latest=False to bypass."
                    )
        self._print("Pre-release validation passed.")
        return True

    def post_release(self, release_message=None):
        """Post-release operations (VCS tagging)."""
        tag_name = self.get_current_tag_name()
        pkg = self.package
        version = getattr(pkg, "version", "unknown") if pkg else "unknown"
        self._print_header(f"Post-release: tagging {version} as {tag_name}")
        with self.repo_operation("tag"):
            if hasattr(self._vcs, "create_tag"):
                self._vcs.create_tag(tag_name, message=release_message)
                self._print(f"Created tag: {tag_name}")
            else:
                logger.warning("VCS has no create_tag; tag %s not created", tag_name)
        return tag_name
    def get_current_tag_name(self):
        """Get the VCS tag name for the current package version."""
        pkg = self.package
        if pkg is None:
            return "unknown"
        version = getattr(pkg, "version", "unknown")
        tag_format = "v{version}"
        from rez_next import config as cfg
        try:
            cfg_tag = cfg.get("plugins.release_vcs.tag_name", None)
            if cfg_tag:
                tag_format = cfg_tag
        except (AttributeError, KeyError):
            pass  # Config may not have the expected structure
        return tag_format.format(version=version)

    def run_hooks(self, hook_event, **kwargs):
        """Run configured release hooks."""
        try:
            from rez_next.release_hook import run_hooks
            run_hooks(hook_event, package=self.package, vcs=self._vcs, **kwargs)
        except ReleaseHookCancellingError:
            raise
        except (ModuleNotFoundError, ImportError):
            logger.debug("release_hook module not available, skipping %s", hook_event)
        except Exception as e:
            logger.warning("Hook %s failed: %s", hook_event, e)

    def get_previous_release(self):
        """Get the previous release package (same name, older version)."""
        pkg = self.package
        if pkg is None:
            return None
        name = getattr(pkg, "name", None)
        if name is None:
            return None
        try:
            from rez_next.packages_ import get_latest_package
            return get_latest_package(name)
        except Exception:
            return None

    def get_changelog(self, max_revisions=None):
        """Get changelog since the last release."""
        if self._vcs is None:
            return None
        try:
            return self._vcs.get_changelog(max_revisions=max_revisions or 100)
        except Exception:
            return None

    def get_release_data(self):
        """Get metadata dict for release recording."""
        data = OrderedDict()
        pkg = self.package
        data["package_name"] = getattr(pkg, "name", None) if pkg else None
        data["package_version"] = str(getattr(pkg, "version", "")) if pkg else None
        if self._vcs:
            data["vcs_name"] = getattr(self._vcs, "name", type(self._vcs).__name__)
            data["vcs_revision"] = getattr(self._vcs, "revision", None)
        else:
            data["vcs_name"] = None
            data["vcs_revision"] = None
        data["changelog"] = self.get_changelog()
        prev = self.get_previous_release()
        data["previous_version"] = str(getattr(prev, "version", "")) if prev else None
        tag_name = self.get_current_tag_name() if self.package else None
        data["tag_name"] = tag_name
        return data

    def build(self, install_path=None, clean=False, install=False, variants=None):
        """Default build implementation."""
        if self._build_system is None:
            raise BuildProcessError("No build system configured")
        pkg = self.package
        name = getattr(pkg, "name", "unknown") if pkg else "unknown"
        version = getattr(pkg, "version", "unknown") if pkg else "unknown"
        self._print_header(f"Building {name}-{version}")

        def _build_variant(idx, var_reqs, **_kw):
            self._print(f"  Building variant {idx}...")
            return self._build_system.build(
                install_path=install_path, clean=clean, install=install, variant=idx
            )

        count, results = self.visit_variants(_build_variant, variants=variants)
        success_count = sum(1 for r in results if r)
        self._print(f"Build complete: {success_count}/{count} variants succeeded")
        return success_count

    def release(self, release_message=None, variants=None):
        """Default release implementation."""
        if self._build_system is None:
            raise BuildProcessError("No build system configured for release")
        pkg = self.package
        name = getattr(pkg, "name", "unknown") if pkg else "unknown"
        version = getattr(pkg, "version", "unknown") if pkg else "unknown"
        self._print_header(f"Releasing {name}-{version}")
        self.pre_release()
        build_count = self.build(install=True, variants=variants)
        if build_count == 0:
            raise ReleaseError("Build failed: no variants were built successfully")
        tag_name = self.post_release(release_message=release_message)
        self._print(f"Release complete: {name}-{version} tagged as {tag_name}")
        return build_count

# --- Factory functions ---

def get_build_process_types():
    """Get registered build process types."""
    return OrderedDict(BuildProcess._registry)


def create_build_process(
    process_type: str,
    build_system: Any = None,
    vcs: Any = None,
    ensure_latest: bool = True,
    skip_repo_errors: bool = False,
    ignore_existing_tag: bool = False,
    verbose: bool = False,
    quiet: bool = False,
) -> "BuildProcess":
    """Create a :class:`BuildProcess` instance.

    Rez API: ``rez.build_process.create_build_process()``

    Args:
        process_type: Name of the build process implementation.
        build_system: BuildSystem instance used to build the package.
        vcs: ReleaseVCS for release process.
        ensure_latest: If True, do not allow release if newer version exists.
        skip_repo_errors: If True, proceed even when VCS errors occur.
        ignore_existing_tag: Perform release even if tag already exists.
        verbose: Verbose mode.
        quiet: Quiet mode (overrides verbose).
    """
    types = get_build_process_types()
    cls = types.get(process_type)
    if cls is None:
        raise BuildProcessError(
            f"Unknown build process type: {process_type!r}. "
            f"Available: {list(types.keys())}"
        )
    return cls(
        build_system=build_system,
        vcs=vcs,
        ensure_latest=ensure_latest,
        skip_repo_errors=skip_repo_errors,
        ignore_existing_tag=ignore_existing_tag,
        verbose=verbose,
        quiet=quiet,
    )


# --- Platform helpers ---

def _remove_readonly(func, path, exc_info):
    """Clear read-only bit and retry (handles AppleDouble on macOS)."""
    try:
        os.chmod(path, stat.S_IWRITE)
        func(path)
    except (OSError, PermissionError):
        time.sleep(0.1)
        try:
            os.chmod(path, stat.S_IWRITE)
            func(path)
        except (OSError, PermissionError):
            pass  # Best-effort: file may be locked by another process


def _retry_rmtree(path, max_retries=3, delay=0.1):
    """Retry shutil.rmtree with readonly handling."""
    for attempt in range(max_retries):
        try:
            if os.path.exists(path):
                shutil.rmtree(path, onerror=_remove_readonly)
            return
        except (OSError, PermissionError) as e:
            if attempt == max_retries - 1:
                raise
            time.sleep(delay * (attempt + 1))