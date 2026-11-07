"""Read-only persistence evidence checks, not a replacement for public actions.

Compare production serialization with the disposable saved bytes and the next
launch's loaded observation. The current camera mode is not a startup-save field.
"""

import argparse
import json
import math
from pathlib import Path
import sys

from profile_process import write_json
from profile_run import digest


def observation(trace, name):
    result = {}
    seen = False
    if trace.stat().st_size > 268_435_456:
        raise ValueError("scenario trace exceeds the bounded 256 MiB report limit")
    with trace.open(encoding="utf-8") as stream:
        for line in stream:
            parts = line.rstrip("\n").split("\t", 3)
            if parts[:2] == ["OBSERVE", name]:
                if seen:
                    raise ValueError(f"duplicate observation {name}")
                seen = True
            if len(parts) == 4 and parts[:2] == ["VALUE", name]:
                if not seen or parts[2] in result:
                    raise ValueError(f"unowned or duplicate value in {name}")
                result[parts[2]] = parts[3]
                if len(result) > 32768:
                    raise ValueError("observation value limit exceeded")
    if not seen or not result or sum(len(k) + len(v) for k, v in result.items()) > 2_097_152:
        raise ValueError(f"missing or oversized observation {name}")
    return result


def unescape(text):
    result = []
    chars = iter(text)
    for char in chars:
        if char == "\\":
            code = next(chars, None)
            if code not in ("\\", "t", "r", "n"):
                raise ValueError("invalid escaped observation text")
            char = {"\\": "\\", "t": "\t", "r": "\r", "n": "\n"}[code]
        result.append(char)
    return "".join(result)


def serialized(snapshot):
    prefix = "settings.saved."
    result = {}
    for key, value in snapshot.items():
        if key.startswith(prefix) and not key.endswith(".encoding"):
            if snapshot.get(key + ".encoding") != "backslash-escaped-utf8":
                raise ValueError("unlabeled settings text")
            result[key[len(prefix):]] = unescape(value)
    if not {"game_time", "labels", "quality_mode", "planet_detail"}.issubset(result):
        raise ValueError("required production settings serialization is missing")
    return result


def file_fields(file):
    if not file.is_file() or file.stat().st_size > 131_072:
        raise ValueError("saved disposable settings are missing or oversized")
    result = {}
    for line in file.read_text(encoding="utf-8").splitlines():
        if not line or line.startswith("#"):
            continue
        key, separator, value = line.partition(" = ")
        if not separator or key in result:
            raise ValueError("saved file is not unique production serialization")
        result[key] = value
    return result


def saved_matches(trace, file):
    return serialized(observation(trace, "terminal")) == file_fields(file)


def compare(before, restart, recalls=False):
    first = json.loads((before / "manifest.json").read_text(encoding="utf-8"))
    second = json.loads((restart / "manifest.json").read_text(encoding="utf-8"))
    for manifest in (first, second):
        if (not manifest.get("complete") or not manifest.get("settings_unchanged")
                or manifest.get("mode") != "production-profile"):
            raise ValueError("both launches must be complete isolated production-profile runs")
    if first["disposable_profile"] != second["disposable_profile"]:
        raise ValueError("launches did not use the same disposable profile")
    if first["profile_after"]["sha256"] != second["profile_before"]["sha256"]:
        raise ValueError("profile bytes changed between Quit and restart")
    if not second["profile_initialization"]["restart"] or second["profile_initialization"]["initial_fixture"] is not None:
        raise ValueError("second launch is not an unseeded profile restart")
    trace = restart / "app/scenario.tsv"
    with trace.open(encoding="utf-8") as stream:
        if any(line.startswith(("PREPARE\t", "PLACEMENT\t", "PHASE\tpreparation\t")) for line in stream):
            raise ValueError("restart replayed preparation")
    old_trace = before / "app/scenario.tsv"
    old_file = before / "profile-after.toml"
    if digest(old_file) != first["profile_after"]["sha256"]:
        raise ValueError("retained saved bytes do not match the manifest")
    if not saved_matches(old_trace, old_file):
        raise ValueError("production saved file differs from the actual Quit state")
    if digest(restart / "profile-before.toml") != second["profile_before"]["sha256"]:
        raise ValueError("retained loaded bytes do not match the manifest")
    if (digest(restart / "profile-after.toml") != second["profile_after"]["sha256"]
            or not saved_matches(trace, restart / "profile-after.toml")):
        raise ValueError("restart Quit did not retain its actual saved state")
    old = observation(old_trace, "terminal")
    loaded = observation(trace, "setup")
    previous, current = serialized(old), serialized(loaded)
    differences = sorted(k for k in previous.keys() | current.keys() if previous.get(k) != current.get(k))
    result = {"schema": "input-persistence-comparison-v1", "complete": not differences,
              "before": str(before), "restart": str(restart),
              "saved_fields_compared": len(previous), "saved_field_differences": differences,
              "saved_file_matches_quit_state": True, "loaded_before_actions_without_preparation": True,
              "slot_recalls": [], "nonserialized_view_differences": [],
              "limitations": ["current camera mode/pose is not automatically restored at startup",
                              "preparation is not travel; moon-relative production saving is not implemented here"]}
    # These live-only fields are not serialized. Retain differences rather than
    # calling them persisted or silently changing the production file format.
    for slot in range(1, 9):
        key = f"views.saved.{slot}.anchored"
        if old.get(key) != loaded.get(key):
            result["nonserialized_view_differences"].append({"field": key, "before": old.get(key), "loaded": loaded.get(key)})
        if recalls:
            recalled = observation(trace, f"recalled-{slot}")
            prefix = f"views.saved.{slot}."
            mode = recalled.get("player.mode")
            fields_match = loaded.get(prefix + "filled") == "true" and mode == loaded.get(prefix + "mode")
            for live, saved in (("camera.look_yaw_rad", "yaw_rad"), ("camera.look_pitch_rad", "pitch_rad")):
                try:
                    a, b = float(recalled[live]), float(loaded[prefix + saved])
                    fields_match &= math.isfinite(a) and math.isfinite(b) and abs(a - b) <= 1e-12
                except (KeyError, ValueError):
                    fields_match = False
            result["slot_recalls"].append({"slot": slot, "mode": mode, "mode_look_match": fields_match})
            result["complete"] &= fields_match
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before", required=True, type=Path)
    parser.add_argument("--restart", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--recalls", action="store_true", help="require all eight named public Go observations")
    args = parser.parse_args()
    created = False
    try:
        args.out.mkdir(parents=True, exist_ok=False)
        created = True
        result = compare(args.before.resolve(), args.restart.resolve(), args.recalls)
    except (OSError, ValueError, KeyError) as exc:
        result = {"complete": False, "reason": str(exc)}
        # Never write to an existing directory after a refusal.
        if not created:
            print(json.dumps(result))
            return 1
    write_json(args.out / "report.json", result)
    print(json.dumps(result))
    return 0 if result["complete"] else 1


if __name__ == "__main__":
    sys.exit(main())
