"""
Tests for rez_next.package_maker module.

Tests PackageMaker, make_package, install_package, and create_package.
"""

import os
from pathlib import Path

import pytest

import rez_next as rez
from rez_next.package_maker import (
    PackageMaker,
    create_package,
    install_package,
    make_package,
)


class TestPackageMaker:
    """Tests for PackageMaker class."""

    def test_create_maker_with_name(self):
        """Test creating a PackageMaker with a name."""
        maker = PackageMaker("test_pkg")
        assert maker._data["name"] == "test_pkg"

    def test_set_version_attribute(self):
        """Test setting version attribute."""
        maker = PackageMaker("test_pkg")
        maker.version = "1.0.0"
        assert maker.version == "1.0.0"

    def test_set_description_attribute(self):
        """Test setting description attribute."""
        maker = PackageMaker("test_pkg")
        maker.description = "A test package"
        assert maker.description == "A test package"

    def test_set_requires_attribute(self):
        """Test setting requires attribute."""
        maker = PackageMaker("test_pkg")
        maker.requires = ["python-3.9", "maya-2024"]
        assert maker.requires == ["python-3.9", "maya-2024"]

    def test_get_package_basic(self):
        """Test get_package() with basic metadata."""
        maker = PackageMaker("test_pkg")
        maker.version = "1.0.0"
        maker.description = "A test package"
        maker.authors = ["Test Author"]

        pkg = maker.get_package()
        assert pkg.name == "test_pkg"
        assert pkg.version_str == "1.0.0"
        assert pkg.description == "A test package"
        assert pkg.authors == ["Test Author"]

    def test_get_package_with_requires(self):
        """Test get_package() with requires."""
        maker = PackageMaker("test_pkg")
        maker.version = "1.0.0"
        maker.requires = ["python-3.9", "maya-2024"]

        pkg = maker.get_package()
        assert pkg.name == "test_pkg"
        assert pkg.requires == ["python-3.9", "maya-2024"]

    def test_get_package_with_commands(self):
        """Test get_package() with commands."""
        maker = PackageMaker("test_pkg")
        maker.version = "1.0.0"
        maker.commands = '    env.PATH.prepend("{root}/bin")'

        pkg = maker.get_package()
        assert pkg.commands is not None
        assert "env.PATH.prepend" in pkg.commands

    def test_get_package_with_variants(self):
        """Test get_package() with variants."""
        maker = PackageMaker("test_pkg")
        maker.version = "1.0.0"
        maker.variants = [["python-3.9"], ["python-3.10"]]

        pkg = maker.get_package()
        assert pkg.variants == [["python-3.9"], ["python-3.10"]]

    def test_get_package_with_tools(self):
        """Test get_package() with tools."""
        maker = PackageMaker("test_pkg")
        maker.version = "1.0.0"
        maker.tools = ["my_tool", "another_tool"]

        pkg = maker.get_package()
        assert "my_tool" in pkg.tools
        assert "another_tool" in pkg.tools

    def test_get_package_with_flags(self):
        """Test get_package() with boolean flags."""
        maker = PackageMaker("test_pkg")
        maker.version = "1.0.0"
        maker.relocatable = True
        maker.cachable = False

        pkg = maker.get_package()
        assert pkg.relocatable is True
        assert pkg.cachable is False

    def test_get_package_with_booleans_are_set(self):
        """Test get_package() with boolean fields set via property."""
        maker = PackageMaker("test_pkg")
        maker.version = "1.0.0"
        maker.relocatable = True
        maker.cachable = True

        pkg = maker.get_package()
        assert pkg.relocatable == True
        assert pkg.cachable == True


class TestMakePackage:
    """Tests for make_package context manager."""

    def test_make_package_no_path(self, tmp_path):
        """Test make_package without a path (no disk write)."""
        with make_package("test_pkg") as maker:
            maker.version = "1.0.0"
            maker.description = "Created via make_package"
            maker.requires = ["python-3.9"]

        # No files should be written since path is None
        assert not (tmp_path / "test_pkg").exists()

    def test_make_package_with_path(self, tmp_path):
        """Test make_package with a path (disk write)."""
        install_path = tmp_path / "packages"

        with make_package("test_pkg", path=str(install_path)) as maker:
            maker.version = "1.0.0"
            maker.description = "Created via make_package"

        # Check that the package.py was written
        pkg_file = install_path / "test_pkg" / "1.0.0" / "package.py"
        assert pkg_file.exists()
        content = pkg_file.read_text()
        assert 'name = "test_pkg"' in content
        assert 'version = "1.0.0"' in content
        assert 'description = "Created via make_package"' in content

    def test_make_package_with_requires(self, tmp_path):
        """Test make_package writes correct requires."""
        install_path = tmp_path / "packages"

        with make_package("test_pkg", path=str(install_path)) as maker:
            maker.version = "2.0.0"
            maker.requires = ["python-3.9", "maya-2024"]

        pkg_file = install_path / "test_pkg" / "2.0.0" / "package.py"
        assert pkg_file.exists()
        content = pkg_file.read_text()
        assert "requires = " in content
        assert '"python-3.9"' in content
        assert '"maya-2024"' in content

    def test_make_package_with_variants(self, tmp_path):
        """Test make_package writes variant directories."""
        install_path = tmp_path / "packages"

        with make_package("test_pkg", path=str(install_path)) as maker:
            maker.version = "1.0.0"
            maker.variants = [["python-3.9"], ["python-3.10"]]

        # Check variant directories
        variant0 = install_path / "test_pkg" / "1.0.0" / "0" / "package.py"
        variant1 = install_path / "test_pkg" / "1.0.0" / "1" / "package.py"

        assert variant0.exists()
        assert variant1.exists()
        assert 0 in maker.installed_variants
        assert 1 in maker.installed_variants

    def test_make_package_skip_existing(self, tmp_path):
        """Test make_package skips existing variants."""
        install_path = tmp_path / "packages"

        # First call - creates variants
        with make_package("test_pkg", path=str(install_path)) as maker:
            maker.version = "1.0.0"
            maker.variants = [["python-3.9"], ["python-3.10"]]

        assert len(maker.installed_variants) == 2

        # Second call - should skip existing
        with make_package(
            "test_pkg", path=str(install_path), skip_existing=True
        ) as maker:
            maker.version = "1.0.0"
            maker.variants = [["python-3.9"], ["python-3.10"]]

        assert len(maker.skipped_variants) == 2

    def test_make_package_with_commands(self, tmp_path):
        """Test make_package writes correct commands."""
        install_path = tmp_path / "packages"

        with make_package("test_pkg", path=str(install_path)) as maker:
            maker.version = "1.0.0"
            maker.commands = '    env.PATH.prepend("{root}/bin")'

        pkg_file = install_path / "test_pkg" / "1.0.0" / "package.py"
        content = pkg_file.read_text()
        assert "def commands():" in content
        assert "env.PATH.prepend" in content

    def test_make_package_callbacks(self, tmp_path):
        """Test make_package invokes make_base and make_root callbacks."""
        install_path = tmp_path / "packages"
        callback_log = []

        def on_base(base_dir):
            callback_log.append(("base", base_dir))

        def on_root(root_dir):
            callback_log.append(("root", root_dir))

        with make_package(
            "test_pkg",
            path=str(install_path),
            make_base=on_base,
            make_root=on_root,
        ) as maker:
            maker.version = "1.0.0"

        assert len(callback_log) == 2
        assert callback_log[0][0] == "base"
        assert callback_log[1][0] == "root"

        # Verify callback directories exist
        assert os.path.isdir(callback_log[0][1])
        assert os.path.isdir(callback_log[1][1])

    def test_make_package_full_roundtrip(self, tmp_path):
        """Test full roundtrip: create package, verify it can be loaded."""
        install_path = tmp_path / "packages"

        with make_package("test_pkg", path=str(install_path)) as maker:
            maker.version = "3.0.0"
            maker.description = "Full roundtrip test"
            maker.requires = ["python-3.9"]
            maker.commands = '    env.PATH.prepend("{root}/bin")'
            maker.relocatable = True

        # Verify the package can be loaded
        pkg_file = install_path / "test_pkg" / "3.0.0" / "package.py"
        loaded_pkg = Package.load(str(pkg_file))
        assert loaded_pkg.name == "test_pkg"
        assert loaded_pkg.version_str == "3.0.0"
        assert loaded_pkg.description == "Full roundtrip test"
        assert "python-3.9" in loaded_pkg.requires
        assert loaded_pkg.relocatable == True


# Need to import Package for load test
from rez_next._native.packages import Package  # noqa: E402


class TestInstallPackage:
    """Tests for install_package function."""

    def test_install_package_basic(self, tmp_path):
        """Test basic install_package."""
        pkg = Package("test_pkg")
        pkg.set_version("1.0.0")
        pkg.description = "Installed package"

        result = install_package(pkg, str(tmp_path))

        pkg_file = result / "package.py"
        assert pkg_file.exists()
        assert "test_pkg" in str(result)
        assert "1.0.0" in str(result)

    def test_install_package_with_callbacks(self, tmp_path):
        """Test install_package with callbacks."""
        pkg = Package("test_pkg")
        pkg.set_version("2.0.0")
        callback_log = []

        def on_base(path):
            callback_log.append(path)

        result = install_package(pkg, str(tmp_path), make_base=on_base)

        assert len(callback_log) == 1
        assert os.path.isdir(callback_log[0])
        # base dir should exist
        assert (Path(result) / "base").exists()
        # root dir should exist too
        assert (Path(result) / "root").exists()

    def test_install_package_sets_filepath(self, tmp_path):
        """Test install_package writes correct content."""
        pkg = Package("test_pkg")
        pkg.set_version("1.0.0")
        pkg.description = "Check content"
        pkg.requires = ["python-3.9"]

        install_package(pkg, str(tmp_path))

        pkg_file = tmp_path / "test_pkg" / "1.0.0" / "package.py"
        content = pkg_file.read_text()
        assert 'name = "test_pkg"' in content
        assert 'version = "1.0.0"' in content
        assert 'description = "Check content"' in content


class TestCreatePackage:
    """Tests for create_package function."""

    def test_create_package_basic(self):
        """Test create_package with minimal data."""
        pkg = create_package({"name": "test_pkg", "version": "1.0.0"})
        assert pkg.name == "test_pkg"
        assert pkg.version_str == "1.0.0"

    def test_create_package_full(self):
        """Test create_package with full metadata."""
        pkg = create_package(
            {
                "name": "full_pkg",
                "version": "2.0.0",
                "description": "A full package",
                "authors": ["Author One"],
                "requires": ["python-3.9"],
                "build_requires": ["cmake"],
                "tools": ["cool_tool"],
                "commands": '    env.PATH.prepend("{root}/bin")',
            }
        )
        assert pkg.name == "full_pkg"
        assert pkg.version_str == "2.0.0"
        assert pkg.description == "A full package"
        assert pkg.authors == ["Author One"]
        assert pkg.requires == ["python-3.9"]
        assert pkg.build_requires == ["cmake"]
        assert pkg.tools == ["cool_tool"]
        assert pkg.commands is not None

    def test_create_package_missing_name(self):
        """Test create_package raises error without name."""
        with pytest.raises(ValueError, match="name"):
            create_package({})


class TestRezNextTopLevelImport:
    """Tests that the new API is accessible from rez_next top-level."""

    def test_make_package_imported(self):
        """Test make_package is accessible from rez_next."""
        assert hasattr(rez, "make_package")
        assert rez.make_package is make_package

    def test_install_package_imported(self):
        """Test install_package is accessible from rez_next."""
        assert hasattr(rez, "install_package")
        assert rez.install_package is install_package

    def test_create_package_imported(self):
        """Test create_package is accessible from rez_next."""
        assert hasattr(rez, "create_package")
        assert rez.create_package is create_package

    def test_package_maker_imported(self):
        """Test PackageMaker is accessible from rez_next."""
        assert hasattr(rez, "PackageMaker")
        assert rez.PackageMaker is PackageMaker
