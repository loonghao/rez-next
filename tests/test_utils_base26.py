"""
Tests for rez_next.utils.base26 module.

Verifies that the bridge module exposes the same functions as
rez.utils.base26 and that function signatures match upstream Rez.
"""

import pytest


class TestBase26Module:
    """Tests for rez_next.utils.base26 module."""

    def test_module_imports(self):
        """Verify base26 module can be imported and has expected exports."""
        from rez_next.utils import base26
        assert hasattr(base26, "get_next_base26")
        assert hasattr(base26, "create_unique_base26_symlink")

    def test_get_next_base26_basic(self):
        """get_next_base26 with no prev returns 'a'."""
        from rez_next.utils.base26 import get_next_base26
        assert get_next_base26() == "a"

    def test_get_next_base26_a(self):
        """get_next_base26('a') should return 'b'."""
        from rez_next.utils.base26 import get_next_base26
        assert get_next_base26("a") == "b"

    def test_get_next_base26_z(self):
        """get_next_base26('z') should return 'aa'."""
        from rez_next.utils.base26 import get_next_base26
        assert get_next_base26("z") == "aa"

    def test_get_next_base26_az(self):
        """get_next_base26('az') should return 'ba'."""
        from rez_next.utils.base26 import get_next_base26
        assert get_next_base26("az") == "ba"

    def test_get_next_base26_invalid_raises(self):
        """Invalid base26 string should raise ValueError."""
        from rez_next.utils.base26 import get_next_base26
        with pytest.raises(ValueError):
            get_next_base26("A")
        with pytest.raises(ValueError):
            get_next_base26("a1")
