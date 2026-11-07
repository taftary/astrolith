"""Hermetic illustrative suite/audit fixtures, never real application evidence."""

import json
import os
from pathlib import Path
import tempfile
import unittest

from scenario_audit import Audit, drop_line, missing_reindexed_state
from scenario_catalog import parse_catalog, selections
from scenario_measure import read_frames, summarize
from scenario_measure_trace import InputTrace
from scenario_suite_support import diagnostics, scenario_result
from test_scenario_measure import install, measure_text, trace_text


class SuiteTests(unittest.TestCase):
    def test_expected_failures_require_the_real_reason_and_preserved_settings(self):
        good = {"complete": True, "settings_unchanged": True}
        failed = {"complete": False, "settings_unchanged": True}
        self.assertTrue(scenario_result(0, good, "SCENARIO-OK", None))
        self.assertTrue(scenario_result(1, failed, "missing or invisible button x", "missing or invisible button"))
        self.assertTrue(scenario_result(1, None, "file already exists", "file already exists"))
        for code, manifest, text, reason in ((0, failed, "missing x", "missing"), (1, failed, "wrong reason", "missing"),
                                              (1, good, "missing", "missing"), (0, None, "SCENARIO-OK", None),
                                              (1, {"complete": False, "settings_unchanged": False}, "missing", "missing")):
            self.assertFalse(scenario_result(code, manifest, text, reason))

    def test_all_runtime_diagnostics_are_retained_and_only_named_owner_vuids_are_separate(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "illustrative.log"
            path.write_text("INFO ready\nERROR VUID-vkAcquireNextImageKHR-semaphore-01286\nWARN real issue\npanic real issue\n", encoding="utf-8")
            report = diagnostics([path])
            self.assertEqual(len(report["owner_accepted"]), 1)
            self.assertEqual(len(report["unexpected"]), 2)
            self.assertEqual(report["unexpected"][0]["line"], 3)


class CatalogTests(unittest.TestCase):
    def test_selected_fixture_parameters_are_explicit_and_never_expected_properties(self):
        rows = selections("scenario-v2\nsetup moon 1.2.3.4.5.6.7.8.9 1 1 0 0 10 123 normal 20 -15\n"
                          "expect fixture.moon.radius_km range 200 201\n"
                          "prepare pole moon 1.2.3.4.5.6.7.8.9 2 0 1 0 10 456 free 0 0\n")
        self.assertEqual(len(rows), 2)
        self.assertEqual(rows[0]["game_s"], "123")
        self.assertEqual(rows[1]["direction"], ["0", "1", "0"])
        self.assertNotIn("radius_km", rows[0])
        self.assertNotIn("type", rows[0])
        with self.assertRaises(ValueError):
            selections("setup moon 1.2.3 1 0 0 1")

    def test_boundary_and_actual_host_records_cannot_be_conflated(self):
        host = "1.2.3.4.5.6.7.8.9"
        text = ("MOON-BOUNDARY\torigin=boundary-only\tradius_min_km=200\tradius_end_km=1700\tmin_inclusive=true\tend_inclusive=false\tgenerated_identity=none\n"
                f"MOON\tseed=42\tplanet={host}\tsystem=1.2.3.4.5.6.7\tmoon=1\tradius_km=200.5\torbit_km=150000\tperiod_days=4\tinstance_origin=procedural\tproperty_source=illustrative\tcontact=unavailable:not-a-contact-observation\n"
                "MOONS-OK count=1 planet_limit=1 moon_limit=64000 scope=bounded-not-global-minimum\n")
        boundary, rows = parse_catalog(text, host)
        self.assertEqual(len(rows), 1)
        self.assertEqual(boundary["generated_identity"], "none")
        for changed in (text.replace("generated_identity=none", "generated_identity=1"),
                        text.replace("radius_km=200.5", "radius_km=199.9"),
                        text.replace("seed=42", "seed=43"), text.replace("orbit_km=150000", "orbit_km=nan"),
                        text.replace("MOONS-OK count=1", "MOONS-OK count=2"), text.rsplit("MOONS-OK", 1)[0]):
            with self.assertRaises(ValueError):
                parse_catalog(changed, host)


class AuditTests(unittest.TestCase):
    def test_missing_record_mutations_must_change_bytes_and_reindexed_state_still_fails(self):
        original = measure_text()
        with self.assertRaises(ValueError):
            drop_line(original, "no-such-record")
        changed = missing_reindexed_state(original)
        self.assertNotEqual(changed, original)
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "trace.tsv").write_bytes(trace_text().encode())
            (root / "changed.tsv").write_bytes(changed.encode())
            with self.assertRaises(ValueError):
                read_frames(root / "changed.tsv", InputTrace(root / "trace.tsv"))

    @unittest.skipUnless(os.name == "nt", "Windows real-profile exclusion boundary")
    def test_nine_corrupted_copy_audits_preserve_the_illustrative_originals(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            source = root / "illustrative-not-app-evidence"
            source.mkdir()
            install(source)
            report = summarize(source)
            (source / "manifest.json").write_text(json.dumps({"measurement_windows": {"warmup_s": 0, "scored_s": None}}), encoding="utf-8")
            (source / "input-report.json").write_text(json.dumps(report), encoding="utf-8")
            audit = Audit(root / "audit", [source])
            audit.measurements()
            audit.save()
            self.assertEqual(len(audit.report["negative_copies"]), 9)
            self.assertTrue(audit.report["recomputed_identically"])
            self.assertTrue(audit.report["original_evidence_unchanged"])
            with self.assertRaises(ValueError):
                Audit(source / "unsafe-output", [source])
            with self.assertRaises(FileExistsError):
                Audit(root / "audit", [source])


if __name__ == "__main__":
    unittest.main()
