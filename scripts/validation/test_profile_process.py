"""Real child/API integration checks for the bounded profiling observer.

No GPU/window, network, shell payload, or mock process-memory provider.
"""

import argparse
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

from profile_process import WorkingSet, observe, physical_ram
from profile_run import APP, RunLock, finite_positive, parser, run


@unittest.skipUnless(os.name == "nt", "Windows working-set API")
class ProcessTests(unittest.TestCase):
    def test_current_process_working_set_is_real_and_positive(self):
        self.assertGreater(physical_ram(), 0)
        sampler = WorkingSet(os.getpid())
        try:
            self.assertGreater(sampler.sample(), 0)
        finally:
            sampler.close()
        self.assertIsNone(sampler.handle)

    def test_live_child_samples_keep_owned_pid_and_monotonic_time(self):
        with tempfile.TemporaryDirectory() as folder:
            result = observe([sys.executable, "-c", "import time; time.sleep(2.2)"],
                             folder, folder, os.environ.copy(), 10)
            self.assertEqual(result["status"], "completed")
            rows = [json.loads(line) for line in (Path(folder) / "memory.jsonl").read_text().splitlines()]
            self.assertGreaterEqual(len(rows), 2)
            self.assertTrue(all(r["pid"] == result["pid"] and r["working_set_bytes"] > 0 for r in rows))
            self.assertEqual(sorted(r["elapsed_s"] for r in rows), [r["elapsed_s"] for r in rows])

    def test_timeout_reaps_only_owned_child_and_preserves_partial_output(self):
        with tempfile.TemporaryDirectory() as folder:
            result = observe([sys.executable, "-u", "-c", "import time; print('started'); time.sleep(30)"],
                             folder, folder, os.environ.copy(), 0.8)
            self.assertEqual(result["status"], "timeout")
            self.assertIsNotNone(result["exit_code"])
            self.assertIn("started", (Path(folder) / "stdout.txt").read_text())
            self.assertEqual(json.loads((Path(folder) / "process.json").read_text())["status"], "timeout")

    def test_process_failure_and_launch_failure_are_distinct(self):
        with tempfile.TemporaryDirectory() as folder:
            result = observe([sys.executable, "-c", "raise SystemExit(7)"],
                             folder, folder, os.environ.copy(), 5, collect=False)
            self.assertEqual((result["status"], result["exit_code"]), ("process-failed", 7))
            result = observe([str(Path(folder) / "missing.exe")],
                             folder, folder, os.environ.copy(), 5, collect=False)
            self.assertEqual(result["status"], "launch-failed")

    def test_concurrent_run_is_refused_by_os_lock(self):
        with RunLock():
            with self.assertRaises(OSError):
                with RunLock():
                    self.fail("second lock acquired")

    @unittest.skipUnless(APP.is_file(), "release app not built")
    def test_existing_evidence_directory_is_refused_before_launch(self):
        fixture = Path(__file__).parent / "fixtures" / "profile" / "short-normal.route"
        with tempfile.TemporaryDirectory() as folder:
            kept = Path(folder) / "kept.txt"
            kept.write_text("original")
            with self.assertRaisesRegex(ValueError, "already exists"):
                run(parser().parse_args(["--route", str(fixture), "--out", folder]))
            self.assertEqual(sorted(p.name for p in Path(folder).iterdir()), ["kept.txt"])
            self.assertEqual(kept.read_text(), "original")

    def test_nonfinite_and_nonpositive_timeout_are_rejected(self):
        for text in ("nan", "inf", "-1", "0"):
            with self.assertRaises(argparse.ArgumentTypeError):
                finite_positive(text)


if __name__ == "__main__":
    unittest.main()
