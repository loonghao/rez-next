"""
Tests for schema-related functions in rez_next.packages_
"""

import pytest
import rez_next as rez
from rez_next._native import packages_


class TestPackageSchema:
    """Tests for packages_.package_schema()"""

    def test_returns_set(self):
        """package_schema() should return a set"""
        result = packages_.package_schema()
        assert isinstance(result, set)
        assert len(result) > 0

    def test_contains_required_keys(self):
        """package_schema() must contain required keys"""
        result = packages_.package_schema()
        required_keys = [
            "name",
            "version",
            "description",
            "authors",
            "requires",
            "build_requires",
            "tools",
            "commands",
            "uuid",
            "timestamp",
        ]
        for key in required_keys:
            assert key in result, f"package_schema() must contain '{key}'"

    def test_does_not_contain_invalid_key(self):
        """package_schema() must not contain invalid keys"""
        result = packages_.package_schema()
        assert "invalid_key_xyz" not in result


class TestVariantSchema:
    """Tests for packages_.variant_schema()"""

    def test_returns_set(self):
        """variant_schema() should return a set"""
        result = packages_.variant_schema()
        assert isinstance(result, set)
        assert len(result) > 0

    def test_same_as_package_schema(self):
        """variant_schema() should return the same keys as package_schema()"""
        pkg_schema = packages_.package_schema()
        var_schema = packages_.variant_schema()
        assert pkg_schema == var_schema


class TestPackageFamilySchema:
    """Tests for packages_.package_family_schema()"""

    def test_returns_set(self):
        """package_family_schema() should return a set"""
        result = packages_.package_family_schema()
        assert isinstance(result, set)
        assert len(result) == 1

    def test_only_name(self):
        """package_family_schema() must only contain 'name'"""
        result = packages_.package_family_schema()
        assert result == {"name"}


class TestSchemaKeys:
    """Tests for packages_.schema_keys()"""

    def test_with_string_keys(self):
        """schema_keys(dict) should extract string keys"""
        test_dict = {
            "name": "test",
            "version": "1.0.0",
            "description": "Test package",
        }
        result = packages_.schema_keys(test_dict)
        assert isinstance(result, set)
        assert len(result) == 3
        assert "name" in result
        assert "version" in result
        assert "description" in result

    def test_ignores_non_string_keys(self):
        """schema_keys(dict) should ignore non-string keys"""
        test_dict = {
            "name": "test",
            123: "value",  # non-string key
        }
        result = packages_.schema_keys(test_dict)
        assert len(result) == 1
        assert "name" in result
        assert 123 not in result

    def test_with_empty_dict(self):
        """schema_keys({}) should return empty set"""
        result = packages_.schema_keys({})
        assert isinstance(result, set)
        assert len(result) == 0


class TestPackageReleaseKeys:
    """Tests for packages_.package_release_keys()"""

    def test_returns_set(self):
        """package_release_keys() should return a set"""
        result = packages_.package_release_keys()
        assert isinstance(result, set)
        assert len(result) > 0

    def test_contains_version(self):
        """package_release_keys() must contain 'version'"""
        result = packages_.package_release_keys()
        assert "version" in result
        assert "requires" in result
        assert "build_requires" in result

    def test_does_not_contain_name(self):
        """package_release_keys() must not contain 'name'"""
        result = packages_.package_release_keys()
        assert "name" not in result


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
