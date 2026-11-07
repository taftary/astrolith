"""Recompute three serial matched 30/120-second collection-off/on pairs.

python scripts/validation/scenario_comparison.py --pair OFF1 ON1 --pair OFF2 ON2 --pair OFF3 ON3
Read-only: no app launch, build, cached-statistic trust or arbitrary commands.
"""

import argparse
from datetime import datetime
import json
from pathlib import Path
import re
import statistics
import sys

from build_receipt import verify_record
from profile_run import digest
from scenario_measure import summarize as measure_summary
from scenario_measure_trace import lines
from scenario_timing import CONDITION_KEYS, timing_summary


def adapter(path):
    if not path.is_file() or path.stat().st_size > 16 * 1024 * 1024:
        raise ValueError("actual app adapter evidence missing/excessive")
    records = re.findall(r'AdapterInfo \{ (.*?) \}', path.read_text(encoding="utf-8", errors="strict"))
    if len(records) != 1:
        raise ValueError("actual app adapter/backend is missing or ambiguous")
    result = {}
    for key in ("name", "driver", "driver_info"):
        match = re.search(r'\b' + key + r': ("(?:[^"\\]|\\.)*")', records[0])
        if not match:
            raise ValueError(f"actual adapter {key} is unavailable")
        result[key] = json.loads(match[1])
    for key in ("vendor", "device"):
        match = re.search(r'\b' + key + r': ([0-9]+)', records[0])
        if not match:
            raise ValueError(f"actual adapter {key} is unavailable")
        result[key] = int(match[1])
    match = re.search(r'\bbackend: ([A-Za-z0-9]+)', records[0])
    if not match:
        raise ValueError("actual adapter backend unavailable")
    result["backend"] = match[1]
    return result


def native(directory, manifest, timing, actual_adapter):
    rows = []
    previous = None
    for text in lines(directory / "conditions.jsonl", limit=16_384):
        row = json.loads(text)
        if (len(rows) >= 4096 or row.get("schema") != "native-comparison-conditions-v1"
                or row.get("pid") != manifest["process"]["pid"]
                or not isinstance(row.get("utc_before_ns"), int) or not isinstance(row.get("utc_after_ns"), int)
                or row["utc_after_ns"] < row["utc_before_ns"]
                or (previous is not None and row["utc_before_ns"] <= previous)):
            raise ValueError("native run-time condition sequence/PID/clock missing or malformed")
        previous = row["utc_after_ns"]
        rows.append(row)
    if len(rows) != manifest["process"].get("condition_samples"):
        raise ValueError("native condition sample count differs from the owned-process record")
    start, end = timing["score_utc_ns"]
    selected = [row for row in rows if start - 1_500_000_000 <= row["utc_after_ns"] <= end + 1_500_000_000]
    if (not selected or selected[0]["utc_before_ns"] > start or selected[-1]["utc_after_ns"] < end
            or any(b["utc_before_ns"] - a["utc_after_ns"] > 1_500_000_000 for a, b in zip(selected, selected[1:]))):
        raise ValueError("native conditions do not cover scoring at bounded one-second cadence")
    facts = []
    for row in selected:
        display, power = row.get("display", {}), row.get("power", {})
        if display.get("available") is not True or power.get("available") is not True:
            raise ValueError("required native display/power condition unavailable in scoring")
        driver = display.get("driver", {})
        if driver.get("available") is not True or not driver.get("version") or not driver.get("source"):
            raise ValueError("required native numeric driver version/provenance unavailable")
        device = display.get("adapter_device_id", "").upper()
        if f'VEN_{actual_adapter["vendor"]:04X}' not in device or f'DEV_{actual_adapter["device"]:04X}' not in device:
            raise ValueError("native display driver does not match the actual rendering adapter")
        for key in ("refresh_hz", "client_pixels", "desktop_pixels", "monitor_device", "source"):
            if not display.get(key):
                raise ValueError(f"required native display {key} missing")
        rational = display.get("refresh_rational", {})
        numerator, denominator = rational.get("numerator"), rational.get("denominator")
        if (type(numerator) is not int or type(denominator) is not int or numerator <= 0 or denominator <= 0
                or rational.get("hz") != numerator / denominator or display["refresh_hz"] != rational["hz"]
                or not rational.get("source")):
            raise ValueError("exact nominal active-path refresh rational/provenance unavailable")
        for key in ("ac_line", "battery_flags", "battery_saver", "active_scheme", "effective_overlay", "source"):
            if key not in power or power[key] is None:
                raise ValueError(f"required native power {key} missing")
        facts.append({"display": display, "power": power})
    if any(fact != facts[0] for fact in facts[1:]):
        raise ValueError("required native comparison conditions changed during scoring")
    return {"facts": facts[0], "scored_samples_with_boundary_neighbors": len(selected),
            "whole_session_samples": len(rows),
            "limitation": "one-second native snapshots can miss intervening condition changes; no thermal/background-load equivalence claim"}


def read_run(directory, collection):
    from scenario_run import completion
    directory = Path(directory).resolve(strict=True)
    path = directory / "manifest.json"
    if path.stat().st_size > 131_072:
        raise ValueError("comparison manifest exceeds 128 KiB")
    manifest = json.loads(path.read_text(encoding="utf-8"))
    if (manifest.get("schema") != "input-scenario-v1" or manifest.get("complete") is not True
            or manifest.get("common_timing") is not True or manifest.get("collection") != collection
            or manifest.get("settings_unchanged") is not True or manifest.get("tracked_changes") != ""
            or manifest.get("process", {}).get("status") != "completed" or manifest["process"].get("exit_code") != 0):
        raise ValueError("comparison requires complete attributable isolated runs with the stated collection mode")
    if manifest.get("measurement_windows") != {"warmup_s": 30.0, "scored_s": 120.0}:
        raise ValueError("overhead comparison requires exactly 30 s warm-up and 120 s scoring")
    process = json.loads((directory / "process.json").read_text(encoding="utf-8"))
    if process != manifest["process"] or manifest.get("settings_before") != manifest.get("settings_after"):
        raise ValueError("raw owned-process/profile-isolation evidence differs from manifest")
    if not completion(process, (directory / "stdout.txt").read_text(encoding="utf-8"),
                      (directory / "app/scenario.tsv").read_text(encoding="utf-8"), True,
                      manifest.get("mode") == "production-profile"):
        raise ValueError("retained app/input evidence did not complete")
    receipt = verify_record(directory / "build-receipt.json", manifest["commit"],
                            manifest["executable_sha256"], manifest["executable_bytes"])
    if manifest.get("build_receipt") != receipt:
        raise ValueError("retained receipt differs from its verified launch attribution")
    if digest(directory / "fixture.scenario") != manifest.get("fixture_sha256"):
        raise ValueError("retained comparison fixture hash mismatch")
    timing = timing_summary(directory, 30, 120)
    if timing["collection"] != ("full" if collection == "on" else "intervals-only"):
        raise ValueError("actual timing collection differs from the launch manifest")
    if collection == "on":
        full = measure_summary(directory, 30, 120)
        if full["scored"]["frame_interval_ms"] != timing["scored"]["frame_interval_ms"]:
            raise ValueError("full and common interval reports disagree")
        sampling = full["working_set"]
    else:
        if (manifest["process"].get("sampling_enabled") is not False
                or (directory / "memory.jsonl").stat().st_size or (directory / "app/loading-v1.tsv").exists()):
            raise ValueError("collection-off unexpectedly ran the full collector/process sampler")
        sampling = {"enabled": False, "reason": "full process sampler disabled on collection-off side"}
    actual_adapter = adapter(directory / "stderr.txt")
    native_conditions = native(directory, manifest, timing, actual_adapter)
    states = [{key: row[key] for key in CONDITION_KEYS} for row in timing["scored_graphics_states"]]
    if not states or any(state != states[0] for state in states[1:]) or any("unavailable" in value for value in states[0].values()):
        raise ValueError("required saved/effective graphics state unavailable or changing during scoring")
    if native_conditions["facts"]["display"]["client_pixels"] != list(map(int, states[0]["output_pixels"].split("x"))):
        raise ValueError("native/app output dimensions do not match")
    machine = manifest.get("machine", {})
    if machine.get("available") is not True or not machine.get("name") or not machine.get("physical_ram_bytes"):
        raise ValueError("reference-machine hardware identity unavailable")
    identity = {"source_commit": receipt["source_commit"], "source_tree": receipt["source_tree"],
                "executable_sha256": receipt["executable_sha256"], "fixture_sha256": manifest["fixture_sha256"],
                "machine": machine, "adapter": actual_adapter, "native": native_conditions["facts"],
                "graphics": states[0], "workers": manifest.get("workers"),
                "environment": manifest.get("comparison_environment"), "mode": manifest.get("mode")}
    if not identity["workers"] or identity["environment"] is None:
        raise ValueError("worker/loader comparison conditions missing")
    start, end = (datetime.fromisoformat(manifest[key]) for key in ("started_utc", "completed_utc"))
    if start.tzinfo is None or end.tzinfo is None or start >= end:
        raise ValueError("run serial-lifetime boundaries are unavailable/malformed")
    return {"directory": str(directory), "collection": collection, "identity": identity,
            "started_utc": start.isoformat(), "completed_utc": end.isoformat(),
            "scored": timing["scored"], "whole_session": timing["whole_session"], "working_set": sampling,
            "native_sampling": native_conditions, "evidence_bytes": manifest.get("evidence_bytes", {}),
            "clock_bracket_drift_ns": timing["clock_bracket_drift_ns"]}


def overhead(runs):
    if len(runs) != 6 or len({run["directory"] for run in runs}) != 6:
        raise ValueError("exactly three distinct serial off/on pairs are required")
    for index, run in enumerate(runs):
        if run["collection"] != ("off" if index % 2 == 0 else "on"):
            raise ValueError("pairs must be in serial collection-off/on order")
        if run["identity"] != runs[0]["identity"]:
            raise ValueError("required source/hash/fixture/hardware/native/graphics/worker conditions mismatch")
        if index and datetime.fromisoformat(run["started_utc"]) <= datetime.fromisoformat(runs[index - 1]["completed_utc"]):
            raise ValueError("comparison run lifetimes overlap or are not serial")
    metrics = {}
    for key, absolute in (("mean", 1.0), ("p95", 2.0)):
        off = statistics.median(run["scored"]["frame_interval_ms"][key] for run in runs[0::2])
        on = statistics.median(run["scored"]["frame_interval_ms"][key] for run in runs[1::2])
        limit = max(absolute, off * 0.05)
        metrics[key] = {"off_median_ms": off, "on_median_ms": on, "growth_ms": on - off,
                        "limit_ms": limit, "within_limit": on - off <= limit}
    return {"schema": "input-collection-overhead-v1", "complete": True, "matched_comparison": True,
            "pairs": 3, "warmup_s": 30, "scored_s": 120, "metrics": metrics,
            "overhead_within_limits": all(metric["within_limit"] for metric in metrics.values()), "runs": runs,
            "limitations": ["common minimal interval/clock/graphics recorder is present on both sides",
                            "collection overhead includes full state/work/loading/resource journals, streaming and process samples",
                            "stalls retained individually and in whole-session/scored worst values; no outlier removal",
                            "one-second working-set and condition sampling can miss intervening peaks/changes",
                            "native driver is mapped to actual rendering adapter; nominal refresh is not physical scan-out",
                            "same recorded conditions do not prove equal background load, temperature or driver-internal wait mode",
                            "testing-tool overhead only; no future moon-gameplay or feature-budget verdict"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pair", type=Path, nargs=2, action="append", required=True, metavar=("OFF", "ON"))
    args = parser.parse_args()
    try:
        if len(args.pair) != 3:
            raise ValueError("exactly three pairs are required")
        runs = [read_run(directory, mode) for pair in args.pair for directory, mode in zip(pair, ("off", "on"))]
        report = overhead(runs)
        print(json.dumps(report, indent=2, allow_nan=False))
        return 0 if report["overhead_within_limits"] else 1
    except (OSError, ValueError, KeyError, TypeError) as exc:
        print(json.dumps({"complete": False, "matched_comparison": False, "reason": str(exc)}))
        return 1


if __name__ == "__main__":
    sys.exit(main())
