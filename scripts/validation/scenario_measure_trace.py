"""Bounded input trace/phase identities for measurement, not inferred travel.

This consumes the actual trace and selected observation fields. It does not
interpret requested Expect values as observations or preparation as input.
"""

from collections import Counter
from pathlib import Path
import re


MAX_BYTES = 256 * 1024 * 1024
MAX_FRAMES = 65_536
MAX_LINKS = 16_384
STATE_FIELDS = """seed camera.path camera.position camera.position.frame camera.forward camera.up
player.path player.mode player.active player.flying player.on_ground player.ground_pending
player.position player.position.frame player.velocity_m_per_s player.heading_rad
support.identity support.identity.reason support.kind support.contact ground.identity
ground.clearance_m player.surface_clearance_m player.surface_clearance_m.reason
fixture.kind fixture.selected.planet fixture.selected.system fixture.selected.moon
fixture.moon.type fixture.moon.radius_km fixture.direction_basis clock.game_s clock.playing
ready screen settings.saved.quality_mode settings.saved.planet_detail
settings.saved.ground_memory_limit settings.saved.target_fps settings.saved.vsync
settings.effective.terrain settings.effective.markers settings.effective.atmosphere
settings.effective.shadows settings.effective.bloom settings.effective.render_scale
settings.effective.terrain_work""".split()
LINKED = {"PHASE", "PLACEMENT", "PREPARE", "ACTION", "OBSERVE", "QUIT-CONTROL",
          "NORMAL-QUIT", "SCENARIO-OK", "SCENARIO-FAIL"}


def lines(path, limit=16_384):
    path = Path(path)
    if not path.is_file() or path.stat().st_size > MAX_BYTES:
        raise ValueError(f"required evidence missing or exceeds 256 MiB: {path.name}")
    with path.open(encoding="utf-8", newline="") as source:
        while line := source.readline(limit + 1):
            if len(line) > limit or not line.endswith("\n") or "\r" in line:
                raise ValueError(f"truncated/noncanonical/oversized evidence line: {path.name}")
            yield line[:-1]


def fields(line):
    parts = line.split("\t")
    row = {}
    for field in parts[1:]:
        key, sep, value = field.partition("=")
        if not sep or not re.fullmatch(r"[a-z][a-z0-9_.]*", key) or key in row or not value:
            raise ValueError("malformed/duplicate input measurement field")
        row[key] = value
    if row.get("version") != "1":
        raise ValueError("unsupported input measurement version")
    return parts[0], row


def integer(row, key):
    value = row.get(key, "")
    if not re.fullmatch(r"[0-9]+", value):
        raise ValueError(f"input measurement {key} is not a nonnegative integer")
    return int(value)


def flag(row, key):
    if row.get(key) not in ("true", "false"):
        raise ValueError(f"input measurement {key} is not boolean")
    return row[key] == "true"


class InputTrace:
    def __init__(self, path):
        self.links, self.observations, self.phases = [], [], []
        self.by_snapshot, self.actions = {}, Counter()
        self.current_phase = self.current_observation = None
        self.header = self.terminal = None
        pending = None
        for line in lines(path, limit=2 * 1024 * 1024 + 1024):
            kind = line.split("\t", 1)[0].split(" ", 1)[0]
            if kind == "MEASURE-LINK":
                if pending is not None:
                    raise ValueError("input measurement link has no adjacent trace record")
                _, pending = fields(line)
                if integer(pending, "trace_id") != len(self.links) or len(self.links) >= MAX_LINKS:
                    raise ValueError("missing/duplicate/excessive input trace links")
                time, frame = integer(pending, "elapsed_ns"), integer(pending, "main_frame")
                if not frame or frame > MAX_FRAMES or (self.links and time < self.links[-1]["elapsed_ns"]):
                    raise ValueError("invalid input trace frame/time")
                pending = {**pending, "elapsed_ns": time, "main_frame": frame}
            elif kind in LINKED:
                if pending is None or pending["kind"] != kind or self.terminal is not None:
                    raise ValueError("required trace record lacks its clock/frame link or follows terminal")
                link = {**pending, "row": line}
                self.links.append(link)
                self.record(kind, line, link)
                pending = None
            elif kind == "VALUE":
                if pending is not None or self.current_observation is None:
                    raise ValueError("observation value lacks its actual observation record")
                parts = line.split("\t")
                if len(parts) != 4 or parts[1] != self.current_observation["name"]:
                    raise ValueError("observation values changed their named snapshot")
                key, value = parts[2:]
                if key in STATE_FIELDS:
                    if key in self.current_observation["values"]:
                        raise ValueError("duplicate measured observation value")
                    self.current_observation["values"][key] = value
            elif kind == "SCENARIO-RUN":
                if self.header is not None or self.links or not line.startswith("SCENARIO-RUN\tversion=1\t"):
                    raise ValueError("invalid input measurement scenario header")
                self.header = line
            elif kind != "PERSISTENCE":
                raise ValueError(f"unknown input measurement trace row: {kind}")
        if pending is not None or self.header is None or self.terminal != "SCENARIO-OK" or not self.phases:
            raise ValueError("incomplete measured input trace")
        if any(phase.get("actions_ns") is None for phase in self.phases):
            raise ValueError("incomplete measured preparation/setup phase")

    def record(self, kind, line, link):
        self.current_observation = None
        parts = line.split("\t")
        if kind == "PHASE":
            values = dict(p.split("=", 1) for p in parts[2:])
            if parts[1] in ("setup", "preparation"):
                if values.get("counts_as_travel") != "false" or values.get("include_in_resource_peaks") != "true":
                    raise ValueError("preparation was mislabeled as travel or excluded from peaks")
                if self.current_phase is not None and self.current_phase.get("actions_ns") is None:
                    raise ValueError("overlapping preparation phases")
                phase = {"name": values.get("name", "setup"), "start_ns": link["elapsed_ns"],
                         "start_frame": link["main_frame"], "kind": parts[1], "counts_as_travel": False,
                         "include_in_resource_peaks": True}
                self.phases.append(phase)
                self.current_phase = phase
            elif parts[1] == "actions":
                if self.current_phase is None or "actions_ns" in self.current_phase:
                    raise ValueError("actions have no unfinished setup/preparation")
                if self.current_phase["kind"] == "preparation" and values.get("after_preparation") != self.current_phase["name"]:
                    raise ValueError("actions changed the preparation identity")
                self.current_phase["actions_ns"] = link["elapsed_ns"]
                self.current_phase["actions_frame"] = link["main_frame"]
            else:
                raise ValueError("unknown measured scenario phase")
        elif kind == "PLACEMENT":
            if self.current_phase is None or self.current_phase["kind"] != "preparation" or parts[1] != self.current_phase["name"] or "placement_ns" in self.current_phase:
                raise ValueError("placement lacks its explicit unique preparation")
            self.current_phase["placement_ns"] = link["elapsed_ns"]
        elif kind == "ACTION":
            self.actions[parts[2].split("(", 1)[0]] += 1
        elif kind == "OBSERVE":
            if len(self.observations) >= MAX_LINKS:
                raise ValueError("too many measured observations")
            snapshot = link.get("snapshot_frame", "")
            if not snapshot.isdecimal() or not 1 <= int(snapshot) <= link["main_frame"]:
                raise ValueError("measured observation has no physical snapshot frame")
            observation = {"name": parts[1], "snapshot_frame": int(snapshot), "values": {}}
            self.observations.append(observation)
            self.by_snapshot.setdefault(int(snapshot), []).append(observation)
            self.current_observation = observation
        elif kind in ("SCENARIO-OK", "SCENARIO-FAIL"):
            self.terminal = kind

    def validate(self, frames):
        if any(link["main_frame"] >= len(frames.begin) or link["elapsed_ns"] < frames.begin[link["main_frame"]] or link["elapsed_ns"] > frames.end_ns for link in self.links):
            raise ValueError("trace links do not belong to the measured clock/frame interval")
        if frames.checked_observations != len(self.observations):
            raise ValueError("trace observation snapshot is missing from frame-state evidence")
        for phase in self.phases:
            if phase["kind"] == "preparation" and "placement_ns" not in phase:
                raise ValueError("preparation lacks a placement witness")
        return {"linked_records": len(self.links), "observations_checked_against_actual_frame_state": len(self.observations),
                "actions": dict(self.actions), "preparations": len(self.phases) - 1,
                "continuous_input_journey": len(self.phases) == 1,
                "continuous_journey_note": "no inter-visit preparation" if len(self.phases) == 1 else "false: explicit inter-visit placements are not travel"}
