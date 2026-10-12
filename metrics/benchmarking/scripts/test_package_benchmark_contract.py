"""Cheap source checks prevent cross-executable Criterion baseline collisions."""

import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
TARGETS = ("package_benchmark.rs", "simple_package_benchmark.rs")


class PackageBenchmarkContractTests(unittest.TestCase):
    def test_package_executables_have_disjoint_group_ids(self):
        groups = {}
        for target in TARGETS:
            source = (ROOT / "benches" / target).read_text(encoding="utf-8")
            names = re.findall(r'benchmark_group\("([^"\n]+)"\)', source)
            self.assertEqual(len(names), 7, target)
            self.assertEqual(len(names), len(set(names)), target)
            groups[target] = set(names)
        self.assertFalse(groups[TARGETS[0]] & groups[TARGETS[1]])

    def test_package_executables_share_sampling_configuration(self):
        for target in TARGETS:
            source = (ROOT / "benches" / target).read_text(encoding="utf-8")
            self.assertIn('#[path = "support/package_criterion.rs"]', source)
            self.assertIn("config = package_criterion::configured();", source)
            self.assertNotIn(".sample_size(", source)
            self.assertNotIn(".measurement_time(", source)
            self.assertNotIn(".warm_up_time(", source)


if __name__ == "__main__":
    unittest.main()
