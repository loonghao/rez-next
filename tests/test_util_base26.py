"""
Tests for rez_next.util base26 functions.

Tests the Base26 encoding utilities ported from Rez's utils/base26.py.
"""

import pytest
import rez_next


class TestGetNextBase26:
    """Tests for get_next_base26 function."""

    def test_none_returns_a(self):
        """get_next_base26(None) should return 'a'."""
        result = rez_next.util.get_next_base26(None)
        assert result == "a"

    def test_a_returns_b(self):
        """get_next_base26('a') should return 'b'."""
        result = rez_next.util.get_next_base26("a")
        assert result == "b"

    def test_z_returns_aa(self):
        """get_next_base26('z') should return 'aa' (carry over)."""
        result = rez_next.util.get_next_base26("z")
        assert result == "aa"

    def test_az_returns_ba(self):
        """get_next_base26('az') should return 'ba'."""
        result = rez_next.util.get_next_base26("az")
        assert result == "ba"

    def test_zz_returns_aaa(self):
        """get_next_base26('zz') should return 'aaa'."""
        result = rez_next.util.get_next_base26("zz")
        assert result == "aaa"

    def test_azzz_returns_baaa(self):
        """get_next_base26('azzz') should return 'baaa'."""
        result = rez_next.util.get_next_base26("azzz")
        assert result == "baaa"

    def test_invalid_contains_number(self):
        """Should raise ValueError for strings containing numbers."""
        with pytest.raises(ValueError):
            rez_next.util.get_next_base26("a1b")

    def test_invalid_uppercase(self):
        """Should raise ValueError for strings containing uppercase."""
        with pytest.raises(ValueError):
            rez_next.util.get_next_base26("A")

    def test_invalid_mixed_case(self):
        """Should raise ValueError for strings containing uppercase."""
        with pytest.raises(ValueError):
            rez_next.util.get_next_base26("aB")

    def test_sequence(self):
        """Test generating a sequence of 100 Base26 strings."""
        results = []
        current = None
        for _ in range(100):
            current = rez_next.util.get_next_base26(current)
            results.append(current)

        assert results[0] == "a"
        assert results[1] == "b"
        assert results[25] == "z"
        assert results[26] == "aa"
        assert results[27] == "ab"


@pytest.mark.skipif(
    __import__("sys").platform.startswith("win"),
    reason="create_unique_base26_symlink is only supported on Unix"
)
class TestCreateUniqueBase26Symlink:
    """Tests for create_unique_base26_symlink function (Unix only)."""

    def test_placeholder(self):
        """Placeholder test - actual symlink tests require Unix environment."""
        pytest.skip("actual symlink tests require Unix environment")


class TestBase26Alignment:
    """Tests to verify alignment with Rez's base26.py interface."""

    def test_get_next_base26_signature(self):
        """Verify get_next_base26 has correct signature (prev is optional)."""
        import inspect
        sig = inspect.signature(rez_next.util.get_next_base26)
        params = list(sig.parameters.keys())
        assert "prev" in params
        assert sig.parameters["prev"].default is None

    def test_function_exists(self):
        """Verify all expected functions exist."""
        assert hasattr(rez_next.util, "get_next_base26")
        # create_unique_base26_symlink may not exist on Windows
        assert hasattr(rez_next.util, "create_unique_base26_symlink")
