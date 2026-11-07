"""Fixed serial public-interface regression groups for #1238, never a build wrapper.

python scripts/validation/scenario_suite.py --group smoke --build-receipt FILE --out NEW_DIRECTORY
Run groups serially. Nothing here certifies future moon gameplay or owner visuals.
"""

import argparse
import json
import os
from pathlib import Path
import sys

from profile_process import write_json
from profile_run import digest, player_settings
from scenario_persistence import observation
from scenario_suite_support import EXISTING_OUTPUT_REASON, ROOT, Suite, diagnostics, read_json

GROUPS = ("smoke", "profiles", "measurements", "negatives", "legacy", "overhead", "all")


def smoke(suite):
    for name in ("planet", "planet-other", "near-moon", "system", "interstellar", "intergalactic", "menu", "scroll",
                 "preparation-phases", "loaded-start", "production-observations", "moon-variety"):
        suite.scenario(name, name)
    suite.scenario("ui-narrow", "ui-layout-narrow", extra=("--width", "640", "--height", "600"))
    starts = [suite.scenario(f"selected-moon-{n}", "selected-moon") for n in (1, 2)]
    values = [observation(path / "app/scenario.tsv", "setup") for path in starts]
    keys = ("seed", "fixture.selected.planet", "fixture.selected.system", "fixture.selected.moon",
            "fixture.moon.type", "fixture.moon.radius_km", "fixture.direction_basis", "clock.game_s",
            "player.position", "player.position.frame", "player.flying", "player.on_ground")
    suite.record_result({"name": "repeat-selected-start", "fields": {key: [v.get(key) for v in values] for key in keys},
                         "as_expected": all(values[0].get(key) is not None and values[0][key] == values[1].get(key) for key in keys)})
    suite.record_result(suite.command("catalog", "scenario_catalog.py", ["--build-receipt", suite.receipt, "--out", suite.out / "catalog"]))


def profiles(suite):
    profile = suite.out / "fresh-profile"
    first = suite.scenario("fresh-initial", "profile-quit", profile=profile)
    second = suite.scenario("fresh-restart", "profile-quit", profile=profile)
    suite.compare("fresh-comparison", first, second)
    profile = suite.out / "views-profile"
    first = suite.scenario("views-save", "profile-save", profile=profile, timeout=330)
    second = suite.scenario("views-recall", "profile-recall", profile=profile, timeout=330)
    suite.compare("views-comparison", first, second, True)
    for seed in ("old-planet", "old-space", "missing-records", "damaged-records", "unresolvable-records"):
        profile = suite.out / f"{seed}-profile"
        first = suite.scenario(f"{seed}-initial", f"profile-{seed}", profile=profile, seed=seed)
        second = suite.scenario(f"{seed}-restart", "profile-quit", profile=profile)
        suite.compare(f"{seed}-comparison", first, second)
    for kind in ("player", "views", "display"):
        profile = suite.out / f"reset-{kind}-profile"
        first = suite.scenario(f"reset-{kind}-initial", f"profile-reset-{kind}", profile=profile, seed="old-planet")
        second = suite.scenario(f"reset-{kind}-restart", "profile-quit", profile=profile)
        suite.compare(f"reset-{kind}-comparison", first, second)


def measurements(suite):
    planet = suite.scenario("measured-planet", "input-profile-planet", collection="--measure")
    suite.scenario("measured-moons", "input-profile-moons", collection="--measure", timeout=660, extra=("--require-multi-moon",))
    suite.scenario("loading-transitions", "loading-transitions", collection="--loading", timeout=270)
    profile = suite.out / "measured-profile"
    first = suite.scenario("measured-save", "profile-save", profile=profile, collection="--measure", timeout=330)
    second = suite.scenario("measured-restart", "profile-recall", profile=profile, collection="--measure", timeout=330)
    suite.compare("measured-persistence", first, second, True)
    suite.record_result(suite.command("measurement-audit", "scenario_audit.py", ["--run", planet, "--out", suite.out / "measurement-audit"]))


def negatives(suite):
    suite.scenario("missing-fixture", "deliberately-missing-fixture", reason="deliberately-missing-fixture.scenario")
    cases = (("missing-control", "missing or invisible button"), ("unmet", "unmet Equal"),
             ("malformed", "unknown input"), ("timeout", "wait timeout"),
             ("invalid-moon", "existing moon"), ("missing-moon-ground", "missing observation"),
             ("missing-sky-visibility", "missing observation"))
    for fixture, reason in cases:
        suite.scenario(fixture, fixture, reason=reason)
    suite.scenario("outer-timeout", "planet", reason="child exceeded the configured timeout", timeout=0.1)
    suite.scenario("short-scoring", "planet", collection="--measure", reason="entire requested scoring", extra=("--score-seconds", "120"))
    for fixture, reason in (("no-quit", "require final public quit"), ("incomplete-quit", "total timeout")):
        suite.scenario(fixture, f"profile-{fixture}", profile=suite.out / f"{fixture}-profile", reason=reason, timeout=40)
    unmarked = suite.out / "unmarked-profile"
    unmarked.mkdir()
    sentinel = unmarked / "untouched.txt"
    sentinel.write_bytes(b"illustrative disposable refusal fixture\n")
    original = digest(sentinel)
    suite.scenario("unmarked", "profile-quit", profile=unmarked, reason="exact disposable input marker")
    suite.record_result({"name": "unmarked-preserved", "as_expected": digest(sentinel) == original and list(unmarked.iterdir()) == [sentinel]})
    suite.scenario("real-root", "profile-quit", profile=player_settings().parent.parent, reason="outside the real configuration root")
    suite.scenario("overlap", "profile-quit", profile=suite.out / "overlap/profile", reason="must be separate", target=suite.out / "overlap")
    profile = suite.out / "restart-refusal-profile"
    suite.scenario("restart-refusal-initial", "profile-quit", profile=profile)
    original = digest(profile / "Astrolith/settings.toml")
    suite.scenario("restart-preparation", "profile-save", profile=profile, reason="restarted profile cannot replay preparation")
    suite.scenario("seed-on-reuse", "profile-quit", profile=profile, seed="old-planet", reason="only seed a new disposable profile")
    suite.record_result({"name": "refused-profile-preserved", "as_expected": digest(profile / "Astrolith/settings.toml") == original})
    invalid = suite.out / "bad-marker-profile"
    invalid.mkdir()
    (invalid / ".astrolith-profile").write_bytes(b"disposable-v1\r\n")
    suite.scenario("bad-marker", "profile-quit", profile=invalid, reason="exact disposable input marker")
    hardlinked = suite.out / "hardlink-profile"
    (hardlinked / "Astrolith").mkdir(parents=True)
    (hardlinked / ".astrolith-profile").write_bytes(b"disposable-v1\n")
    original_file = suite.out / "illustrative-hardlink-original.toml"
    original_file.write_bytes(b"illustrative unrelated bytes\n")
    os.link(original_file, hardlinked / "Astrolith/settings.toml")
    suite.scenario("hardlink", "profile-quit", profile=hardlinked, reason="multiple hard links")
    suite.record_result({"name": "hardlink-original-preserved", "as_expected": original_file.read_bytes() == b"illustrative unrelated bytes\n"})
    # Native Python/Windows junction setup is fixed to these disposable paths;
    # no shell, privilege prompt, external target or caller command is involved.
    import _winapi
    destination = suite.out / "junction-destination"
    destination.mkdir()
    sentinel = destination / "untouched.txt"
    sentinel.write_bytes(b"junction destination must stay untouched\n")
    link = suite.out / "junction-profile"
    _winapi.CreateJunction(str(destination), str(link))
    suite.scenario("junction", "profile-quit", profile=link, reason="links or reparse points")
    suite.record_result({"name": "junction-destination-preserved", "as_expected": sentinel.read_bytes() == b"junction destination must stay untouched\n" and list(destination.iterdir()) == [sentinel]})
    for key, value, name in (("executable_sha256", "0" * 64, "bad-binary"), ("source_commit_after", "0" * 40, "bad-source")):
        record = read_json(suite.receipt)
        receipt = suite.out / f"deliberately-corrupted-{name}-receipt.json"
        write_json(receipt, {**record, key: value})
        suite.scenario(name, "planet", collection="--timing-only", receipt=receipt, reason="build receipt " + ("executable hash/size mismatch" if name == "bad-binary" else "source commit/tree mismatch"))
    existing = suite.out / "existing-evidence"
    existing.mkdir()
    sentinel = existing / "untouched.txt"
    sentinel.write_bytes(b"existing evidence must not change\n")
    suite.scenario("existing-output", "planet", target=existing, reason=EXISTING_OUTPUT_REASON)
    suite.record_result({"name": "existing-evidence-preserved", "as_expected": sentinel.read_bytes() == b"existing evidence must not change\n" and list(existing.iterdir()) == [sentinel]})


def legacy(suite):
    for fixture in ("planet", "menu", "scroll"):
        suite.scenario(f"legacy-{fixture}", fixture)
    args = ["--route", ROOT / "scripts/validation/fixtures/profile/short-normal.route", "--out", suite.out / "legacy-route", "--timeout", "240"]
    row = suite.command("legacy-route", "profile_run.py", args)
    manifest = read_json(suite.out / "legacy-route/manifest.json")
    scan = diagnostics([suite.out / "legacy-route/stdout.txt", suite.out / "legacy-route/stderr.txt"])
    row.update(log_scan=scan, manifest=manifest)
    row["as_expected"] &= manifest.get("complete") is True and manifest.get("settings_unchanged") is True and not scan["unexpected"]
    suite.record_result(row)
    suite.normal()
    suite.normal(switches=True)


def overhead(suite):
    arguments = []
    for number in range(1, 4):
        pair = []
        for mode, flag in (("off", "--timing-only"), ("on", "--measure")):
            pair.append(suite.scenario(f"{mode}-{number}", "collection-overhead", collection=flag, timeout=240,
                                       extra=("--warmup-seconds", "30", "--score-seconds", "120")))
        arguments += ["--pair", *pair]
    suite.record_result(suite.command("overhead-report", "scenario_comparison.py", arguments))
    suite.record_result(suite.command("overhead-audit", "scenario_audit.py", [*arguments, "--out", suite.out / "overhead-audit"]))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--group", choices=GROUPS, required=True)
    parser.add_argument("--build-receipt", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    suite = None
    try:
        suite = Suite(args.out, args.build_receipt, args.group)
        for name in GROUPS[:-1] if args.group == "all" else (args.group,):
            globals()[name](suite)
        return suite.finish()
    except (OSError, ValueError, KeyError, TypeError) as exc:
        if suite is not None:
            suite.record["problem"] = str(exc)
            suite.record["complete"] = False
            suite.save()
        print(json.dumps({"complete": False, "reason": str(exc)}), flush=True)
        return 1


if __name__ == "__main__":
    sys.exit(main())
