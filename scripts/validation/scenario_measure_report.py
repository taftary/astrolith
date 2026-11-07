"""Raw-derived phase/resource/readiness and bracketed process-memory summaries."""

from bisect import bisect_right
from collections import Counter
import json
import math
import re

from profile_report import distribution
from scenario_loading import LoadingEvidence, fields as loading_fields
from scenario_measure_trace import fields, integer, lines


DRIFT_ALLOWANCE_NS = 5_000_000


def readiness(trace, frames):
    visits, seen = [], set()
    moons, systems = set(), set()
    for index, phase in enumerate(trace.phases):
        ready = frames.ready_phases[index]
        scene = frames.scenes[ready["scene"]]
        body = scene[:2] + scene[4:5]
        visit = {"name": phase["name"], "phase": phase["kind"], "readiness_scope": "fixture-not-contact",
                 "temperature": "warm-body-revisit" if body in seen else "cold-process-first-body-visit-not-disk-cache-claim",
                 "from_phase_start_s": (ready["elapsed_ns"] - phase["start_ns"]) / 1e9,
                 "from_placement_s": None if "placement_ns" not in phase else (ready["elapsed_ns"] - phase["placement_ns"]) / 1e9,
                 "actions_after_readiness_s": (phase["actions_ns"] - ready["elapsed_ns"]) / 1e9,
                 "seed": scene[0], "host_path": scene[1], "selected_moon": scene[4],
                 "system": scene[3], "counts_as_travel": False, **ready}
        visits.append(visit)
        seen.add(body)
        if scene[4].isdecimal():
            moons.add((scene[0], scene[2], scene[4]))
            systems.add((scene[0], scene[3]))
    cold = [v["from_phase_start_s"] for v in visits if v["temperature"].startswith("cold")]
    warm = [v["from_phase_start_s"] for v in visits if v["temperature"].startswith("warm")]
    detail_changes, memory_choices, last_detail = 0, set(), None
    for quality in frames.quality_changes:
        detail = quality["settings.saved.planet_detail"]
        if last_detail is not None and detail != last_detail:
            detail_changes += 1
        last_detail = detail
        memory_choices.add(quality["settings.saved.ground_memory_limit"])
    first, last = visits[0], visits[-1]
    return {"visits": visits, "cold_first_body_visit_s": distribution(cold), "warm_body_revisit_s": distribution(warm),
            "distinct_selected_moons": len(moons), "containing_systems_for_selected_moons": len(systems),
            "actual_saved_detail_changes": detail_changes, "actual_saved_memory_choices_mib": sorted(memory_choices),
            "initial_body_revisited": len(visits) > 1 and (first["seed"], first["host_path"], first["selected_moon"]) == (last["seed"], last["host_path"], last["selected_moon"]),
            "selected_moon_ready_samples_with_actual_contact": sum(v["selected_moon"].isdecimal() and v["on_ground"] == "true" and v["support"] != "unavailable" for v in visits),
            "preparation_is_travel": False}


def loading_correlation(path, frames):
    # Reuses the strict lifecycle parser; per-phase peaks never reset live ownership.
    from scenario_loading import summarize
    summary = summarize(path)
    if summary["main_frames"] != len(frames.begin) - 1 or summary["rendered_main_frames"] != frames.rendered:
        raise ValueError("input/loading main or extracted render frame frontiers differ")
    groups = [{"name": p["name"], "include_preparation_in_peaks": True, "events": Counter(), "high_water": Counter()} for p in frames.trace.phases]
    evidence = LoadingEvidence()
    for text in lines(path):
        tag, row = loading_fields(text)
        if tag != "LOAD-EVENT":
            continue
        stamp, observed = integer(row, "elapsed_ns"), integer(row, "observed_main_frame")
        if observed >= len(frames.begin) or stamp > frames.end_ns:
            raise ValueError("loading event does not belong to the measured frame/clock interval")
        kind = row["kind"]
        if kind in ("resource-scene", "render-frame"):
            frame = observed if kind == "resource-scene" else integer(row, "extracted_main_frame")
            if not 1 <= frame < len(frames.begin):
                raise ValueError("resource checkpoint lacks its input frame identity")
            scene = frames.scenes[frames.scene[frame]]
            host = scene[1] if re.fullmatch(r"[0-9]+(?:\.[0-9]+){8}", scene[1]) else "unavailable-not-open-planet"
            if row["root_seed"] != scene[0] or row["host_path"] != host:
                raise ValueError("resource checkpoint changed its actual main/extracted scene identity")
        evidence.event(row)
        group = groups[max(0, bisect_right(frames.phase_starts, stamp) - 1)]
        group["events"][kind] += 1
        values = {"actual_running_jobs": evidence.running}
        if kind == "render-frame":
            values["tracked_resident_logical_gpu_bytes"] = sum(v["gpu_bytes"] for v in evidence.resources.values() if v["resident"])
        for key, value in row.items():
            if key.endswith(("_bytes", "_entries", "_writes", "_resident", "_proxies")) or key in ("pending", "completed", "entries", "pages", "mesh_entities", "logical_submission_groups", "proxy_vertices"):
                if value.isascii() and value.isdecimal():
                    values[f"{kind}.{key}"] = int(value)
        for key, value in values.items():
            group["high_water"][key] = max(group["high_water"][key], value)
    summary["phase_high_water_including_preparation"] = groups
    summary["correlation"] = "same-app-monotonic-clock-and-exact-main-or-extracted-frame-identity"
    summary["limitations"].remove("input timing and full-session working-set integration are separate")
    return summary


def memory_correlation(directory, frames):
    process_path = directory / "process.json"
    if process_path.stat().st_size > 131_072:
        raise ValueError("process receipt exceeds bound")
    process = json.loads(process_path.read_text(encoding="utf-8"))
    if process.get("status") != "completed" or process.get("exit_code") != 0 or not process.get("sampling_enabled") or process.get("sample_period_s") != 1:
        raise ValueError("process did not complete with one-second memory sampling")
    if not frames.syncs:
        raise ValueError("required app/process clock synchronization missing")
    drift = 0
    for before, after in zip(frames.syncs, frames.syncs[1:]):
        wall = after["unix_ns"] - before["unix_ns"]
        lo = after["before_ns"] - before["after_ns"]
        hi = after["after_ns"] - before["before_ns"]
        gap = max(0, lo - wall, wall - hi)
        if gap > DRIFT_ALLOWANCE_NS:
            raise ValueError("app wall/monotonic clock discontinuity prevents memory/frame correlation")
        drift = max(drift, gap)
    sync_walls = [row["unix_ns"] for row in frames.syncs]
    samples, gaps, previous = [], [], None
    for text in lines(directory / "memory.jsonl"):
        if len(samples) >= 4096:
            raise ValueError("too many working-set samples")
        row = json.loads(text)
        required = ("elapsed_s", "query_begin_s", "query_end_s", "utc_before_ns", "utc_after_ns", "pid", "working_set_bytes", "clock_link")
        if any(key not in row for key in required):
            raise ValueError("required working-set/correlation sample field missing")
        elapsed, begin, end = (row[k] for k in ("elapsed_s", "query_begin_s", "query_end_s"))
        if any(isinstance(n, bool) or not isinstance(n, (int, float)) or not math.isfinite(n) for n in (elapsed, begin, end)) or not 0 <= elapsed <= begin <= end or (previous and elapsed <= previous["elapsed_s"]):
            raise ValueError("working-set observation time is invalid/nonmonotonic")
        if any(type(row[k]) is not int or row[k] < 1 for k in ("pid", "working_set_bytes", "utc_before_ns", "utc_after_ns")) or row["pid"] != process.get("pid") or row["utc_before_ns"] > row["utc_after_ns"] or row["clock_link"] != "bracketed-wall-observation-not-same-instant":
            raise ValueError("working-set sample ownership/value/clock bracket invalid")
        if previous:
            lo = (begin - previous["query_end_s"]) * 1e9
            hi = (end - previous["query_begin_s"]) * 1e9
            wall = row["utc_before_ns"] - previous["utc_after_ns"]
            if max(0, lo - wall, wall - hi) > DRIFT_ALLOWANCE_NS:
                raise ValueError("process wall/monotonic clock discontinuity prevents frame correlation")
            if elapsed - previous["elapsed_s"] > 1.5:
                gaps.append({"from_s": previous["elapsed_s"], "to_s": elapsed})
        anchor = frames.syncs[min(len(sync_walls) - 1, max(0, bisect_right(sync_walls, row["utc_before_ns"]) - 1))]
        allowance = DRIFT_ALLOWANCE_NS + drift
        lower = row["utc_before_ns"] - anchor["unix_ns"] + anchor["before_ns"] - allowance
        upper = row["utc_after_ns"] - anchor["unix_ns"] + anchor["after_ns"] + allowance
        correlation = {"app_elapsed_lower_ns": lower, "app_elapsed_upper_ns": upper,
                       "uncertainty_scope": "capture-brackets-plus-5ms-wall-drift-allowance-not-exact-simultaneity"}
        if upper < frames.begin[1] or lower > frames.end_ns:
            correlation["location"] = "before-app-first-frame" if upper < frames.begin[1] else "after-app-measurement-seal"
        else:
            first = max(1, bisect_right(frames.begin, lower) - 1)
            last = min(len(frames.begin) - 1, max(1, bisect_right(frames.begin, upper) - 1))
            correlation.update(location="overlaps-app-frame-range", first_frame=first, last_frame=last,
                               first_scene=frames.scene[first], last_scene=frames.scene[last],
                               first_phase=frames.trace.phases[frames.stage[first]]["name"],
                               last_phase=frames.trace.phases[frames.stage[last]]["name"])
        samples.append({**row, "correlation": correlation})
        previous = row
    if not samples or len(samples) != process.get("samples") or gaps != process.get("sampling_gaps"):
        raise ValueError("missing working-set samples or inconsistent sampling-gap receipt")
    peak = max(row["working_set_bytes"] for row in samples)
    if peak != process.get("observed_peak_working_set_bytes") or samples[-1]["query_end_s"] > process.get("elapsed_s", 0):
        raise ValueError("working-set peak or observation tail differs from process receipt")
    return {"observed_peak_working_set_bytes": peak, "sample_count": len(samples), "sample_period_s": 1,
            "sampling_gaps": gaps, "leading_gap_s": samples[0]["elapsed_s"],
            "trailing_gap_s": process["elapsed_s"] - samples[-1]["query_end_s"],
            "max_sampling_interval_s": max((b["elapsed_s"] - a["elapsed_s"] for a, b in zip(samples, samples[1:])), default=None),
            "app_clock_syncs": frames.syncs, "max_observed_clock_drift_ns": drift,
            "samples": samples, "includes_process_startup_and_all_preparation": True,
            "limitations": ["resident working set, not GPU/device allocation", "one-second samples can miss intervening process peaks",
                            "frame ranges use retained bracketed UTC observations with drift checks, not identical process clocks",
                            "app stalls create clock-anchor gaps; a discontinuity makes correlation non-successful"]}
