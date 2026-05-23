"""Tests for rez_next.release_hook module."""

import pytest
from rez_next.release_hook import (
    ReleaseHook,
    ReleaseHookEvent,
    get_release_hook_types,
    create_release_hook,
    create_release_hooks,
)


class TestReleaseHook:
    def test_name_raises(self):
        with pytest.raises(NotImplementedError):
            ReleaseHook.name()

    def test_concrete_hook(self):
        class TestHook(ReleaseHook):
            @classmethod
            def name(cls):
                return "test_hook"

        hook = TestHook("/tmp")
        assert hook.name() == "test_hook"
        assert hook.source_path == "/tmp"

    def test_pre_build_default(self):
        class TestHook(ReleaseHook):
            @classmethod
            def name(cls):
                return "test"

        hook = TestHook("/tmp")
        hook.pre_build()  # should not raise

    def test_pre_release_default(self):
        class TestHook(ReleaseHook):
            @classmethod
            def name(cls):
                return "test"

        hook = TestHook("/tmp")
        hook.pre_release()

    def test_post_release_default(self):
        class TestHook(ReleaseHook):
            @classmethod
            def name(cls):
                return "test"

        hook = TestHook("/tmp")
        hook.post_release()


class TestReleaseHookEvent:
    def test_pre_build(self):
        event = ReleaseHookEvent.pre_build
        assert event.label == "pre-build"
        assert event.noun == "build"
        assert event.__name__ == "pre_build"

    def test_pre_release(self):
        event = ReleaseHookEvent.pre_release
        assert event.label == "pre-release"
        assert event.noun == "release"
        assert event.__name__ == "pre_release"

    def test_post_release(self):
        event = ReleaseHookEvent.post_release
        assert event.label == "post-release"
        assert event.noun == "release"
        assert event.__name__ == "post_release"


class TestFactoryFunctions:
    def test_get_release_hook_types(self):
        types = get_release_hook_types()
        assert isinstance(types, list)

    def test_create_release_hook_nonexistent(self):
        from rez_next.exceptions import RezPluginError
        with pytest.raises(RezPluginError):
            create_release_hook("nonexistent_hook", "/tmp")

    def test_create_release_hooks_empty(self):
        hooks = create_release_hooks([], "/tmp")
        assert hooks == []

    def test_create_release_hooks_graceful_fallback(self):
        hooks = create_release_hooks(["nonexistent"], "/tmp")
        assert hooks == []
