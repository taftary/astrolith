"""Hermetic completion/CLI fixtures. Real-window checks are manual validator runs."""

import unittest
from contextlib import redirect_stderr
from io import StringIO

from scenario_run import completion, parser


class ScenarioRunnerTests(unittest.TestCase):
    def test_incomplete_runs_and_unpreserved_settings_never_succeed(self):
        process = {"status": "completed", "exit_code": 0}
        lines = ["SCENARIO-RUN\tversion=1\texecutable=x", "OBSERVE\tsetup\tstep=0",
                 "PHASE\tactions", "ACTION\t0\tObserve", "OBSERVE\tterminal\tstep=1", "SCENARIO-OK"]
        trace = "\n".join(lines)
        self.assertTrue(completion(process, "SCENARIO-OK\n", trace, True))
        for index in range(len(lines)):
            partial = "\n".join(lines[:index] + lines[index + 1:])
            self.assertFalse(completion(process, "SCENARIO-OK", partial, True))
        for status in ("timeout", "interrupted", "launch-failed", "process-failed"):
            self.assertFalse(completion({"status": status, "exit_code": 0}, "SCENARIO-OK", trace, True))
        self.assertFalse(completion(process, "SCENARIO-OK", trace, False))
        self.assertFalse(completion(process, "SCENARIO-FAIL unmet\nSCENARIO-OK", trace, True))
        self.assertFalse(completion(process, "SCENARIO-OK", "SCENARIO-FAIL unmet\n" + trace, True))
        self.assertFalse(completion(process, "SCENARIO-OK\nanything", trace, True))
        self.assertFalse(completion({"status": "completed", "exit_code": 1}, "SCENARIO-OK", trace, True))

    def test_only_bounded_parameters_are_accepted(self):
        p = parser()
        values = p.parse_args(["--scenario", "a.scenario", "--out", "new", "--timeout", "3"])
        self.assertEqual(values.timeout, 3)
        for text in ("0", "-1", "nan", "inf"):
            with redirect_stderr(StringIO()), self.assertRaises(SystemExit):
                p.parse_args(["--scenario", "a", "--out", "b", "--timeout", text])


if __name__ == "__main__":
    unittest.main()
