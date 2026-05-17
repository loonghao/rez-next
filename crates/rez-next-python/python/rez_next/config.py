import rez_next._native  # ensure extension module is initialized  # noqa: F401
from rez_next._native.config import *  # noqa: F401,F403


class Config:
    """Small Rez-compatible Python config facade."""

    def __init__(self):
        self.packages_path = []
        self.local_packages_path = "~/.rez/packages/int"
        self.release_packages_path = "~/.rez/packages/ext"
        self.default_shell = "cmd" if __import__("os").name == "nt" else "bash"

    def get(self, key, default=None):
        return getattr(self, key, default)


def get(key, default=None):
    """Get a config value (compatible with rez.config.get)."""
    return Config().get(key, default)
