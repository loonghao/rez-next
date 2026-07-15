"""Regression tests for public compatibility surfaces that are not implemented yet."""

import pytest
from rez_next.exceptions import PackageCopyError, PackageMoveError
from rez_next.package_copy import copy_package
from rez_next.package_move import move_package
from rez_next.resolver import Resolver


def test_copy_rejects_unsupported_dry_run(tmp_path):
    with pytest.raises(PackageCopyError, match="dry_run"):
        copy_package("tool", str(tmp_path), dry_run=True)


def test_copy_rejects_unsupported_destination_rename(tmp_path):
    with pytest.raises(PackageCopyError, match="dest_name"):
        copy_package("tool", str(tmp_path), dest_name="renamed")


def test_move_rejects_unsupported_timestamp_preservation(tmp_path):
    with pytest.raises(PackageMoveError, match="keep_timestamp"):
        move_package("tool", str(tmp_path), keep_timestamp=True)


def test_resolver_defaults_to_uncached_execution():
    resolver = Resolver(None, [], [])
    assert resolver.caching is False


def test_resolver_rejects_unimplemented_cache_request():
    with pytest.raises(NotImplementedError, match="caching"):
        Resolver(None, [], [], caching=True)
