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


def profile_trace(lines):
    """Require loaded-before-action and actual ordered Quit evidence, not markers."""
    positions, actions, controls = {}, [], []
    start = None
    prepared = False
    last = None

    def once(key, index):
        if key in positions:
            raise ValueError(f"duplicate production-profile trace marker: {key}")
        positions[key] = index

    for index, raw in enumerate(lines):
        line = raw.rstrip("\r\n")
        last = index
        if line.startswith("SCENARIO-RUN\t"):
            if not line.startswith("SCENARIO-RUN\tversion=1\t") or not line.endswith("\tsettings=production-disposable-profile"):
                raise ValueError("not a production-profile input trace")
            once("header", index)
        elif line.startswith("PERSISTENCE\t"):
            once("persistence", index)
            fields = line.split("\t")
            start = fields[1].removeprefix("start=") if len(fields) > 1 else None
            if start not in ("Initial", "Restart"):
                raise ValueError("missing initial/restart profile provenance")
        elif line.startswith("PHASE\tsetup\t"):
            if "\tfixture=Loaded\tfixture_version=2\t" not in line:
                raise ValueError("profile trace did not begin from production-loaded state")
            once("setup", index)
        elif line == "PHASE\tactions":
            once("actions", index)
        elif line.startswith("OBSERVE\tsetup\t"):
            once("loaded", index)
        elif line.startswith("OBSERVE\tterminal\t"):
            once("terminal", index)
        elif line.startswith(("ACTION\t", "PREPARE\t")):
            fields = line.split("\t", 2)
            if len(fields) != 3 or fields[1] != str(len(actions)) or len(actions) >= 2048:
                raise ValueError("missing or misordered input action evidence")
            actions.append((index, fields[2]))
            prepared |= fields[0] == "PREPARE"
        elif line.startswith(("PLACEMENT\t", "PHASE\tpreparation\t")):
            prepared = True
        elif line.startswith("QUIT-CONTROL\t"):
            controls.append((index, line))
            if len(controls) > 5:
                raise ValueError("unexpected repeated Quit input evidence")
        elif line.startswith("NORMAL-QUIT\t"):
            if line != "NORMAL-QUIT\twitnessed=true\tsource=production-confirmation-input-and-exit":
                raise ValueError("normal Quit production witness is missing")
            once("normal", index)
        elif line == "SCENARIO-OK":
            once("success", index)
        elif line.startswith("SCENARIO-FAIL"):
            raise ValueError("profile trace contains non-success")
    required = ("header", "persistence", "setup", "loaded", "actions", "terminal", "normal", "success")
    if any(key not in positions for key in required) or not actions:
        raise ValueError("incomplete production-profile trace")
    if start == "Restart" and prepared:
        raise ValueError("restart replayed preparation")
    if actions[-1][1] != "Quit" or any(kind == "Quit" for _, kind in actions[:-1]):
        raise ValueError("public Quit is not the final input action")
    expected = [("menu-Quit", 0), ("menu-Quit", 1), ("menu-Quit", 2), ("quit-Quit", 0), ("quit-Quit", 1)]
    if [line for _, line in controls] != [
            f"QUIT-CONTROL\t{control}\tinput_phase={phase}\tstep={len(actions) - 1}"
            for control, phase in expected]:
        raise ValueError("missing or misordered public Quit input evidence")
    order = [positions[key] for key in required[:5]] + [actions[0][0]]
    if order != sorted(set(order)):
        raise ValueError("input actions precede the loaded-state observation")
    tail = [actions[-1][0]] + [index for index, _ in controls] + [positions[key] for key in required[5:]]
    if tail != sorted(set(tail)) or positions["success"] != last:
        raise ValueError("normal Quit evidence is not in production order")
    return start


def read_profile_trace(trace):
    if trace.stat().st_size > 268_435_456:
        raise ValueError("scenario trace exceeds the bounded 256 MiB report limit")
    with trace.open(encoding="utf-8") as stream:
        return profile_trace(stream)


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


def loaded_context(previous, old, loaded):
    """Check actual restored player/clock, not merely the in-memory saved store."""
    fields = ("player.remembered.path", "player.remembered.position",
              "player.remembered.position.frame", "player.remembered.yaw_rad")
    if any(old.get(key) in (None, "unavailable") or loaded.get(key) in (None, "unavailable") for key in fields):
        raise ValueError("live remembered player context is missing")
    differences = [key for key in fields if old[key] != loaded[key]]
    # Views Reset may clear the saved record while the Settings screen still
    # shows the previously parked live body. With no record the next launch
    # takes the production fallback, not that deliberately unsaved live pose.
    unexpected = differences.copy() if "player" in previous else []
    if (loaded.get("player.selected") != "true"
            or loaded.get("navigation.destination.path") != loaded["player.remembered.path"]
            or loaded.get("navigation.destination.anchored") != "true"):
        unexpected.append("loaded-player-selection-and-destination")
    saved_time, live_time = float(previous["game_time"]), float(loaded["clock.game_s"])
    if not math.isfinite(saved_time) or not math.isfinite(live_time) or live_time < saved_time:
        raise ValueError("actual loaded clock did not resume the saved time")
    return differences, unexpected, {"saved_game_s": saved_time, "observed_loaded_game_s": live_time,
                                     "startup_advance_game_s": live_time - saved_time,
                                     "note": "play/pause and speed are not saved preferences; no restart clock reset"}


def recall_matches(loaded, recalled, slot):
    prefix = f"views.saved.{slot}."
    if (loaded.get(prefix + "filled") != "true"
            or recalled.get("player.mode") != loaded.get(prefix + "mode")):
        return False
    try:
        for live, saved in (("camera.look_yaw_rad", "yaw_rad"), ("camera.look_pitch_rad", "pitch_rad")):
            a, b = float(recalled[live]), float(loaded[prefix + saved])
            if not math.isfinite(a) or not math.isfinite(b) or abs(a - b) > 1e-12:
                return False
        if recalled["player.mode"] == "Free":
            # Free look is a full basis (including roll); yaw/pitch alone are
            # not the actual look. The observed camera boundary narrows to f32.
            for field in ("forward", "up"):
                a = [float(v) for v in recalled[f"camera.{field}"].split(",")]
                b = [float(v) for v in loaded[prefix + field].split(",")]
                if (len(a) != 3 or len(b) != 3 or not all(math.isfinite(v) for v in a + b)
                        or math.dist(a, b) > 1e-6):
                    return False
    except (KeyError, ValueError):
        return False
    return True


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
    if read_profile_trace(trace) != "Restart":
        raise ValueError("trace does not witness an unprepared profile restart")
    old_trace = before / "app/scenario.tsv"
    read_profile_trace(old_trace)
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
    live_differences, unexpected_live, clock = loaded_context(previous, old, loaded)
    result = {"schema": "input-persistence-comparison-v1", "complete": not differences and not unexpected_live,
              "before": str(before), "restart": str(restart),
              "saved_fields_compared": len(previous), "saved_field_differences": differences,
              "live_player_differences": live_differences, "clock_restart": clock,
              "live_player_preservation_required": "player" in previous,
              "unexpected_live_player_differences": unexpected_live,
              "saved_file_matches_quit_state": True, "loaded_before_actions_without_preparation": True,
              "slot_recalls": [], "nonserialized_view_differences": [],
              "limitations": ["current camera mode/pose is not automatically restored at startup",
                              "without a saved player record the next launch uses the production fallback, not an unsaved live pose",
                              "preparation is not travel; moon-relative production saving is not implemented here"]}
    # These live-only fields are not serialized. Retain differences rather than
    # calling them persisted or silently changing the production file format.
    for slot in range(1, 9):
        key = f"views.saved.{slot}.anchored"
        if old.get(key) != loaded.get(key):
            result["nonserialized_view_differences"].append({"field": key, "before": old.get(key), "loaded": loaded.get(key)})
        if recalls:
            recalled = observation(trace, f"recalled-{slot}")
            mode = recalled.get("player.mode")
            fields_match = recall_matches(loaded, recalled, slot)
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
