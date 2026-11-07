"""Strict retained real-input timing/state audit (#1238); never a moon verdict.

Read-only recomputation: python scripts/validation/scenario_measure.py --run RUN
Uses raw records, not cached summaries or expected assertions.
"""

from array import array
import argparse
from bisect import bisect_right
from collections import Counter
import json
import math
from pathlib import Path
import sys

from profile_report import distribution
from scenario_measure_trace import (InputTrace, MAX_BYTES, MAX_FRAMES, STATE_FIELDS,
                                    fields, flag, integer, lines)


STATE_META = "step phase snapshot_frame snapshot_available output_width output_height target_width target_height ground_ring_ready ground_detail_ready".split()
PHASES = {"before-setup", "setup-waiting", "preparation-released", "preparation-waiting", "actions", "completed", "failed"}


class Frames:
    """Fixed-bounded scalar arrays; no retained per-frame snapshots/asset handles."""

    def __init__(self, trace):
        self.trace = trace
        self.sequence = self.time = self.rendered = self.end_ns = 0
        self.begin = array("Q", [0])
        self.state_ns = array("Q", [0])
        self.scene = array("I", [0])
        self.stage = array("I", [0])
        self.seen = bytearray([0])
        self.values = {kind: array("Q", [0]) for kind in ("main-work", "render-work", "cap-wait", "render-wait")}
        self.scenes, self.scene_ids = [], {}
        self.counts, self.support = Counter(), Counter()
        self.quality_changes = []
        self.syncs = []
        self.checked_observations = 0
        self.ready_phases = {}
        self.last_quality = None
        self.phase_starts = [phase["start_ns"] for phase in trace.phases]

    def event(self, row):
        sequence, stamp = integer(row, "sequence"), integer(row, "elapsed_ns")
        frame, kind = integer(row, "observed_main_frame"), row.get("kind")
        if sequence != self.sequence or stamp < self.time or not 1 <= frame <= len(self.begin):
            raise ValueError("missing/reordered measured sequence, time or frame")
        self.sequence += 1
        self.time = stamp
        if kind == "frame-begin":
            start = integer(row, "begin_ns")
            if frame != len(self.begin) or frame > MAX_FRAMES or start > stamp or (frame > 1 and start <= self.begin[-1]):
                raise ValueError("missing/duplicate/excessive input frame begin")
            interval = row.get("interval_ns")
            if row.get("interval_scope") != "First-to-First" or (frame == 1 and interval != "unavailable-first-frame") or (frame > 1 and integer(row, "interval_ns") != start - self.begin[-1]):
                raise ValueError("frame interval differs from retained monotonic begins")
            self.begin.append(start)
            self.state_ns.append(0)
            self.scene.append(0)
            self.stage.append(0)
            self.seen.append(0)
            for values in self.values.values():
                values.append(0)
        elif kind == "state":
            self.state(row, frame, stamp)
        elif kind in ("main-work", "render-work", "cap-wait"):
            measured = integer(row, "measured_frame")
            if not 1 <= measured < len(self.begin) or measured > frame:
                raise ValueError("work has no actual begun/extracted frame")
            bit = {"main-work": 2, "render-work": 4, "cap-wait": 8}[kind]
            if self.seen[measured] & bit:
                raise ValueError("duplicate work record for the measured frame")
            work, wait = integer(row, "work_ns"), integer(row, "wait_ns")
            if row.get("gpu_execution") != "unavailable-not-enabled" or (kind == "main-work" and wait) or (kind == "cap-wait" and work):
                raise ValueError("CPU work/cap/GPU measurement scope changed")
            if kind == "render-work":
                if measured != self.rendered + 1:
                    raise ValueError("missing/reordered extracted render-work frames")
                self.rendered = measured
                self.values["render-wait"][measured] = wait
            self.values[kind][measured] = wait if kind == "cap-wait" else work
            self.seen[measured] |= bit
        elif kind == "clock-sync":
            before, after = integer(row, "before_ns"), integer(row, "after_ns")
            if len(self.begin) < 2 or not self.begin[1] <= before <= after <= stamp or len(self.syncs) >= 4096 or row.get("scope") != "bracketed-wall-observation-not-shared-process-monotonic":
                raise ValueError("invalid/excessive process-memory clock synchronization")
            self.syncs.append({"before_ns": before, "after_ns": after, "unix_ns": integer(row, "unix_ns")})
        else:
            raise ValueError(f"unknown input measurement event: {kind}")

    def state(self, row, frame, stamp):
        if frame >= len(self.begin) or self.seen[frame] & 1 or any(key not in row for key in STATE_META + STATE_FIELDS):
            raise ValueError("missing/duplicate/incomplete actual frame state")
        phase = row["phase"]
        if phase not in PHASES:
            raise ValueError("unknown measured action/preparation phase")
        available = flag(row, "snapshot_available")
        if available:
            snapshot = integer(row, "snapshot_frame")
            if snapshot != frame and not (snapshot == frame - 1 and phase in ("completed", "failed")):
                raise ValueError("state reused an unrelated/stale physical snapshot")
            for key in ("ready", "player.active", "player.on_ground", "player.flying"):
                flag(row, key)
        elif phase != "before-setup":
            raise ValueError("required frame snapshot unavailable after setup")
        self.state_ns[frame] = stamp
        self.seen[frame] |= 1
        phase_id = max(0, bisect_right(self.phase_starts, stamp) - 1)
        self.stage[frame] = phase_id
        self.counts[phase] += 1
        self.support[(row["support.kind"], row["support.contact"], row["player.flying"], row["player.on_ground"])] += 1
        scene = tuple(row[key] for key in ("seed", "camera.path", "fixture.selected.planet", "fixture.selected.system", "fixture.selected.moon", "support.identity"))
        if scene not in self.scene_ids:
            if len(self.scenes) >= 8192:
                raise ValueError("too many distinct scene/support identities")
            self.scene_ids[scene] = len(self.scenes)
            self.scenes.append(scene)
        self.scene[frame] = self.scene_ids[scene]
        quality_keys = [key for key in STATE_FIELDS if key.startswith("settings.")] + ["output_width", "output_height", "target_width", "target_height"]
        quality = tuple(row[key] for key in quality_keys)
        if quality != self.last_quality:
            if len(self.quality_changes) >= 8192:
                raise ValueError("too many actual quality/resolution changes")
            self.quality_changes.append({"frame": frame, "elapsed_ns": stamp, **dict(zip(quality_keys, quality))})
            self.last_quality = quality
        for observation in self.trace.by_snapshot.get(frame, []):
            if any(row[key] != value for key, value in observation["values"].items()):
                raise ValueError("trace observation differs from its actual physical frame state")
            self.checked_observations += 1
        setup = self.trace.phases[phase_id]
        origin = setup.get("placement_ns", setup["start_ns"])
        if available and row["ready"] == "true" and origin <= stamp <= setup["actions_ns"] and phase_id not in self.ready_phases:
            self.ready_phases[phase_id] = {"frame": frame, "elapsed_ns": stamp, "scene": self.scene[frame],
                                         "kind": row["fixture.kind"], "on_ground": row["player.on_ground"],
                                         "flying": row["player.flying"], "support": row["support.identity"],
                                         "moon_type": row["fixture.moon.type"], "radius_km": row["fixture.moon.radius_km"],
                                         "ring_ready": row["ground_ring_ready"], "detail_ready": row["ground_detail_ready"]}

    def finish(self, row):
        total = len(self.begin) - 1
        self.end_ns = integer(row, "elapsed_ns")
        if not flag(row, "complete") or integer(row, "records") != self.sequence or integer(row, "main_frame") != total or self.end_ns < self.time or row.get("shutdown_idle_or_gpu_release_claim") != "false":
            raise ValueError("incomplete input measurement or mismatched seal")
        if not total or any(bits & 11 != 11 for bits in self.seen[1:]) or not 0 <= total - self.rendered <= 2 or not self.rendered:
            raise ValueError("required per-frame state/main/cap/render evidence missing")
        if any(not self.seen[frame] & 4 for frame in range(1, self.rendered + 1)):
            raise ValueError("required render work missing within retained frontier")
        if len(self.ready_phases) != len(self.trace.phases):
            raise ValueError("missing fresh readiness observation for setup/preparation")


def read_frames(path, trace):
    result, header, end = Frames(trace), None, None
    for text in lines(path):
        tag, row = fields(text)
        if end is not None:
            raise ValueError("input measurement records follow terminal boundary")
        if header is None:
            if tag != "MEASURE-RUN" or row.get("clock") != "shared-input-loading-monotonic" or row.get("state_source") != "actual-scenario-observation-not-requested-assertions" or row.get("shutdown_idle_or_gpu_release_claim") != "false" or row.get("process_clock_link") != "bracketed-wall-observations-with-drift-check" or integer(row, "event_limit") != 8192 or integer(row, "output_limit_bytes") != MAX_BYTES:
                raise ValueError("missing/incomplete input measurement header/scope/bounds")
            header = row
        elif tag == "MEASURE-EVENT":
            result.event(row)
        elif tag == "MEASURE-END":
            end = row
        else:
            raise ValueError("input measurement loss, duplicate header or unknown tag")
    if end is None:
        raise ValueError("input measurement sidecar has no terminal boundary")
    result.finish(end)
    return result


def window(frames, start, end):
    """An interval counts only if both First boundaries lie inside the window."""
    indices = [i for i in range(2, len(frames.begin)) if frames.begin[i - 1] >= start and frames.begin[i] <= end]
    intervals = [(frames.begin[i] - frames.begin[i - 1]) / 1e6 for i in indices]
    # First-to-First i-1 -> i is associated with work from frame i-1, not i.
    work_frames = [i - 1 for i in indices]
    stats = {key.replace("-", "_") + "_ms": distribution([values[i] / 1e6 for i in work_frames if key not in ("render-work", "render-wait") or frames.seen[i] & 4]) for key, values in frames.values.items()}
    return {"start_s": start / 1e9, "end_s": end / 1e9, "frame_interval_ms": distribution(intervals),
            "interval_coverage_s": math.fsum(intervals) / 1000,
            "stalls_over_100_ms": sum(value > 100 for value in intervals),
            "render_work_missing_exit_tail": sum(not frames.seen[i] & 4 for i in work_frames), **stats}


def summarize(directory, warmup_s=0, scored_s=None):
    from scenario_measure_report import loading_correlation, memory_correlation, readiness
    directory = Path(directory)
    trace = InputTrace(directory / "app" / "scenario.tsv")
    frames = read_frames(directory / "app" / "input-measure-v1.tsv", trace)
    linked = trace.validate(frames)
    if not math.isfinite(warmup_s) or warmup_s < 0 or (scored_s is not None and (not math.isfinite(scored_s) or scored_s <= 0)):
        raise ValueError("invalid measurement warm-up/scoring duration")
    origin = trace.phases[0]["actions_ns"]
    score_start = origin + round(warmup_s * 1e9)
    score_end = frames.end_ns if scored_s is None else score_start + round(scored_s * 1e9)
    if score_end > frames.begin[-1] and scored_s is not None:
        raise ValueError("input session does not cover the entire requested scoring window")
    scored = window(frames, score_start, score_end)
    if not scored["frame_interval_ms"]["count"]:
        raise ValueError("input session contains no complete scored frame intervals")
    memory = memory_correlation(directory, frames)
    report = {"schema": "input-measure-summary-v1", "complete": True, "events": frames.sequence,
              "main_frames": len(frames.begin) - 1, "rendered_main_frames": frames.rendered,
              "unrendered_exit_tail_frames": len(frames.begin) - 1 - frames.rendered,
              "clock_start_to_first_frame_s": frames.begin[1] / 1e9,
              "input": linked, "whole_session": window(frames, frames.begin[1], frames.end_ns),
              "warmup": window(frames, origin, score_start), "scored": scored,
              "readiness": readiness(trace, frames), "working_set": memory,
              "loading": loading_correlation(directory / "app" / "loading-v1.tsv", frames),
              "phase_frame_counts": dict(frames.counts),
              "actual_support_states": [{"kind": key[0], "contact": key[1], "flying": key[2], "on_ground": key[3], "frames": count} for key, count in frames.support.items()],
              "scene_support_identities": [dict(zip(("seed", "camera_path", "selected_planet", "selected_system", "selected_moon", "support"), scene)) for scene in frames.scenes],
              "actual_quality_resolution_changes": frames.quality_changes,
              "phase_timing_including_preparation": [{"name": phase["name"], **window(frames, phase["start_ns"],
                   trace.phases[index + 1]["start_ns"] if index + 1 < len(trace.phases) else frames.end_ns)}
                   for index, phase in enumerate(trace.phases)],
              "limitations": ["First-to-First CPU intervals are not physical scan-out timing",
                              "main and render CPU work retain separate extracted-frame identities; never sum threads",
                              "first interval unavailable; incomplete last interval omitted and render exit tail disclosed",
                              "readiness is fixture readiness, not moon contact or rendered-pixel completion",
                              "preparation is included in whole-session peaks, never flight/landing/travel",
                              "process memory is one-second sampled; wall-clock brackets are not simultaneous exact frame samples",
                              "GPU execution/actual draw calls/physical GPU release unavailable, not zero",
                              "observations do not certify future #1215 gameplay or performance"]}
    mean, p95 = scored["frame_interval_ms"]["mean"], scored["frame_interval_ms"]["p95"]
    report["diagnostic_baseline_limits_not_feature_verdict"] = {
        "mean_over_45_ms": mean > 45, "p95_over_65_ms": p95 > 65,
        "sampled_working_set_at_or_over_1_25_gib": memory["observed_peak_working_set_bytes"] >= 1.25 * 1024 ** 3,
        "fixture_readiness_over_3_s": any(visit["from_phase_start_s"] > 3 for visit in report["readiness"]["visits"])}
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", type=Path, required=True)
    args = parser.parse_args()
    try:
        manifest = json.loads((args.run / "manifest.json").read_text(encoding="utf-8"))
        config = manifest.get("measurement_windows", {})
        print(json.dumps(summarize(args.run, config.get("warmup_s", 0), config.get("scored_s")), indent=2, allow_nan=False))
        return 0
    except (OSError, ValueError, KeyError, TypeError) as exc:
        print(json.dumps({"complete": False, "reason": str(exc)}))
        return 1


if __name__ == "__main__":
    sys.exit(main())
