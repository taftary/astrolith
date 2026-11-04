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


# A frame "waited long" when its combined wait reaches half a 60 Hz vblank
# (#1085 T1): #1087 runs sit near 0.2 ms or near 33 ms, nothing in between.
WAIT_LONG_MS = 1000 / 60 / 2
# A run is one mode when at most this share of its frames is in the other.
WAIT_MODE_SHARE = 0.10


def wait_mode(waits):
    """Label a run's frame waits fast, slow or mixed (pure)."""
    if not waits:
        return {"label": None, "long_wait_share": None}
    share = sum(w >= WAIT_LONG_MS for w in waits) / len(waits)
    label = ("fast" if share <= WAIT_MODE_SHARE
             else "slow" if share >= 1 - WAIT_MODE_SHARE else "mixed")
    return {"label": label, "long_wait_share": share}


def gpu_passes(path):
    """Read gpu.txt: per-scenario mean per GPU pass, or unsupported."""
    lines = [line for line in Path(path).read_text().splitlines() if line.strip()]
    if lines and lines[0].startswith("GPU-UNSUPPORTED"):
        return {"supported": False, "reason": lines[0].split("reason=", 1)[-1],
                "scenarios": {}}
    scenarios = {}
    for number, line in enumerate(lines, 1):
        head, sep, pass_path = line.partition(" path=")
        words = head.split()
        if not sep or not words or words[0] != "GPU-PASS":
            raise ValueError(f"gpu.txt line {number}: expected GPU-PASS ... path=...")
        row = dict(word.split("=", 1) for word in words[1:])
        mean = float(row["mean"])
        if not math.isfinite(mean) or mean < 0:
            raise ValueError(f"gpu.txt line {number}: invalid mean")
        scenarios.setdefault(row["scenario"], {})[pass_path] = {
            "mean": mean, "samples": int(row["samples"])}
    return {"supported": True, "scenarios": scenarios}


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
                "wait_mode": wait_mode([r[6] for r in rows]),
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
    gpu = Path(folder) / "gpu.txt"
    result["gpu_passes"] = gpu_passes(gpu) if gpu.exists() else None
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


def mad(values):
    """Median absolute deviation (pure)."""
    centre = statistics.median(values)
    return statistics.median(abs(v - centre) for v in values)


def improvement(baseline, prototype):
    """Spec v1 improvement test for one lower-is-better metric (pure).

    Per-run values of the same metric. Better needs both: a median at least
    5% lower, and a drop above twice the baseline run-to-run MAD. Exactly
    one of the two holding is ambiguous.
    """
    base, proto = statistics.median(baseline), statistics.median(prototype)
    drop = base - proto
    by_share = base > 0 and drop >= 0.05 * base
    by_noise = drop > 2 * mad(baseline)
    verdict = ("better" if by_share and by_noise
               else "ambiguous" if by_share != by_noise else "not-better")
    return {"baseline_median": base, "prototype_median": proto, "drop": drop,
            "five_percent": by_share, "beyond_noise": by_noise, "verdict": verdict}


def within(baseline, prototype, share, floor):
    """Prototype median at most max(share x baseline, floor) above baseline (pure)."""
    base, proto = statistics.median(baseline), statistics.median(prototype)
    return proto - base <= max(share * base, floor)


def classify(scenarios, pairs):
    """Spec v1 verdict over matched scenarios (pure).

    `scenarios` maps a scenario name to per-run lists, each a
    `(baseline, prototype)` pair of lists: `p95` and `mean` frame interval
    (ms), optional `bottleneck` execution time (ms), `readiness_p95` (ms)
    and `memory_peak` (bytes). `pairs` is the alternating pairs run so far.
    Visual and behaviour checks are judged by people and block rollout
    separately; this reads numbers only.
    """
    tests = {name: {key: improvement(*m[key]) for key in ("p95", "bottleneck") if key in m}
             for name, m in scenarios.items()}
    verdicts = [test["verdict"] for per in tests.values() for test in per.values()]
    if "better" in verdicts:
        verdict = "promising"
    elif "ambiguous" in verdicts:
        verdict = "extend-to-five-pairs" if pairs < 5 else "inconclusive"
    else:
        verdict = "rejected"
    rollout = {name: {"mean": within(*m["mean"], 0.05, 0.5),
                      "p95": within(*m["p95"], 0.05, 0.5),
                      "readiness_p95": within(*m["readiness_p95"], 0.10, 100.0),
                      "memory_peak": within(*m["memory_peak"], 0.05, 0.0)}
               for name, m in scenarios.items()}
    return {"verdict": verdict, "tests": tests, "rollout_numbers": rollout,
            "rollout_numbers_ok": verdict == "promising" and all(
                all(r.values()) for r in rollout.values()),
            "limitations": ["visual or behaviour regressions block rollout regardless of numbers",
                            "an execution-time drop alone is not a higher displayed frame rate",
                            "runs in different wait modes are not a matched pair"]}
