"""Exercise result publication against real, local-only Git repositories."""

import json
import os
import shlex
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("publish_results.py").resolve()


class PublishResultsTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        root = Path(self.temp.name)
        self.remote = root / "remote.git"
        self.writer = root / "writer"
        self.repo = root / "measurement"
        self.env = dict(
            os.environ, GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull
        )
        self.run_git(root, "init", "--bare", str(self.remote))
        self.run_git(root, "init", "--initial-branch=main", str(self.writer))
        self.run_git(self.writer, "config", "user.name", "Test")
        self.run_git(self.writer, "config", "user.email", "test@example.invalid")
        (self.writer / "README.md").write_text("measured source\n", encoding="utf-8")
        self.run_git(self.writer, "add", "README.md")
        self.run_git(self.writer, "commit", "-m", "measured source")
        self.run_git(self.writer, "remote", "add", "origin", str(self.remote))
        self.run_git(self.writer, "push", "origin", "main")
        self.run_git(
            root,
            "--git-dir",
            str(self.remote),
            "symbolic-ref",
            "HEAD",
            "refs/heads/main",
        )
        self.run_git(root, "clone", str(self.remote), str(self.repo))
        self.sha = self.run_git(self.repo, "rev-parse", "HEAD").stdout.strip()
        self.short_sha = self.run_git(
            self.repo, "rev-parse", "--short", "HEAD"
        ).stdout.strip()
        self.report = self.repo / "metrics/benchmarking/RESULTS.md"
        self.summary = (
            self.repo
            / f"metrics/benchmarking/artifacts/2026.10.12-{self.short_sha}/summary.json"
        )
        self.summary.parent.mkdir(parents=True)
        self.report.write_text(
            f"<!-- Commit: {self.short_sha} -->\nMeasured results\n", encoding="utf-8"
        )
        self.summary.write_text(
            json.dumps({"commit": self.short_sha}), encoding="utf-8"
        )

    def run_git(self, cwd, *args):
        return subprocess.run(
            ["git", *args],
            cwd=cwd,
            env=self.env,
            check=True,
            capture_output=True,
            text=True,
        )

    def publish(self):
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--measured-sha", self.sha],
            cwd=self.repo,
            env=self.env,
            check=False,
            capture_output=True,
            text=True,
        )

    def remote_head(self):
        return self.run_git(
            self.writer, "ls-remote", "origin", "refs/heads/main"
        ).stdout.split()[0]

    def advance_main(self, destination="main"):
        (self.writer / "newer.txt").write_text("new source\n", encoding="utf-8")
        self.run_git(self.writer, "add", "newer.txt")
        self.run_git(self.writer, "commit", "-m", "newer source")
        sha = self.run_git(self.writer, "rev-parse", "HEAD").stdout.strip()
        self.run_git(self.writer, "push", "origin", f"HEAD:refs/heads/{destination}")
        return sha

    def test_current_measurement_publishes_only_result_paths(self):
        (self.repo / "unrelated.txt").write_text("do not publish\n", encoding="utf-8")
        result = self.publish()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Published results", result.stdout)
        self.assertNotEqual(self.remote_head(), self.sha)
        changed = self.run_git(
            self.repo, "diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD"
        ).stdout
        self.assertEqual(
            set(changed.splitlines()),
            {
                "metrics/benchmarking/RESULTS.md",
                f"metrics/benchmarking/artifacts/2026.10.12-{self.short_sha}/summary.json",
            },
        )
        message = self.run_git(self.repo, "log", "-1", "--format=%s").stdout
        self.assertIn(self.sha, message)
        self.assertIn("[skip ci]", message)

    def test_already_stale_measurement_leaves_main_untouched(self):
        newer = self.advance_main()
        result = self.publish()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Stale measurement", result.stdout)
        self.assertEqual(self.remote_head(), newer)
        self.assertEqual(
            self.run_git(self.repo, "rev-parse", "HEAD").stdout.strip(), self.sha
        )
        self.assertTrue(self.report.exists())

    def test_main_advancing_during_push_is_not_overwritten(self):
        newer = self.advance_main("race-source")
        hook = self.repo / ".git/hooks/pre-push"
        hook.write_text(
            "#!/bin/sh\n"
            f"git --git-dir={shlex.quote(str(self.remote))} update-ref refs/heads/main {newer} {self.sha}\n",
            encoding="utf-8",
        )
        hook.chmod(0o755)
        result = self.publish()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("became stale", result.stdout)
        self.assertEqual(self.remote_head(), newer)

    def test_real_push_failure_is_not_hidden(self):
        hook = self.remote / "hooks/pre-receive"
        hook.write_text("#!/bin/sh\nexit 1\n", encoding="utf-8")
        hook.chmod(0o755)
        result = self.publish()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Result push failed", result.stderr)
        self.assertEqual(self.remote_head(), self.sha)

    def test_wrong_report_commit_is_rejected(self):
        self.report.write_text(f"<!-- Commit: {'0' * 40} -->\n", encoding="utf-8")
        result = self.publish()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.remote_head(), self.sha)

    def test_wrong_summary_commit_is_rejected(self):
        self.summary.write_text(json.dumps({"commit": "wrong"}), encoding="utf-8")
        result = self.publish()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Measured commit mismatch", result.stderr)
        self.assertEqual(self.remote_head(), self.sha)

    def test_missing_summary_is_rejected(self):
        self.summary.unlink()
        result = self.publish()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("No matching", result.stderr)
        self.assertEqual(self.remote_head(), self.sha)

    def test_staged_unrelated_changes_are_rejected(self):
        (self.repo / "unrelated.txt").write_text("do not publish\n", encoding="utf-8")
        self.run_git(self.repo, "add", "unrelated.txt")
        result = self.publish()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("pre-existing staged changes", result.stderr)
        self.assertEqual(self.remote_head(), self.sha)

    def test_wrong_measured_checkout_is_rejected(self):
        original = self.sha
        self.sha = "0" * 40
        result = self.publish()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Checkout HEAD does not match", result.stderr)
        self.assertEqual(self.remote_head(), original)

    def test_fetch_failure_is_not_hidden(self):
        self.run_git(
            self.repo, "remote", "set-url", "origin", str(self.remote / "missing")
        )
        result = self.publish()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("git fetch failed", result.stderr)
        self.assertEqual(self.remote_head(), self.sha)


if __name__ == "__main__":
    unittest.main()
