"""Illustrative receipts test verification, never substitute for a real build."""

import json
from pathlib import Path
import tempfile
import unittest

from build_receipt import COMMAND, verify, verify_record
from profile_run import digest, git


class BuildReceiptTests(unittest.TestCase):
    def test_source_command_completion_and_exact_executable_are_required(self):
        commit, tree = git("rev-parse", "HEAD"), git("rev-parse", "HEAD^{tree}")
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            executable, receipt = root / "illustrative-not-an-app", root / "receipt.json"
            executable.write_bytes(b"illustrative verifier input, not application evidence")
            record = {"schema": "main-agent-release-build-v1", "command": COMMAND, "source_commit": commit,
                      "source_commit_after": commit, "source_tree": tree, "source_tree_after": tree,
                      "tracked_before": "", "tracked_after": "", "exit_code": 0,
                      "rustc": "illustrative", "cargo": "illustrative", "cwd": str(root),
                      "started_utc": "2026-11-07T00:00:00+00:00", "completed_utc": "2026-11-07T00:01:00+00:00",
                      "executable_sha256": digest(executable), "executable_bytes": executable.stat().st_size}

            def write(value):
                receipt.write_text(json.dumps(value), encoding="utf-8")

            write(record)
            self.assertTrue(verify(receipt, executable, commit)["verified"])
            self.assertTrue(verify_record(receipt, commit, digest(executable), executable.stat().st_size)["verified"])
            for key, value in (("command", COMMAND + ["&&", "anything"]), ("source_commit_after", "0" * 40),
                               ("source_tree", "0" * 40), ("tracked_before", " M source.rs"),
                               ("tracked_after", " M source.rs"), ("exit_code", 1), ("rustc", ""),
                               ("source_commit", None), ("completed_utc", "2026-11-06T00:01:00+00:00"),
                               ("executable_sha256", "0" * 64), ("executable_bytes", 1)):
                write({**record, key: value})
                with self.subTest(key=key), self.assertRaises(ValueError):
                    verify(receipt, executable, commit)
            write(record)
            with self.assertRaisesRegex(ValueError, "source commit/tree mismatch"):
                verify(receipt, executable, "0" * 40)
            executable.write_bytes(b"changed executable")
            with self.assertRaisesRegex(ValueError, "hash/size mismatch"):
                verify(receipt, executable, commit)


if __name__ == "__main__":
    unittest.main()
