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
        self.assertFalse(values.loading)
        self.assertTrue(p.parse_args(["--scenario", "a", "--out", "b", "--loading"]).loading)
        for text in ("0", "-1", "nan", "inf"):
            with redirect_stderr(StringIO()), self.assertRaises(SystemExit):
                p.parse_args(["--scenario", "a", "--out", "b", "--timeout", text])

    def test_persistent_success_requires_public_confirmation_and_exit_witness(self):
        process = {"status": "completed", "exit_code": 0}
        lines = ["SCENARIO-RUN\tversion=1\texecutable=x\tsettings=production-disposable-profile",
                 "PERSISTENCE\tstart=Restart\tsettings=x",
                 "PHASE\tsetup\tseed=42\tfixture=Loaded\tfixture_version=2\tcounts_as_travel=false",
                 "OBSERVE\tsetup\tstep=0", "PHASE\tactions", "ACTION\t0\tQuit",
                 "QUIT-CONTROL\tmenu-Quit\tinput_phase=0\tstep=0",
                 "QUIT-CONTROL\tmenu-Quit\tinput_phase=1\tstep=0",
                 "QUIT-CONTROL\tmenu-Quit\tinput_phase=2\tstep=0",
                 "QUIT-CONTROL\tquit-Quit\tinput_phase=0\tstep=0",
                 "QUIT-CONTROL\tquit-Quit\tinput_phase=1\tstep=0", "OBSERVE\tterminal\tstep=0",
                 "NORMAL-QUIT\twitnessed=true\tsource=production-confirmation-input-and-exit", "SCENARIO-OK"]
        self.assertTrue(completion(process, "SCENARIO-OK", "\n".join(lines), True, True))
        for index in range(len(lines)):
            self.assertFalse(completion(process, "SCENARIO-OK", "\n".join(lines[:index] + lines[index + 1:]), True, True))
        for a, b in ((2, 5), (3, 5), (6, 10), (11, 12)):
            reordered = lines.copy()
            reordered[a], reordered[b] = reordered[b], reordered[a]
            self.assertFalse(completion(process, "SCENARIO-OK", "\n".join(reordered), True, True))
        repeated = lines[:5] + ["PREPARE\t0\tPrepare(x)"] + lines[5:]
        self.assertFalse(completion(process, "SCENARIO-OK", "\n".join(repeated), True, True))

    def test_persistent_marker_presence_cannot_hide_actions_before_loaded_state(self):
        lines = ["SCENARIO-RUN\tversion=1\texecutable=x\tsettings=production-disposable-profile",
                 "PERSISTENCE\tstart=Restart\tsettings=x", "ACTION\t0\tQuit",
                 "OBSERVE\tsetup\tstep=0", "PHASE\tactions",
                 "QUIT-CONTROL\tmenu-Quit\tinput_phase=1\tstep=0",
                 "QUIT-CONTROL\tquit-Quit\tinput_phase=1\tstep=0", "OBSERVE\tterminal\tstep=0",
                 "NORMAL-QUIT\twitnessed=true\tsource=production-confirmation-input-and-exit", "SCENARIO-OK"]
        self.assertFalse(completion({"status": "completed", "exit_code": 0},
                                    "SCENARIO-OK", "\n".join(lines), True, True))


if __name__ == "__main__":
    unittest.main()
