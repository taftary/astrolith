"""Bounded real-window input scenarios (#1227, Windows).

Build: cargo build --locked --release -p universe-app
Run: python scripts/validation/scenario_run.py --scenario FILE --out NEW_DIRECTORY
No shell commands, settings writes, or desktop automation are accepted as actions.
"""

import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import subprocess
import sys

from profile_process import observe, write_json
from profile_run import (APP, ROOT, RunLock, digest, finite_positive, fingerprint,
                          git, player_settings, positive_integer)
import scenario_profile
from scenario_loading import summarize as loading_summary
from scenario_persistence import profile_trace, saved_matches
from scenario_measure import summarize as measure_summary


def parser():
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument("--scenario", type=Path, required=True)
    result.add_argument("--out", type=Path, required=True)
    result.add_argument("--timeout", type=finite_positive, default=180.0,
                        help="outer process bound, including graphics initialization")
    result.add_argument("--width", type=positive_integer, default=1600)
    result.add_argument("--height", type=positive_integer, default=900)
    result.add_argument("--workers", type=positive_integer, default=5)
    result.add_argument("--profile", type=Path,
                        help="explicit disposable production profile, reused on restart")
    result.add_argument("--profile-fixture", type=Path,
                        help="initial settings bytes, only for a new --profile")
    collection = result.add_mutually_exclusive_group()
    collection.add_argument("--loading", action="store_true",
                         help="explicit bounded loading/resource sidecar; not integrated timing proof")
    collection.add_argument("--measure", action="store_true",
                            help="integrated real-input timing, loading, state and process sampling")
    result.add_argument("--warmup-seconds", type=finite_positive,
                        help="exclude this initial action duration from scoring, not whole-session peaks")
    result.add_argument("--score-seconds", type=finite_positive,
                        help="require this entire duration after warm-up; default is the remaining session")
    result.add_argument("--require-multi-moon", action="store_true",
                        help="require observed 20 moons/five systems/20 detail changes/8,64,128 MiB/revisit")
    return result


def completion(process, stdout, trace, unchanged, persistent=False):
    """Fail closed: a zero exit, partial trace, or stray OK alone proves nothing."""
    stdout = stdout.splitlines()
    trace = trace.splitlines()
    normal_quit = True
    if persistent:
        try:
            profile_trace(trace)
        except ValueError:
            normal_quit = False
    return (normal_quit and process.get("status") == "completed" and process.get("exit_code") == 0
            and unchanged and stdout[-1:] == ["SCENARIO-OK"]
            and trace[-1:] == ["SCENARIO-OK"]
            and any(line.startswith("SCENARIO-RUN\tversion=1\t") for line in trace)
            and "PHASE\tactions" in trace
            and any(line.startswith("OBSERVE\tsetup\t") for line in trace)
            and any(line.startswith("ACTION\t") for line in trace)
            and any(line.startswith("OBSERVE\tterminal\t") for line in trace)
            and not any(line.startswith("SCENARIO-FAIL") for line in stdout + trace))


def run(args):
    if os.name != "nt":
        raise OSError("the documented scenario runner currently supports Windows only")
    source = args.scenario.resolve(strict=True)
    if not source.is_file() or source.stat().st_size > 131_072:
        raise ValueError("scenario must be a file no larger than 128 KiB")
    if not APP.is_file():
        raise ValueError("release app missing; build it before running scenarios")
    real_settings = player_settings()
    real_root = real_settings.parent.parent.resolve()
    out = scenario_profile.outside_real(args.out, real_root)
    if args.profile_fixture is not None and args.profile is None:
        raise ValueError("--profile-fixture requires --profile")
    if (args.warmup_seconds is not None or args.score_seconds is not None or args.require_multi_moon) and not args.measure:
        raise ValueError("measurement windows/coverage require --measure")
    with RunLock():
        out.mkdir(parents=True, exist_ok=False)
        before = fingerprint(real_settings)
        manifest = {"schema": "input-scenario-v1", "complete": False,
                    "mode": "production-profile" if args.profile is not None else "memory-only",
                    "started_utc": datetime.now(timezone.utc).isoformat(),
                    "settings_before": before, "problems": []}
        write_json(out / "manifest.json", manifest)
        try:
            launch(args, source, out, real_settings, real_root, manifest)
        except (OSError, ValueError, subprocess.SubprocessError) as exc:
            manifest["complete"] = False
            manifest["problems"].append(str(exc))
            raise
        finally:
            manifest["settings_after"] = fingerprint(real_settings)
            manifest["settings_unchanged"] = before == manifest["settings_after"]
            if not manifest["settings_unchanged"]:
                manifest["complete"] = False
            write_json(out / "manifest.json", manifest)
        print(json.dumps({"ok": manifest["complete"], "out": str(out),
                          "process": manifest["process"]["status"],
                          "settings_unchanged": manifest["settings_unchanged"]}))
        return 0 if manifest["complete"] else 1


def launch(args, source, out, real_settings, real_root, manifest):
    persistent = args.profile is not None
    if persistent:
        profile, metadata = scenario_profile.prepare(args.profile, real_root, out, args.profile_fixture)
        manifest["profile_initialization"] = metadata
        manifest["profile_before"] = scenario_profile.retain(profile, out, "before")
    else:
        profile = out / "profile"
        profile.mkdir()
        (profile / ".astrolith-profile").write_bytes(b"disposable-v1\n")
    fixture = out / "fixture.scenario"
    fixture.write_bytes(source.read_bytes())
    argv = [str(APP), "--scenario-profile" if persistent else "--scenario",
            str(fixture), str(out / "app")]
    if persistent:
        argv.append(str(profile))
    if args.loading:
        argv.append("--loading")
    if args.measure:
        argv.append("--measure")
    argv += ["--size", f"{args.width}x{args.height}", "--build-workers", str(args.workers)]
    env = os.environ.copy()
    env["APPDATA"] = str(profile)
    env["RUST_BACKTRACE"] = "1"
    env["ASTROLITH_PROFILE"] = "0"
    env.pop("ASTROLITH_PROTOTYPE", None)
    env.pop("ASTROLITH_GPU_TIMERS", None)
    # Same loader defaults as .cargo/config.toml for cargo-launched runs
    # (ADR 0042). Preserve any explicit environment override.
    env.setdefault("VK_IMPLICIT_LAYER_PATH", str(ROOT / ".cargo"))
    env.setdefault("VK_LOADER_LAYERS_DISABLE", "~implicit~")
    manifest.update({
            "commit": git("rev-parse", "HEAD"),
            "tracked_changes": git("status", "--porcelain", "--untracked-files=no"),
            "executable": str(APP), "executable_sha256": digest(APP),
            "executable_mtime_ns": APP.stat().st_mtime_ns,
            "build_identity_note": "executable hash is authoritative; commit describes source, not build provenance",
            "fixture_sha256": digest(fixture), "argv": argv, "cwd": str(ROOT),
             "disposable_profile": str(profile),
              "loading_collection": args.loading or args.measure,
              "input_measurement": args.measure,
              "measurement_windows": {"warmup_s": args.warmup_seconds or 0, "scored_s": args.score_seconds},
            "timeout_s": args.timeout, "complete": False,
            "limitations": ["near-moon is hovering, not landing or moon collision ground",
                            "visitation is local to this scenario, not player discovery",
                            "scripted events exercise production input, not OS desktop control"],
    })
    write_json(out / "manifest.json", manifest)
    process = observe(argv, out, ROOT, env, args.timeout, collect=args.measure, clock_samples=args.measure)
    manifest["process"] = process
    manifest["settings_unchanged"] = manifest["settings_before"] == fingerprint(real_settings)
    if persistent:
        manifest["profile_after"] = scenario_profile.retain(profile, out, "after")
    stdout = (out / "stdout.txt").read_text(encoding="utf-8", errors="replace")
    trace_file = out / "app" / "scenario.tsv"
    trace = trace_file.read_text(encoding="utf-8", errors="replace") if trace_file.is_file() else ""
    manifest["complete"] = completion(process, stdout, trace, manifest["settings_unchanged"], persistent)
    if args.loading or args.measure:
        try:
            report = loading_summary(out / "app" / "loading-v1.tsv")
            write_json(out / "loading-report.json", report)
            manifest["loading_report"] = str(out / "loading-report.json")
        except (OSError, ValueError) as exc:
            manifest["problems"].append(f"loading evidence: {exc}")
            manifest["complete"] = False
    if args.measure:
        try:
            report = measure_summary(out, args.warmup_seconds or 0, args.score_seconds)
            write_json(out / "input-report.json", report)
            manifest["input_report"] = str(out / "input-report.json")
            coverage = report["readiness"]
            if args.require_multi_moon and not (coverage["distinct_selected_moons"] >= 20 and coverage["containing_systems_for_selected_moons"] >= 5 and coverage["actual_saved_detail_changes"] >= 20 and {"8", "64", "128"}.issubset(coverage["actual_saved_memory_choices_mib"]) and coverage["initial_body_revisited"]):
                raise ValueError("observed multi-moon/detail/memory/revisit coverage is incomplete")
        except (OSError, ValueError, KeyError, TypeError) as exc:
            manifest["problems"].append(f"input measurement evidence: {exc}")
            manifest["complete"] = False
            write_json(out / "input-incomplete.json", {"complete": False, "reason": str(exc), "partial_raw_evidence_retained": True})
    if persistent and not manifest["profile_after"]["exists"]:
        manifest["problems"].append("normal Quit produced no disposable settings file")
        manifest["complete"] = False
    if persistent and manifest["complete"]:
        manifest["saved_state_matches_file"] = saved_matches(trace_file, out / "profile-after.toml")
        if not manifest["saved_state_matches_file"]:
            manifest["problems"].append("production settings file differs from the actual Quit state")
            manifest["complete"] = False


def main():
    args = parser().parse_args()
    try:
        return run(args)
    except (OSError, ValueError, subprocess.SubprocessError) as exc:
        print(json.dumps({"ok": False, "reason": str(exc)}))
        return 1


if __name__ == "__main__":
    sys.exit(main())
