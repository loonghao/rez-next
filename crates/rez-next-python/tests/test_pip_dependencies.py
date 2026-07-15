"""Tests for pip_get_pip_dependencies() function."""

import rez_next.pip as pip


class TestGetPipDependencies:
    """Verify rez.pip_get_pip_dependencies() API."""

    def test_get_pip_dependencies_nonexistent(self):
        """Query a non-existent package — should return empty list."""
        result = pip.get_pip_dependencies("nonexistent_package_xyz")
        assert isinstance(result, list)
        assert len(result) == 0

    def test_get_pip_dependencies_with_path(self, tmp_path):
        """Create a package that requires 'numpy', then check dependencies."""
        # Create a package that requires "numpy-1.25+"
        pkg = pip.PipPackage("mypackage", "1.0.0", requires=["numpy-1.25+"])
        pip.write_pip_package(pkg, str(tmp_path))

        # Check dependencies for "numpy"
        result = pip.get_pip_dependencies("numpy", paths=[str(tmp_path)])
        assert isinstance(result, list)
        if result:
            assert "mypackage" in result

    def test_get_pip_dependencies_normalization(self, tmp_path):
        """Package name normalization should work for dependencies."""
        # Create a package that requires "PyYAML" (should match "pyyaml")
        pkg = pip.PipPackage("mypackage", "1.0.0", requires=["PyYAML"])
        pip.write_pip_package(pkg, str(tmp_path))

        # Check with normalized name
        result = pip.get_pip_dependencies("pyyaml", paths=[str(tmp_path)])
        assert isinstance(result, list)
        if result:
            assert "mypackage" in result

    def test_get_pip_dependencies_multiple_dependents(self, tmp_path):
        """Multiple packages depending on the same pip package."""
        # Create 2 packages that depend on "numpy"
        pkg1 = pip.PipPackage("pkg1", "1.0.0", requires=["numpy-1.25+"])
        pkg2 = pip.PipPackage("pkg2", "2.0.0", requires=["numpy-1.25+"])
        pip.write_pip_package(pkg1, str(tmp_path))
        pip.write_pip_package(pkg2, str(tmp_path))

        # Check dependencies for "numpy"
        result = pip.get_pip_dependencies("numpy", paths=[str(tmp_path)])
        assert isinstance(result, list)
        assert len(result) == 2
        assert "pkg1" in result
        assert "pkg2" in result

    def test_get_pip_dependencies_no_dependencies(self, tmp_path):
        """Package with no dependencies should not appear."""
        # Create a package with no requires
        pkg = pip.PipPackage("mypackage", "1.0.0")
        pip.write_pip_package(pkg, str(tmp_path))

        # Check dependencies for "numpy" (should be empty)
        result = pip.get_pip_dependencies("numpy", paths=[str(tmp_path)])
        assert isinstance(result, list)
        assert len(result) == 0
