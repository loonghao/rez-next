"""Performance benchmark tests for rez_next vs rez."""

import rez_next as rez
import rez_next.pip as pip


class TestPerformanceBenchmark:
    """Benchmark key operations in rez_next."""

    def test_pip_install_performance(self, benchmark):
        """Benchmark pip_install() with multiple packages."""
        packages = ["numpy==1.25.0", "scipy==1.11.0"]

        def run():
            return pip.pip_install(packages)

        result = benchmark(run)
        assert isinstance(result, list)
        assert len(result) == 2

    def test_get_pip_dependencies_performance(self, benchmark, tmp_path):
        """Benchmark get_pip_dependencies() with a small repo."""
        # Create a package that requires "numpy"
        pkg = pip.PipPackage("mypackage", "1.0.0", requires=["numpy-1.25+"])
        pip.write_pip_package(pkg, str(tmp_path))

        def run():
            return pip.get_pip_dependencies("numpy", paths=[str(tmp_path)])

        result = benchmark(run)
        assert isinstance(result, list)

    def test_walk_packages_performance(self, benchmark):
        """Benchmark walk_packages()."""

        # This uses the default package paths (might be empty)
        def run():
            return rez.walk_packages(paths=["/nonexistent/path_xyz"])

        result = benchmark(run)
        assert isinstance(result, list)
