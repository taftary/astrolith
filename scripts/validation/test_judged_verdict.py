"""Hermetic public-entrypoint fixtures for all-judged verdicts (#1235).

The validation and posting code runs unchanged, writing real temporary evidence.
Only external cargo/git/CI dependencies are fixed unit-test inputs: these are
classification regressions, not evidence that the real application ran.

Usage: python scripts/validation/test_judged_verdict.py
"""

import contextlib
import io
import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "gates"))

import post_verdict  # noqa: E402
import validate  # noqa: E402
from compare_verdict import extract_span  # noqa: E402
from merge_gate import MARKER, select_verdict, verdict_problems  # noqa: E402

SHA = "a" * 40
OTHER_SHA = "b" * 40
CHECKLIST = """# Requirements (CONFIRMED) - issue #1

- [ ] C1: The player moves (judged by the validator from movement evidence)
- [ ] C2: The camera stays separate (judged by the validator from view evidence)
"""
JUDGMENTS = ["C1=MET: movement evidence checked", "C2=MET: view evidence checked"]


@contextlib.contextmanager
def generated(checklist=CHECKLIST, *, head=SHA, dirty=False, test_rc=0,
              verify_runs=None, python_rc=0):
    """Run the real entrypoint with temporary files and fixed external inputs."""
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        agent = root / ".agent" / "validation"
        issue = agent / "issue-1"
        issue.mkdir(parents=True)
        (issue / "requirements.md").write_text(checklist, encoding="utf-8")
        fixture = root / "scripts" / "test_sample.py"
        fixture.parent.mkdir()
        fixture.write_text("# External fixture input, not executed.\n", encoding="utf-8")
        verifies = iter(verify_runs or [(0, "VERIFY-OK\n", ""), (0, "VERIFY-OK\n", "")])

        def external(cmd, timeout=600):
            if cmd[:2] == ["cargo", "test"]:
                return test_rc, "test result: ok. 1 passed; 0 failed\n", ""
            if cmd[:2] == ["cargo", "run"]:
                return next(verifies)
            if cmd == [sys.executable, "scripts/test_sample.py"]:
                return python_rc, "fixture output\n", ""
            if cmd == ["git", "rev-parse", "HEAD"]:
                return 0, head + "\n", ""
            if cmd == ["git", "status", "--porcelain", "--untracked-files=no"]:
                return 0, " M tracked.py\n" if dirty else "", ""
            raise AssertionError(f"unexpected external command: {cmd}")

        with (patch.object(validate, "ROOT", root),
              patch.object(validate, "AGENT", agent),
              patch.object(post_verdict, "AGENT", agent),
              patch.object(validate, "run", external),
              patch.object(validate, "ci_test_log", return_value=(None, None)),
              patch.object(shutil, "disk_usage", return_value=SimpleNamespace(free=10**10))):
            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                rc = validate.main(["--issue", "1", "--sha", SHA])
            summary = json.loads(stdout.getvalue().splitlines()[0])
            run_dir = Path(summary["verdict_md"]).parent
            yield SimpleNamespace(
                rc=rc, summary=summary, run_dir=run_dir,
                verdict=(run_dir / "verdict.md").read_text(encoding="utf-8"),
                report=(run_dir / "report.md").read_text(encoding="utf-8"),
            )


def prepare(judgments=JUDGMENTS, *, drift="none", dry_run=True):
    """Use the documented posting preparation; no Issue is posted by fixtures."""
    args = ["--issue", "1", "--sha", SHA, "--drift", drift]
    if dry_run:
        args += ["--dry-run"]
    for judgment in judgments:
        args += ["--judge", judgment]
    stdout = io.StringIO()
    with contextlib.redirect_stdout(stdout):
        rc = post_verdict.main(args)
    return rc, json.loads(stdout.getvalue())


class JudgedVerdictTests(unittest.TestCase):
    def assert_unsuccessful(self, body):
        self.assertIsNone(MARKER.search(body))
        self.assertNotIn("Validator: PASS ", body)
        self.assertTrue(verdict_problems(body))

    def test_entirely_judged_checklist_passes_after_all_met(self):
        with generated() as run:
            self.assertIn("Runtime: PASS\n", run.verdict)
            self.assertIn("Validator: FAIL ", run.verdict)
            self.assertIn("Requirements: MET 0/2\n", run.verdict)
            self.assertIn("awaiting explicit validator judgment", run.verdict)
            original = (run.run_dir / "verdict.md").read_bytes()
            rc, result = prepare()
            self.assertEqual(rc, 0)
            body = result["body"]
            self.assertIn("Runtime: PASS\n", body)
            self.assertIn(f"Validator: PASS for {SHA}\n", body)
            self.assertIn("Requirements: MET 2/2\n", body)
            self.assertEqual(MARKER.search(body).group(1), SHA)
            self.assertEqual(verdict_problems(body), [])
            self.assertEqual(select_verdict([body], OTHER_SHA), None)
            span, problem = extract_span(body)
            self.assertFalse(problem)
            self.assertEqual(span, result["block"])
            self.assertIn("- MET (judged): C1:", body)
            self.assertIn("(judged: movement evidence checked)", body)
            self.assertNotIn("awaiting explicit validator judgment", body)
            self.assertNotIn("no criterion verifiable", body)
            self.assertNotIn("no criterion verifiable", run.report)
            self.assertEqual((run.run_dir / "verdict.md").read_bytes(), original)

    def test_missing_or_negative_judgments_never_succeed(self):
        with generated():
            for judgments, count in (
                ([], "0/2"),
                (JUDGMENTS[:1], "1/2"),
                ([JUDGMENTS[0], "C2=NOT MET: view moved incorrectly"], "1/2"),
            ):
                with self.subTest(judgments=judgments):
                    rc, result = prepare(judgments)
                    self.assertEqual(rc, 0)
                    self.assert_unsuccessful(result["body"])
                    self.assertIn("Runtime: PASS\n", result["body"])
                    self.assertIn(f"Requirements: MET {count}\n", result["body"])

    def test_mixed_checklist_and_automated_override_refusal(self):
        automated = "- [ ] C3: Health signal (verify: contains VERIFY-OK in verify.log)\n"
        with generated(CHECKLIST + automated):
            rc, result = prepare()
            self.assertEqual(rc, 0)
            self.assertIn("Requirements: MET 3/3\n", result["body"])
            self.assertEqual(verdict_problems(result["body"]), [])
            self.assertEqual(MARKER.search(result["body"]).group(1), SHA)
            rc, result = prepare(JUDGMENTS + ["C3=MET: hand override"])
            self.assertEqual(rc, 1)
            self.assertIn("cannot be judged by hand", result["error"])

    def test_genuine_runtime_problems_cannot_be_cleared(self):
        healthy = (0, "VERIFY-OK\n", "")
        problems = [
            {"test_rc": 1},
            {"python_rc": 1},
            {"verify_runs": [(1, "VERIFY-FAIL\n", ""), healthy]},
            {"verify_runs": [(1, "VERIFY-OK\n", ""), healthy]},
            {"verify_runs": [(0, "", ""), healthy]},
            {"verify_runs": [healthy, (1, "VERIFY-OK\n", "")]},
            {"verify_runs": [healthy, (0, "", "")]},
            {"verify_runs": [healthy, (0, "changed\nVERIFY-OK\n", "")]},
            {"verify_runs": [(0, "VERIFY-OK\n", "error: real diagnostic\n"), healthy]},
        ]
        for problem in problems:
            with self.subTest(problem=problem), generated(**problem) as run:
                rc, result = prepare()
                self.assertEqual(rc, 0)
                self.assertTrue(run.summary["errors"])
                self.assertIn("Runtime: FAIL\n", result["body"])
                self.assertIn("Requirements: MET 2/2\n", result["body"])
                self.assert_unsuccessful(result["body"])

    def test_wrong_commit_or_dirty_tracked_tree_remains_blocked(self):
        for identity in ({"head": OTHER_SHA}, {"dirty": True}):
            with self.subTest(identity=identity), generated(**identity) as run:
                self.assertEqual(run.rc, 2)
                self.assertTrue(run.summary["notes"])
                rc, result = prepare()
                self.assertEqual(rc, 0)
                self.assertIn("Runtime: BLOCKED\n", result["body"])
                self.assertIn("Validator: BLOCKED ", result["body"])
                for reason in run.summary["notes"]:
                    self.assertIn(reason, result["body"])
                self.assert_unsuccessful(result["body"])

    def test_empty_or_unconfirmed_checklist_remains_blocked(self):
        unconfirmed = CHECKLIST + "\n> Status: UNCONFIRMED\n"
        for checklist in ("# Empty checklist\n", unconfirmed):
            with self.subTest(checklist=checklist), generated(checklist) as run:
                self.assertEqual(run.rc, 2)
                rc, result = prepare(JUDGMENTS if checklist == unconfirmed else [])
                self.assertEqual(rc, 0)
                self.assertIn("Runtime: BLOCKED\n", result["body"])
                self.assert_unsuccessful(result["body"])
                count = "2/3" if checklist == unconfirmed else "0/1"
                self.assertIn(f"Requirements: MET {count}\n", result["body"])

    def test_unmet_automated_criterion_cannot_be_overridden(self):
        automated = ("- [ ] C3: Required result (judged by the validator) "
                     "(verify: contains missing-signal in verify.log)\n")
        with generated(CHECKLIST + automated):
            rc, result = prepare()
            self.assertEqual(rc, 0)
            self.assertIn("Requirements: MET 2/3\n", result["body"])
            self.assert_unsuccessful(result["body"])
            rc, result = prepare(JUDGMENTS + ["C3=MET: override"])
            self.assertEqual(rc, 1)
            self.assertIn("cannot be judged by hand", result["error"])

    def test_unavailable_automated_probe_is_not_a_hand_judgment(self):
        for probe in ("contains signal in unknown.log", "file missing.txt contains signal"):
            checklist = ("# Requirements (CONFIRMED)\n\n"
                         f"- [ ] C1: Required result (judged by the validator) (verify: {probe})\n")
            with self.subTest(probe=probe), generated(checklist) as run:
                self.assertEqual(run.rc, 2)
                self.assertIn("no criterion verifiable", run.verdict)
                rc, result = prepare([JUDGMENTS[0]])
                self.assertEqual(rc, 1)
                self.assertIn("cannot be judged by hand", result["error"])

    def test_unmapped_unverifiable_row_is_still_a_blocker(self):
        with generated("# Requirements (CONFIRMED)\n\n- [ ] C1: No probe or judgment\n") as run:
            self.assertEqual(run.rc, 2)
            self.assertIn("no criterion verifiable", run.verdict)
            rc, result = prepare([JUDGMENTS[0]])
            self.assertEqual(rc, 1)
            self.assertIn("cannot be judged by hand", result["error"])

    def test_drift_turns_all_met_into_non_success(self):
        with generated():
            rc, result = prepare(drift="unrequested behavior")
            self.assertEqual(rc, 0)
            self.assertIn("Requirements: MET 2/2\n", result["body"])
            self.assertIn("Runtime: PASS\n", result["body"])
            self.assertIn("Drift: unrequested behavior\n", result["body"])
            self.assert_unsuccessful(result["body"])

    def test_invalid_judgments_are_refused(self):
        with generated():
            for judges in (["C1=MET:"], ["C1=MET:   "], ["C9=MET: unknown"], ["C1=PARTIAL: x"]):
                with self.subTest(judges=judges):
                    rc, result = prepare(judges)
                    self.assertEqual(rc, 1)
                    self.assertFalse(result["ok"])

    def test_public_post_preserves_evidence_and_duplicate_guard(self):
        with generated() as run:
            original = (run.run_dir / "verdict.md").read_bytes()
            posted = []

            def github(cmd):
                if cmd[:3] == ["gh", "issue", "view"]:
                    return SimpleNamespace(returncode=0, stdout="[]", stderr="")
                self.assertEqual(cmd[:3], ["gh", "issue", "comment"])
                body = Path(cmd[cmd.index("--body-file") + 1]).read_text(encoding="utf-8")
                posted.append(body)
                return SimpleNamespace(returncode=0, stdout="https://example.test/comment", stderr="")

            with patch.object(post_verdict, "run", github):
                rc, result = prepare(dry_run=False)
            self.assertEqual(rc, 0)
            self.assertTrue(result["posted"])
            self.assertEqual(len(posted), 1)
            self.assertEqual(verdict_problems(posted[0]), [])
            self.assertEqual(MARKER.search(posted[0]).group(1), SHA)
            self.assertEqual(extract_span(posted[0])[0], result["block"])
            self.assertEqual((run.run_dir / "verdict.md").read_bytes(), original)
            with patch.object(post_verdict, "already_posted", return_value=True):
                rc, result = prepare(dry_run=False)
            self.assertEqual(rc, 1)
            self.assertIn("replacement needs --owner-decision", result["error"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
