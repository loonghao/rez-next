import io
import unittest
from contextlib import redirect_stderr, redirect_stdout
from unittest.mock import patch

from parse_criterion import main, parse


class ParseCriterionTests(unittest.TestCase):
    def test_parses_grouped_results_and_attributes_regressions(self) -> None:
        suite = parse(
            """
rex_parser_construction time:   [1.0228 ms 1.0234 ms 1.0241 ms]
package_creation/simple_package
                        time:   [69.107 ns 69.206 ns 69.286 ns]
                        change: [+11.066% +12.097% +12.790%] (p = 0.00 < 0.05)
                        Performance has regressed.
package_creation/package_with_version
                        time:   [613.17 ns 613.97 ns 614.56 ns]
                        change: [+1.5965% +2.3301% +2.9992%] (p = 0.00 < 0.05)
                        Performance has regressed.
""".splitlines()
        )

        self.assertEqual(
            [result.name for result in suite.results],
            [
                "rex_parser_construction",
                "package_creation/simple_package",
                "package_creation/package_with_version",
            ],
        )
        self.assertEqual(
            suite.regressions,
            [
                "package_creation/simple_package",
                "package_creation/package_with_version",
            ],
        )
        self.assertEqual(suite.results[1].change_pct, 12.097)

    def test_supports_ansi_picoseconds_and_unicode_minus(self) -> None:
        suite = parse(
            [
                "\x1b[1mpackage_variants/access_variants\x1b[0m",
                "                        time:   [703.53 ps 704.47 ps 705.51 ps]",
                "                        change: [−0.3107% −0.0373% +0.3183%] (p = 0.82 > 0.05)",
                "                        Performance has improved.",
            ]
        )

        self.assertEqual(len(suite.results), 1)
        result = suite.results[0]
        self.assertEqual(result.name, "package_variants/access_variants")
        self.assertAlmostEqual(result.mean_ns, 0.70447)
        self.assertEqual(result.change_pct, -0.0373)
        self.assertTrue(result.improved)

    def test_records_each_regression_once(self) -> None:
        suite = parse(
            [
                "example time: [1.0 ns 2.0 ns 3.0 ns]",
                "Performance has regressed.",
                "Performance has regressed.",
            ]
        )

        self.assertEqual(suite.regressions, ["example"])

    def test_cli_only_fails_on_regression_when_requested(self) -> None:
        benchmark_output = """example time: [1.0 ns 2.0 ns 3.0 ns]
Performance has regressed.
"""

        with (
            patch("sys.stdin", io.StringIO(benchmark_output)),
            redirect_stdout(io.StringIO()),
            redirect_stderr(io.StringIO()),
        ):
            self.assertEqual(main([]), 0)

        with (
            patch("sys.stdin", io.StringIO(benchmark_output)),
            redirect_stdout(io.StringIO()),
            redirect_stderr(io.StringIO()),
        ):
            self.assertEqual(main(["--fail-on-regression"]), 1)


if __name__ == "__main__":
    unittest.main()
