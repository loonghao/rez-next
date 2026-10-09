"""Trust-boundary checks for the packaged SDK manifest."""

import importlib.util
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    "check_runtime_package", Path(__file__).resolve().parents[1] / "check_runtime_package.py"
)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class RuntimePackageContract(unittest.TestCase):
    def test_public_registry_dependencies_are_accepted(self):
        module.validate_manifest({
            "dependencies": {"serde": "1", "core": {"version": "0.3"}},
            "target": {"cfg(windows)": {"build-dependencies": {"win": "1"}}},
        })

    def test_non_registry_sources_are_rejected_in_every_dependency_scope(self):
        for table in ("dependencies", "dev-dependencies", "build-dependencies"):
            for source in ("path", "git", "registry", "registry-index"):
                dependency = {table: {"core": {"version": "0.3", source: "fixture"}}}
                for manifest in (dependency, {"target": {"cfg(windows)": dependency}}):
                    with self.subTest(table=table, source=source, manifest=manifest):
                        with self.assertRaisesRegex(ValueError, "must use crates.io"):
                            module.validate_manifest(manifest)

    def test_dependency_replacements_are_rejected(self):
        for table in ("patch", "replace"):
            with self.subTest(table=table):
                with self.assertRaisesRegex(ValueError, "without replacements"):
                    module.validate_manifest({table: {"core": {"path": "fixture"}}})


if __name__ == "__main__":
    unittest.main()
