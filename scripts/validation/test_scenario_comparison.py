"""Hermetic illustrative parser/math fixtures, never real-run overhead evidence."""

import copy
import ctypes
import json
import os
from pathlib import Path
import tempfile
import unittest

from native_conditions import Guid, PowerStatus, unavailable
from native_display import DisplayDevice, DisplayMode, MonitorInfo
from native_refresh import DisplayPath, ModeInfo, SourceName
from scenario_comparison import adapter, native, overhead
from scenario_timing import CONDITION_KEYS, Timing, timing_summary
from test_scenario_measure import HEADER, WALL, emit, measure_rows, trace_text


def timing_text():
    rows = [row for row in measure_rows() if row["kind"] in ("frame-begin", "clock-sync")]
    rows.extend([{"version": 1, "elapsed_ns": 160_000_000, "observed_main_frame": 1,
                  "kind": "comparison-state", "snapshot_frame": 1,
                  **{key: "illustrative" for key in CONDITION_KEYS}},
                 {"version": 1, "elapsed_ns": 301_000_000, "observed_main_frame": 3, "kind": "clock-sync",
                  "before_ns": 301_000_000, "after_ns": 301_000_000, "unix_ns": WALL + 301_000_000}])
    rows.sort(key=lambda row: row["elapsed_ns"])
    for index, row in enumerate(rows):
        row["sequence"] = index
    return "\n".join([HEADER, "COLLECTION\tversion=1\tmode=intervals-only"]
                     + [emit("MEASURE-EVENT", row) for row in rows]
                     + [f"MEASURE-END\tversion=1\tcomplete=true\tmain_frame=4\trecords={len(rows)}\telapsed_ns=460000000\tshutdown_idle_or_gpu_release_claim=false"]) + "\n"


def illustrative_runs():
    runs = []
    for index in range(6):
        runs.append({"directory": f"illustrative-only-{index}", "collection": "off" if index % 2 == 0 else "on",
                     "identity": {"source": "illustrative", "condition": "same"},
                     "started_utc": f"2026-11-07T00:{index * 3:02d}:00+00:00",
                     "completed_utc": f"2026-11-07T00:{index * 3 + 2:02d}:59+00:00",
                     "scored": {"frame_interval_ms": {"mean": 20.0 if index % 2 == 0 else 21.0,
                                                        "p95": 30.0 if index % 2 == 0 else 32.0}}})
    return runs


class TimingTests(unittest.TestCase):
    def test_common_intervals_have_no_full_collector_and_recompute_without_cache(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "app").mkdir()
            (root / "app/scenario.tsv").write_bytes(trace_text().encode())
            path = root / "app/input-measure-v1.tsv"
            path.write_bytes(timing_text().encode())
            summary = timing_summary(root)
            self.assertEqual(summary["collection"], "intervals-only")
            self.assertEqual(summary["scored"]["frame_interval_ms"]["count"], 1)
            self.assertEqual(summary["whole_session"]["frame_interval_ms"]["mean"], 100)
            with self.assertRaisesRegex(ValueError, "entire scoring"):
                timing_summary(root, 30, 120)
            for index in range(len(timing_text().splitlines())):
                rows = timing_text().splitlines(keepends=True)
                path.write_bytes("".join(rows[:index] + rows[index + 1:]).encode())
                with self.subTest(missing=index), self.assertRaises(ValueError):
                    Timing(path)

    def test_clock_discontinuities_limits_and_wrong_or_truncated_records_fail_closed(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "timing.tsv"
            original = timing_text()
            for changed in (original[:-1], original.replace("complete=true", "complete=false"),
                            original.replace("interval_ns=100000000", "interval_ns=99999999", 1),
                            original.replace("kind=comparison-state", "kind=main-work", 1),
                            original.replace("event_limit=8192", "event_limit=8193"),
                            original.replace("snapshot_frame=1", "snapshot_frame=99999"),
                            original + "MEASURE-LOSS\tversion=1\trequired_evidence_missing=true\n"):
                path.write_bytes(changed.encode())
                with self.assertRaises(ValueError):
                    Timing(path)
            path.write_bytes(original.replace(str(WALL + 301_000_000), str(WALL + 1_301_000_000)).encode())
            with self.assertRaisesRegex(ValueError, "clock discontinuity"):
                Timing(path).utc_offset()


class OverheadTests(unittest.TestCase):
    def test_medians_and_exact_larger_of_thresholds_are_used_without_outlier_removal(self):
        runs = illustrative_runs()
        result = overhead(runs)
        self.assertTrue(result["overhead_within_limits"])
        self.assertEqual(result["metrics"]["mean"]["limit_ms"], 1)
        runs[1]["scored"]["frame_interval_ms"]["mean"] = 1000
        self.assertTrue(overhead(runs)["overhead_within_limits"])
        runs[3]["scored"]["frame_interval_ms"]["mean"] = 21.01
        self.assertFalse(overhead(runs)["overhead_within_limits"])
        for run in runs:
            run["scored"]["frame_interval_ms"] = {"mean": 100, "p95": 100}
        result = overhead(runs)
        self.assertEqual(result["metrics"]["mean"]["limit_ms"], 5)
        self.assertEqual(result["metrics"]["p95"]["limit_ms"], 5)

    def test_source_condition_hash_or_serial_mismatch_cannot_claim_a_match(self):
        for change in (lambda r: r[1]["identity"].update(source="other"),
                       lambda r: r[1]["identity"].update(condition="unavailable"),
                       lambda r: r[1].update(started_utc=r[0]["started_utc"]),
                       lambda r: r[1].update(collection="off"),
                       lambda r: r[1].update(directory=r[0]["directory"])):
            runs = illustrative_runs()
            change(runs)
            with self.assertRaises(ValueError):
                overhead(runs)
        with self.assertRaises(ValueError):
            overhead(illustrative_runs()[:4])


class NativeConditionsTests(unittest.TestCase):
    @unittest.skipUnless(os.name == "nt", "native Windows ABI layouts")
    def test_windows_native_structure_sizes_and_guid_are_exact(self):
        for structure, size in ((Guid, 16), (PowerStatus, 12), (DisplayDevice, 840), (DisplayMode, 220), (MonitorInfo, 104),
                                (DisplayPath, 72), (ModeInfo, 64), (SourceName, 84)):
            self.assertEqual(ctypes.sizeof(structure), size)
        self.assertEqual(Guid().text(), "00000000-0000-0000-0000-000000000000")

    def test_unavailable_is_explicit_and_never_zero(self):
        def missing():
            raise OSError("illustrative unavailable observation")
        self.assertEqual(unavailable(missing), {"available": False, "reason": "illustrative unavailable observation"})

    def test_missing_changed_unequal_adapter_driver_or_sample_coverage_is_refused(self):
        actual = {"vendor": 0x8086, "device": 0x5917}
        display = {"available": True, "source": "illustrative-native-fixture", "refresh_hz": 60,
                   "refresh_rational": {"numerator": 60, "denominator": 1, "hz": 60, "source": "fixture"},
                   "client_pixels": [1600, 900], "desktop_pixels": [1920, 1080], "monitor_device": "fixture",
                   "adapter_device_id": r"PCI\VEN_8086&DEV_5917", "driver": {"available": True, "version": "1.2.3", "source": "fixture"}}
        power = {"available": True, "ac_line": 1, "battery_flags": 1, "battery_saver": 0,
                 "active_scheme": "fixture", "effective_overlay": "fixture", "source": "fixture"}
        rows = [{"schema": "native-comparison-conditions-v1", "pid": 42,
                 "utc_before_ns": WALL + i * 1_000_000_000, "utc_after_ns": WALL + i * 1_000_000_000 + 10,
                 "display": copy.deepcopy(display), "power": copy.deepcopy(power)} for i in range(4)]
        manifest = {"process": {"pid": 42, "condition_samples": 4}}
        timing = {"score_utc_ns": [WALL + 500_000_000, WALL + 2_500_000_000]}
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)

            def check(values, adapter_info=actual):
                (root / "conditions.jsonl").write_bytes(("\n".join(json.dumps(row) for row in values) + "\n").encode())
                return native(root, manifest, timing, adapter_info)

            self.assertEqual(check(rows)["whole_session_samples"], 4)
            for field, value in (("available", False), ("refresh_hz", 59), ("driver", {"available": False})):
                changed = copy.deepcopy(rows)
                changed[2]["display"][field] = value
                with self.subTest(field=field), self.assertRaises(ValueError):
                    check(changed)
            for changed in (rows[:2], rows[1:], rows + [rows[-1]]):
                with self.assertRaises(ValueError):
                    check(changed)
            with self.assertRaisesRegex(ValueError, "rendering adapter"):
                check(rows, {"vendor": 0x10DE, "device": 1})

    def test_actual_adapter_requires_a_single_complete_runtime_record(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "stderr.txt"
            record = 'AdapterInfo { name: "fixture", vendor: 32902, device: 22807, driver: "fixture", driver_info: "fixture", backend: Vulkan }\n'
            path.write_text(record, encoding="utf-8")
            self.assertEqual(adapter(path)["backend"], "Vulkan")
            for value in ("", record + record, record.replace("backend: Vulkan", "unavailable: true")):
                path.write_text(value, encoding="utf-8")
                with self.assertRaises(ValueError):
                    adapter(path)


if __name__ == "__main__":
    unittest.main()
