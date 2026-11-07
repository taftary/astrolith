"""Hermetic illustrative protocol bytes, never fabricated application evidence."""

import json
from pathlib import Path
import tempfile
import unittest

from scenario_measure import InputTrace, read_frames, summarize, window
from scenario_measure_trace import STATE_FIELDS
from test_scenario_loading import fixture as loading_fixture, scene, render, text as loading_text


WALL = 1_800_000_000_000_000_000
HEADER = "MEASURE-RUN\tversion=1\tevent_limit=8192\toutput_limit_bytes=268435456\tclock=shared-input-loading-monotonic\tinterval_scope=First-to-First\tstate_source=actual-scenario-observation-not-requested-assertions\tprocess_clock_link=bracketed-wall-observations-with-drift-check\tshutdown_idle_or_gpu_release_claim=false"


def emit(tag, row):
    return tag + "\t" + "\t".join(f"{k}={v}" for k, v in row.items())


def trace_text():
    rows = ["SCENARIO-RUN\tversion=1\texecutable=illustrative-test-only\tsettings=defaults-no-file-io"]
    entries = [(110_000_000, 1, None, "PHASE\tsetup\tcounts_as_travel=false\tinclude_in_resource_peaks=true"),
               (210_000_000, 2, 1, "OBSERVE\tsetup\tstep=0"),
               (212_000_000, 2, 1, "PHASE\tactions"),
               (220_000_000, 2, 1, "ACTION\t0\tHold(0.1)"),
               (410_000_000, 4, 3, "OBSERVE\tterminal\tstep=1"),
               (411_000_000, 4, 3, "SCENARIO-OK")]
    for number, (stamp, frame, snapshot, row) in enumerate(entries):
        rows.append(emit("MEASURE-LINK", {"version": 1, "trace_id": number, "elapsed_ns": stamp, "main_frame": frame,
                                         "snapshot_frame": snapshot or "unavailable-before-first-observation", "kind": row.split("\t")[0]}))
        rows.append(row)
        if row.startswith("OBSERVE\t"):
            name = row.split("\t")[1]
            rows.extend(f"VALUE\t{name}\t{key}\t{value}" for key, value in (("seed", "42"), ("camera.path", "root"), ("ready", "true")))
    return "\n".join(rows) + "\n"


def state(frame, phase):
    row = {key: "unavailable-not-observed" for key in STATE_FIELDS}
    row.update({"seed": "42", "camera.path": "root", "player.path": "root", "player.active": "false",
                "player.on_ground": "false", "player.flying": "false", "ready": "true", "fixture.kind": "loaded",
                "settings.saved.planet_detail": "normal", "settings.saved.ground_memory_limit": "64"})
    return {"kind": "state", "step": 0, "phase": phase, "snapshot_frame": frame if frame < 4 else 3,
            "snapshot_available": "true", "output_width": 1600, "output_height": 900, "target_width": 1600,
            "target_height": 900, "ground_ring_ready": "unavailable-no-matching-production-surface",
            "ground_detail_ready": "unavailable-no-matching-production-surface", **row}


def measure_rows():
    rows = []
    for frame in range(1, 5):
        start = frame * 100_000_000
        rows.append((start, frame, {"kind": "frame-begin", "begin_ns": start, "interval_ns": 100_000_000 if frame > 1 else "unavailable-first-frame", "interval_scope": "First-to-First"}))
        if frame == 1:
            rows.append((start + 1_000_000, frame, {"kind": "clock-sync", "before_ns": start + 1_000_000, "after_ns": start + 1_000_000,
                                                  "unix_ns": WALL + start + 1_000_000, "scope": "bracketed-wall-observation-not-shared-process-monotonic"}))
        rows.append((start + (50_000_000 if frame < 4 else 20_000_000), frame, state(frame, ("setup-waiting", "actions", "actions", "completed")[frame - 1])))
        for offset, kind, work, wait in ((60_000_000, "main-work", 10_000_000, 0),
                                         (70_000_000, "cap-wait", 0, 20_000_000),
                                         (80_000_000, "render-work", 30_000_000, 5_000_000)):
            if frame == 4:
                offset -= 30_000_000
            rows.append((start + offset, frame, {"kind": kind, "measured_frame": frame, "work_ns": work, "wait_ns": wait,
                                               "gpu_execution": "unavailable-not-enabled"}))
    return [{"version": 1, "sequence": index, "elapsed_ns": stamp, "observed_main_frame": frame, **row}
            for index, (stamp, frame, row) in enumerate(rows)]


def measure_text(rows=None):
    rows = measure_rows() if rows is None else rows
    return "\n".join([HEADER] + [emit("MEASURE-EVENT", row) for row in rows] +
                     [f"MEASURE-END\tversion=1\tcomplete=true\tmain_frame=4\trecords={len(rows)}\telapsed_ns=460000000\tshutdown_idle_or_gpu_release_claim=false"]) + "\n"


def install(root):
    app = root / "app"
    app.mkdir()
    (app / "scenario.tsv").write_bytes(trace_text().encode())
    (app / "input-measure-v1.tsv").write_bytes(measure_text().encode())
    load = loading_fixture() + [scene(3), render(3, 0), scene(4), render(4, 0)]
    for index, row in enumerate(load):
        frame = int(row["observed_main_frame"])
        row.update(sequence=str(index), elapsed_ns=str(frame * 100_000_000 + 25_000_000 + index))
    value = loading_text(load).rsplit("LOAD-END", 1)[0] + f"LOAD-END\tversion=1\tcomplete=true\tmain_frame=4\trecords={len(load)}\telapsed_ns=455000000\tall_jobs_or_retirements_finished=not-claimed\n"
    (app / "loading-v1.tsv").write_bytes(value.encode())
    rows = [{"elapsed_s": 0.25, "query_begin_s": 0.251, "query_end_s": 0.252,
             "utc_before_ns": WALL + 251_000_000, "utc_after_ns": WALL + 252_000_000,
             "pid": 42, "working_set_bytes": 123456789, "clock_link": "bracketed-wall-observation-not-same-instant"}]
    (root / "memory.jsonl").write_bytes(("\n".join(json.dumps(r) for r in rows) + "\n").encode())
    (root / "process.json").write_text(json.dumps({"status": "completed", "exit_code": 0, "sampling_enabled": True,
                                                 "sample_period_s": 1, "pid": 42, "samples": 1, "sampling_gaps": [],
                                                 "observed_peak_working_set_bytes": 123456789, "elapsed_s": 0.5}), encoding="utf-8")


class MeasurementTests(unittest.TestCase):
    def test_raw_frames_resources_and_bracketed_memory_are_recomputed(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            install(root)
            report = summarize(root)
            self.assertTrue(report["complete"])
            self.assertEqual(report["whole_session"]["frame_interval_ms"]["count"], 3)
            self.assertEqual(report["whole_session"]["main_work_ms"]["mean"], 10)
            self.assertEqual(report["whole_session"]["render_work_ms"]["mean"], 30)
            self.assertNotIn("summed_work_ms", report["whole_session"])
            self.assertEqual(report["working_set"]["samples"][0]["correlation"]["first_frame"], 2)
            self.assertEqual(report["loading"]["high_water"]["tracked_resident_logical_gpu_bytes"], 8388608)
            self.assertEqual(report["readiness"]["distinct_selected_moons"], 0)
            self.assertFalse(report["readiness"]["preparation_is_travel"])

    def test_every_required_measurement_row_is_checked_not_just_the_seal(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            install(root)
            path = root / "app/input-measure-v1.tsv"
            rows = measure_text().splitlines(keepends=True)
            for index in range(len(rows)):
                path.write_bytes("".join(rows[:index] + rows[index + 1:]).encode())
                with self.subTest(missing=index), self.assertRaises(ValueError):
                    summarize(root)

    def test_reindexed_missing_state_work_and_render_are_still_refused(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            install(root)
            for kind in ("state", "main-work", "cap-wait", "render-work", "frame-begin", "clock-sync"):
                rows = measure_rows()
                index = next(i for i, r in enumerate(rows) if r["kind"] == kind)
                del rows[index]
                for n, row in enumerate(rows):
                    row["sequence"] = n
                (root / "app/input-measure-v1.tsv").write_bytes(measure_text(rows).encode())
                with self.subTest(kind=kind), self.assertRaises(ValueError):
                    summarize(root)

    def test_work_frame_and_intervals_cannot_be_latest_unrelated_samples(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            install(root)
            original = measure_text()
            for old, new in (("measured_frame=2", "measured_frame=1"), ("interval_ns=100000000", "interval_ns=99999999"),
                             ("gpu_execution=unavailable-not-enabled", "gpu_execution=0"),
                             ("complete=true", "complete=false"), ("snapshot_frame=2", "snapshot_frame=1"),
                             ("event_limit=8192", "event_limit=8191")):
                (root / "app/input-measure-v1.tsv").write_bytes(original.replace(old, new, 1).encode())
                with self.subTest(change=old), self.assertRaises(ValueError):
                    summarize(root)

    def test_wrong_source_identity_snapshot_or_missing_trace_link_is_refused(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            install(root)
            path = root / "app/scenario.tsv"
            original = path.read_text()
            for changed in (original.replace("VALUE\tsetup\tseed\t42", "VALUE\tsetup\tseed\t43"),
                            original.replace("counts_as_travel=false", "counts_as_travel=true"),
                            original.replace("include_in_resource_peaks=true", "include_in_resource_peaks=false"),
                            "\n".join(line for line in original.splitlines() if "trace_id=3\t" not in line) + "\n"):
                path.write_bytes(changed.encode())
                with self.assertRaises(ValueError):
                    summarize(root)

    def test_missing_clock_and_memory_peak_or_owned_pid_mismatch_is_refused(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            install(root)
            path = root / "memory.jsonl"
            original = path.read_text()
            for old, new in (("123456789", "123456788"), ('"pid": 42', '"pid": 43'),
                             ('"utc_before_ns"', '"missing"')):
                path.write_bytes(original.replace(old, new).encode())
                with self.assertRaises(ValueError):
                    summarize(root)

    def test_scoring_never_includes_boundary_crossing_intervals_or_invents_duration(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            install(root)
            frames = read_frames(root / "app/input-measure-v1.tsv", InputTrace(root / "app/scenario.tsv"))
            self.assertEqual(window(frames, 150_000_000, 300_000_000)["frame_interval_ms"]["count"], 1)
            with self.assertRaisesRegex(ValueError, "entire requested scoring"):
                summarize(root, 30, 120)

    def test_app_clock_discontinuity_blocks_apparently_valid_memory_alignment(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            install(root)
            rows = measure_rows()
            sync = next(row for row in rows if row["kind"] == "clock-sync").copy()
            sync.update(elapsed_ns=201_000_000, observed_main_frame=2, before_ns=201_000_000,
                        after_ns=201_000_000, unix_ns=WALL + 1_201_000_000)
            rows.append(sync)
            rows.sort(key=lambda row: row["elapsed_ns"])
            for index, row in enumerate(rows):
                row["sequence"] = index
            (root / "app/input-measure-v1.tsv").write_bytes(measure_text(rows).encode())
            with self.assertRaisesRegex(ValueError, "clock discontinuity"):
                summarize(root)

    def test_app_side_limits_truncation_duplicate_fields_and_noncanonical_lines_fail_closed(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            install(root)
            original = measure_text()
            for changed in (original[:-1], original.replace("\n", "\r\n"),
                            original.replace("work_ns=10000000", "work_ns=nan", 1),
                            original.replace("\twork_ns=10000000", "\twork_ns=10000000\twork_ns=10000000", 1),
                            original + "MEASURE-LOSS\tversion=1\trequired_evidence_missing=true\n",
                            original.replace("kind=state", "kind=unknown", 1)):
                (root / "app/input-measure-v1.tsv").write_bytes(changed.encode())
                with self.assertRaises(ValueError):
                    summarize(root)


if __name__ == "__main__":
    unittest.main()
