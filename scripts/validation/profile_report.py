"""Audit raw route observations; never treat missing readiness as zero."""

import math
import csv
from itertools import groupby
from pathlib import Path
import statistics


def distribution(values):
    if not values:
        return {"count": 0, "mean": None, "p95": None, "worst": None}
    ordered = sorted(values)
    return {"count": len(values), "mean": statistics.fmean(values),
            "p95": ordered[math.ceil(0.95 * len(values)) - 1], "worst": ordered[-1]}


def summarize(folder):
    frames = []
    for number, line in enumerate((Path(folder) / "frames.txt").read_text().splitlines(), 1):
        parts = line.split()
        if len(parts) != 7:
            raise ValueError(f"frames.txt line {number}: expected seven columns")
        time_s, scenario, *raw = parts
        values = [float(time_s), *map(float, raw)]
        if not all(math.isfinite(v) and v >= 0 for v in values):
            raise ValueError(f"frames.txt line {number}: invalid measurement")
        if frames and values[0] < frames[-1][0]:
            raise ValueError(f"frames.txt line {number}: nonmonotonic timestamp")
        frames.append((values[0], scenario, *values[1:]))
    if not frames:
        raise ValueError("frames.txt has no observations")

    def report(rows):
        return {"interval_ms": distribution([r[2] for r in rows]),
                "cpu_work_ms": distribution([r[3] for r in rows]),
                "main_ms": distribution([r[4] for r in rows]),
                "render_ms": distribution([r[5] for r in rows]),
                "combined_wait_ms": distribution([r[6] for r in rows]),
                "over_100ms": sum(r[2] > 100 for r in rows),
                "interval_coverage_s": sum(r[2] for r in rows) / 1000}

    scored = [row for row in frames if row[1] != "warmup"]
    result = {"schema": 1, "all": report(scored),
              "scenarios": {name: report([r for r in frames if r[1] == name])
                            for name in dict.fromkeys(r[1] for r in frames)},
              "first_five_minutes": report([r for r in scored if r[0] < scored[0][0] + 300]) if scored else report([]),
              "final_five_minutes": report([r for r in scored if r[0] > scored[-1][0] - 300]) if scored else report([])}
    result["blocks"] = []
    for scenario, group in groupby(frames, key=lambda row: row[1]):
        rows = list(group)
        result["blocks"].append({"scenario": scenario, "first_timestamp_s": rows[0][0],
                                 "last_timestamp_s": rows[-1][0], **report(rows)})
    readiness = []
    for line in (Path(folder) / "route.txt").read_text().splitlines():
        if not line.startswith("ROUTE-LEG "):
            continue
        row = dict(word.split("=", 1) for word in line.split()[1:])
        for key in ("ring_ms", "ready_ms"):
            value = row.get(key)
            if value is None:
                raise ValueError(f"missing {key} readiness field")
            row[key] = None if value == "none" else float(value)
            if row[key] is not None and (not math.isfinite(row[key]) or row[key] < 0):
                raise ValueError("invalid readiness observation")
        readiness.append(row)
    result["readiness"] = readiness
    result["incomplete_ring_legs"] = sum(r["ring_ms"] is None for r in readiness)
    result["incomplete_detail_legs"] = sum(r["ready_ms"] is None for r in readiness)
    result["limitations"] = [
        "readiness none means not completed within that leg, not zero milliseconds",
        "CPU work is not GPU execution; main/render overlap and are not added",
        "wait combines GPU pacing/acquisition; unresolved present cost remains in render work",
        "raw legacy timings are rounded to 0.01 ms; absent FrameWork is flagged by telemetry",
    ]
    return result


def resources(path):
    """Audit required live observations, retaining transitions and growth."""
    with Path(path).open(newline="", encoding="utf-8") as stream:
        rows = list(csv.DictReader(stream))
    if not rows:
        raise ValueError("telemetry has no observations")
    required = ("meshes", "images", "standard_materials", "ground_materials",
                "terrain_bytes", "terrain_cap_bytes", "render_scale",
                "output_width", "output_height", "target_width", "target_height")
    columns = {*required, "elapsed_s", "schema", "frame_work_available", "leg",
               "ring_ready", "detail_ready"}
    if not columns.issubset(rows[0]):
        raise ValueError("telemetry is missing required columns")
    series = {key: [] for key in required}
    previous = -1.0
    gaps = []
    settled = {}
    for row in rows:
        elapsed = float(row["elapsed_s"])
        if not math.isfinite(elapsed) or elapsed < previous:
            raise ValueError("invalid telemetry timestamp")
        if previous >= 0 and elapsed - previous > 1.5:
            gaps.append({"from_s": previous, "to_s": elapsed})
        previous = elapsed
        if row["schema"] != "1" or row["frame_work_available"] != "true":
            raise ValueError("unsupported telemetry schema or missing frame-work observations")
        for key in required:
            # Initial camera target dimensions may be unavailable before setup.
            if row[key] == "none" and key.startswith("target_"):
                continue
            value = float(row[key])
            if not math.isfinite(value) or value < 0:
                raise ValueError(f"invalid telemetry field: {key}")
            series[key].append(value)
        if row["ring_ready"] == "true" and row["detail_ready"] == "true":
            settled[row["leg"]] = row
    if any(not values for values in series.values()):
        raise ValueError("required telemetry field has no valid observation")
    return {"schema": 1, "samples": len(rows), "sampling_gaps": gaps,
            "observations": {key: {"first": values[0], "last": values[-1],
                                   "peak": max(values), "net_change": values[-1] - values[0]}
                             for key, values in series.items()},
            "last_settled_sample_per_leg": list(settled.values()),
            "limitations": ["net growth includes warming caches and is not itself a leak verdict",
                            "main-world asset counts do not measure total GPU allocations"]}
