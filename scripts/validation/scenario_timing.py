"""Common off/on raw interval/clock/graphics audit. No cached-summary trust."""

from array import array
from bisect import bisect_right
import math
from pathlib import Path

from profile_report import distribution
from scenario_measure_trace import InputTrace, MAX_FRAMES, fields, flag, integer, lines

GRAPHICS = ("quality_mode target_fps auto_ceiling vsync resolution_3d planet_detail marker_detail "
            "atmosphere shadows bloom ground_memory_limit fullscreen exposure labels label_limit "
            "dimming night_light mode_reminder show_pull scale_hud eye_adaptation body_colour").split()
EFFECTIVE = "terrain markers atmosphere shadows bloom render_scale terrain_work".split()
CONDITION_KEYS = ["settings.saved." + key for key in GRAPHICS] + ["settings.effective." + key for key in EFFECTIVE] + ["output_pixels", "target_pixels"]


class Timing:
    def __init__(self, path):
        self.begin = array("Q", [0])
        self.states, self.syncs = [], []
        self.sequence = self.time = self.end_ns = 0
        self.collection = None
        header = end = False
        for text in lines(path):
            tag, row = fields(text)
            if end:
                raise ValueError("common timing has rows after its terminal boundary")
            if not header:
                if (tag != "MEASURE-RUN" or row.get("clock") != "shared-input-loading-monotonic"
                        or row.get("interval_scope") != "First-to-First" or integer(row, "event_limit") != 8192
                        or integer(row, "output_limit_bytes") != 268435456):
                    raise ValueError("common timing has no supported bounded header")
                header = True
            elif tag == "COLLECTION":
                if self.collection is not None or self.sequence or row.get("mode") not in ("full", "intervals-only"):
                    raise ValueError("common timing collection mode missing/duplicate/malformed")
                self.collection = row["mode"]
            elif tag == "MEASURE-EVENT":
                self.event(row)
            elif tag == "MEASURE-END":
                self.end_ns = integer(row, "elapsed_ns")
                if (not flag(row, "complete") or integer(row, "records") != self.sequence
                        or integer(row, "main_frame") != len(self.begin) - 1 or self.end_ns < self.time
                        or row.get("shutdown_idle_or_gpu_release_claim") != "false"):
                    raise ValueError("common timing has an incomplete/mismatched seal")
                end = True
            else:
                raise ValueError("unknown/duplicate/lost common timing record")
        if not end or len(self.begin) < 3 or not self.states or len(self.syncs) < 2 or self.collection is None:
            raise ValueError("common timing required interval/clock/graphics/terminal evidence missing")

    def event(self, row):
        sequence, stamp = integer(row, "sequence"), integer(row, "elapsed_ns")
        frame = integer(row, "observed_main_frame")
        if sequence != self.sequence or stamp < self.time or not 1 <= frame <= len(self.begin):
            raise ValueError("common timing sequence/time/frame missing or reordered")
        self.sequence += 1
        self.time = stamp
        kind = row.get("kind")
        if kind == "frame-begin":
            start = integer(row, "begin_ns")
            if (frame != len(self.begin) or frame > MAX_FRAMES or start > stamp
                    or (frame > 1 and start <= self.begin[-1])):
                raise ValueError("common timing frame begin missing/duplicate/excessive")
            if (row.get("interval_scope") != "First-to-First"
                    or (frame == 1 and row.get("interval_ns") != "unavailable-first-frame")
                    or (frame > 1 and integer(row, "interval_ns") != start - self.begin[-1])):
                raise ValueError("common interval differs from its retained begins")
            self.begin.append(start)
        elif kind == "comparison-state":
            snapshot = integer(row, "snapshot_frame")
            if not 1 <= snapshot <= frame or len(self.states) >= 8192 or any(key not in row for key in CONDITION_KEYS):
                raise ValueError("required common graphics state missing/excessive")
            self.states.append({"frame": frame, "elapsed_ns": stamp, "snapshot_frame": snapshot,
                                **{key: row[key] for key in CONDITION_KEYS}})
        elif kind == "clock-sync":
            before, after, utc = (integer(row, key) for key in ("before_ns", "after_ns", "unix_ns"))
            if not self.begin[1] <= before <= after <= stamp or len(self.syncs) >= 4096:
                raise ValueError("invalid/excessive common clock capture")
            self.syncs.append({"before_ns": before, "after_ns": after, "unix_ns": utc})
        elif kind in ("state", "main-work", "render-work", "cap-wait"):
            if self.collection != "full":
                raise ValueError("full collector record exists in collection-off timing")
        else:
            raise ValueError("unknown common timing event")

    def window(self, start, end):
        indices = [i for i in range(2, len(self.begin)) if self.begin[i - 1] >= start and self.begin[i] <= end]
        values = [(self.begin[i] - self.begin[i - 1]) / 1e6 for i in indices]
        return {"start_s": start / 1e9, "end_s": end / 1e9,
                "frame_interval_ms": distribution(values), "interval_coverage_s": math.fsum(values) / 1000,
                "stalls_over_100_ms": [{"ending_frame": i, "interval_ms": value}
                                       for i, value in zip(indices, values) if value > 100]}

    def utc_offset(self):
        # Each app UTC reading lies between its retained monotonic brackets.
        lower = max(row["unix_ns"] - row["after_ns"] for row in self.syncs)
        upper = min(row["unix_ns"] - row["before_ns"] for row in self.syncs)
        if lower - upper > 5_000_000:
            raise ValueError("common wall/monotonic clock discontinuity prevents conditions correlation")
        return (lower + upper) // 2, max(0, lower - upper)


def timing_summary(directory, warmup_s=0, scored_s=None):
    directory = Path(directory)
    timing = Timing(directory / "app/input-measure-v1.tsv")
    trace = InputTrace(directory / "app/scenario.tsv")
    if (not math.isfinite(warmup_s) or warmup_s < 0
            or (scored_s is not None and (not math.isfinite(scored_s) or scored_s <= 0))):
        raise ValueError("invalid common warm-up/scoring duration")
    if any(link["main_frame"] >= len(timing.begin) or not timing.begin[link["main_frame"]] <= link["elapsed_ns"] <= timing.end_ns for link in trace.links):
        raise ValueError("common input trace is not linked to its measured interval")
    origin = trace.phases[0]["actions_ns"]
    start = origin + round(warmup_s * 1e9)
    end = timing.begin[-1] if scored_s is None else start + round(scored_s * 1e9)
    if end > timing.begin[-1] or start >= end:
        raise ValueError("common input session does not cover the entire scoring window")
    times = [row["elapsed_ns"] for row in timing.states]
    first = bisect_right(times, start) - 1
    if first < 0:
        raise ValueError("common scoring lacks initial actual graphics state")
    states = timing.states[first:bisect_right(times, end)]
    scored = timing.window(start, end)
    if not scored["frame_interval_ms"]["count"]:
        raise ValueError("common scoring has no complete intervals")
    offset, drift = timing.utc_offset()
    return {"schema": "common-input-timing-summary-v1", "complete": True, "collection": timing.collection,
            "main_frames": len(timing.begin) - 1, "events": timing.sequence,
            "whole_session": timing.window(timing.begin[1], timing.end_ns),
            "warmup": timing.window(origin, start), "scored": scored,
            "scored_graphics_states": states, "all_graphics_changes": timing.states,
            "score_utc_ns": [start + offset, end + offset], "clock_bracket_drift_ns": drift,
            "limitations": ["common minimal interval/clock/graphics recorder remains on both sides",
                            "full work/state/loading/resources and one-second memory sampling are off only in intervals-only",
                            "First-to-First intervals are CPU-delivered intervals, not physical scan-out",
                            "boundary-crossing intervals excluded; raw stalls never removed",
                            "UTC correlation is bracketed with 5 ms discontinuity allowance, not exact simultaneity"]}
