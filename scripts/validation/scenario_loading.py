"""Strict streaming loading/resource evidence checks, not gameplay certification.

Only retained production records are summarized. Missing rows, loss, invalid
lifecycles and incomplete tails cannot satisfy the public runner's completion.
"""

from collections import Counter
import math
from pathlib import Path
import re


MAX_BYTES = 256 * 1024 * 1024
MAX_LINE = 16 * 1024
LIMIT = 8192
COMMON = "version sequence elapsed_ns observed_main_frame root_seed open_seed epoch host_path surface_owner kind".split()
FIELDS = {
    "epoch-start": "",
    "epoch-abandoned": "pending_discarded completed_discarded",
    "job-queued": "job patch full synchronous",
    "job-started": "job patch",
    "job-finished": "job patch succeeded mesh_logical_cpu_bytes epoch_active_at_finish",
    "job-cancelled": "job patch",
    "result-collected": "patch succeeded",
    "apply-unit": "patch cpu_replacement upload_returned_gpu unit_ms budget_ms single_unit_overrun",
    "apply-batch": "units new_patch_upload_attempts elapsed_ms budget_ms work_share includes_synchronous_roots",
    "proxy-apply-unit": "unit_ms budget_ms single_unit_overrun",
    "ground-cache": "entries info_accounted_bytes cpu_accounted_bytes tile_accounted_bytes total_accounted_bytes total_gpu_allocation",
    "inbox-insert": "patch replaced entries logical_texel_bytes count_scope",
    "inbox-take": "patch accepted actual_relief_seed actual_radius_bits entries logical_texel_bytes count_scope",
    "inbox-clear": "discarded_entries discarded_logical_texel_bytes count_scope",
    "height-cache": "entries entry_cap logical_grid_bytes count_scope external_readers_included",
    "queues": "stage pending completed wanted build_candidates missing_wanted_at_dispatch ground_cap_bytes",
    "resource-created": "resource asset role moon_marker",
    "main-remove-request": "resource removed_from_store render_retired physical_gpu_release",
    "main-ownership": "resource in_asset_store entity_refs known_owner_refs owner_scope all_strong_handles_counted",
    "render-resource": "resource extracted_main_frame in_render_store previously_resident stage physical_gpu_release",
    "render-mesh-descriptor": "resource extracted_main_frame vertices vertex_stride_bytes logical_gpu_vertex_bytes logical_gpu_index_bytes measurement slab_padding_included total_gpu_allocation",
    "render-atlas-descriptor": "resource extracted_main_frame width height layers mip_levels samples format base_level_logical_bytes total_gpu_allocation",
    "render-frame": "extracted_main_frame tracked_resident active_private_resident outgoing_resident shared_resident current_atlas render_meshes render_images render_materials checkpoint total_gpu_allocation actual_draw_calls gpu_execution_ms",
    "resource-scene": "main_meshes main_images main_standard_materials main_ground_materials mesh_entities logical_submission_groups group_key active_proxy_entities outgoing_proxy_entities proxy_vertices proxy_logical_vertex_bytes proxy_logical_index_bytes proxy_bytes_scope",
    "atlas-state": "asset pages tile_entries free_tiles next_tile main_pending_writes main_pending_logical_bytes",
    "render-writes-extract": "extracted_main_frame asset previous_asset pending_before added pending_logical_bytes image_changed",
    "render-writes-submit": "extracted_main_frame asset writes logical_bytes queue_write_cpu_ns gpu_completion",
}
ROLES = {"shared-atlas", "shared-grid", "shared-ground-material", "moon-proxy", "moon-material", "parent-proxy", "parent-material"}
PROVENANCE = "root_seed open_seed host_path surface_owner".split()


def fields(line):
    parts = line.rstrip("\n").split("\t")
    values = {}
    for text in parts[1:]:
        key, sep, value = text.partition("=")
        if not sep or not re.fullmatch(r"[a-z_]+", key) or key in values or not value:
            raise ValueError("malformed/duplicate loading field")
        values[key] = value
    if values.get("version") != "1":
        raise ValueError("unsupported loading record version")
    return parts[0], values


def integer(row, key):
    value = row[key]
    if not re.fullmatch(r"[0-9]+", value):
        raise ValueError(f"loading {key} is not a nonnegative integer")
    return int(value)


def boolean(row, key):
    if row[key] not in ("true", "false"):
        raise ValueError(f"loading {key} is not boolean")
    return row[key] == "true"


def finite(row, key):
    value = float(row[key])
    if not math.isfinite(value) or value < 0:
        raise ValueError(f"loading {key} is not finite/nonnegative")
    return value


class LoadingEvidence:
    """Bounded state: current jobs and at most 8192 scalar asset lifetimes."""

    def __init__(self):
        self.sequence = self.time = self.main_frame = self.render_frame = 0
        self.last_job = -1
        self.jobs, self.completed, self.resources = {}, {}, {}
        self.epochs = {}
        self.active_epoch = None
        self.counts, self.high = Counter(), Counter()
        self.running = self.resident = 0
        self.highest_render_stamp = 0
        self.stale_finishes = self.stale_tiles = self.retired = self.overruns = 0

    def peak(self, key, value):
        self.high[key] = max(self.high[key], value)

    def event(self, row):
        kind = row["kind"]
        if kind not in FIELDS or any(k not in row for k in COMMON + FIELDS[kind].split()):
            raise ValueError(f"unknown/incomplete loading event: {kind}")
        sequence, time = integer(row, "sequence"), integer(row, "elapsed_ns")
        if sequence != self.sequence or time < self.time:
            raise ValueError("missing/reordered loading sequence or monotonic time")
        self.sequence += 1
        self.time = time
        frame = integer(row, "observed_main_frame")
        integer(row, "root_seed")
        integer(row, "open_seed")
        if not (row["host_path"] == "unavailable-not-open-planet" or re.fullmatch(r"[0-9]+(?:\.[0-9]+){8}", row["host_path"])):
            raise ValueError("invalid loading host identity")
        self.counts[kind] += 1
        epoch = row["epoch"]
        if not (row["surface_owner"] in ("not-a-surface-epoch", "planet") or re.fullmatch(r"moon:[0-9]+", row["surface_owner"])):
            raise ValueError("invalid production surface owner")
        if kind == "epoch-start":
            if epoch in self.epochs or not epoch.isascii() or not epoch.isdecimal() or len(self.epochs) == LIMIT:
                raise ValueError("invalid/duplicate/excessive loading epochs")
            self.epochs[epoch] = tuple(row[key] for key in PROVENANCE)
            self.active_epoch = epoch
        elif epoch != "not-a-patch-epoch" and self.epochs.get(epoch) != tuple(row[key] for key in PROVENANCE):
            raise ValueError("event changed or lacks its original epoch provenance")
        if kind.startswith("job-"):
            self.job(kind, row)
        elif kind == "result-collected":
            matching = [(n, j) for n, j in self.jobs.items() if j["epoch"] == epoch and j["patch"] == row["patch"] and not j["sync"]]
            if len(matching) != 1 or matching[0][1]["state"] != "finished":
                raise ValueError("collected result lacks its real finished job")
            n, job = matching[0]
            if job["success"] != boolean(row, "succeeded"):
                raise ValueError("collected result changed build outcome")
            if job["success"]:
                self.completed[(epoch, row["patch"])] = True
            del self.jobs[n]
        elif kind == "apply-unit":
            if not self.completed.pop((epoch, row["patch"]), False):
                raise ValueError("applied patch has no collected completion")
            overrun = finite(row, "unit_ms") > finite(row, "budget_ms")
            if overrun != boolean(row, "single_unit_overrun"):
                raise ValueError("apply overrun differs from retained unit time/budget")
            self.overruns += int(overrun)
            boolean(row, "cpu_replacement")
            boolean(row, "upload_returned_gpu")
        elif kind == "proxy-apply-unit":
            overrun = finite(row, "unit_ms") > finite(row, "budget_ms")
            if overrun != boolean(row, "single_unit_overrun"):
                raise ValueError("proxy apply overrun differs from retained unit time/budget")
            self.overruns += int(overrun)
        elif kind == "apply-batch":
            for key in ("units", "new_patch_upload_attempts"):
                integer(row, key)
            for key in ("elapsed_ms", "budget_ms", "work_share"):
                finite(row, key)
            if boolean(row, "includes_synchronous_roots"):
                raise ValueError("synchronous root work cannot be called budgeted applies")
        elif kind == "epoch-abandoned":
            pending = [j for j in self.jobs.values() if j["epoch"] == epoch and not j["sync"]]
            completed = [key for key in self.completed if key[0] == epoch]
            if integer(row, "pending_discarded") != len(pending) or integer(row, "completed_discarded") != len(completed):
                raise ValueError("abandoned ownership differs from queued/completed records")
            self.active_epoch = None
            for n, job in list(self.jobs.items()):
                if job["epoch"] == epoch and job["state"] == "finished":
                    del self.jobs[n]
            for key in completed:
                del self.completed[key]
        elif kind == "queues":
            pending = sum(j["epoch"] == epoch and not j["sync"] for j in self.jobs.values())
            completed = sum(key[0] == epoch for key in self.completed)
            if integer(row, "pending") != pending or integer(row, "completed") != completed:
                raise ValueError("queue snapshot differs from retained job/apply transitions")
            if row["stage"] not in ("collected", "dispatched", "applied"):
                raise ValueError("unknown queue checkpoint")
            if row["stage"] != "collected":
                integer(row, "missing_wanted_at_dispatch")
        elif kind == "ground-cache":
            parts = sum(integer(row, k) for k in ("info_accounted_bytes", "cpu_accounted_bytes", "tile_accounted_bytes"))
            if parts != integer(row, "total_accounted_bytes") or boolean(row, "total_gpu_allocation"):
                raise ValueError("ground cap parts are inconsistent or overclaim device allocation")
        elif kind == "inbox-take":
            self.stale_tiles += int(not boolean(row, "accepted"))
        elif kind.startswith("resource-") or kind.startswith("main-") or kind.startswith("render-"):
            self.resource(kind, row, frame)
        # Generic per-event peaks retain short-lived queue/cache/atlas counters.
        for key, value in row.items():
            if key.endswith(("_bytes", "_entries", "_writes", "_resident", "_proxies")) or key in ("pending", "completed", "entries", "pages", "mesh_entities", "logical_submission_groups", "proxy_vertices"):
                if re.fullmatch(r"[0-9]+", value):
                    self.peak(f"{kind}.{key}", int(value))

    def job(self, kind, row):
        n = integer(row, "job")
        if kind == "job-queued":
            if n != self.last_job + 1 or len(self.jobs) == LIMIT or row["epoch"] != self.active_epoch:
                raise ValueError("missing/duplicate/excessive job or dispatch outside active epoch")
            self.last_job = n
            self.jobs[n] = {"epoch": row["epoch"], "patch": row["patch"], "sync": boolean(row, "synchronous"), "state": "queued"}
            boolean(row, "full")
            return
        job = self.jobs.get(n)
        if job is None or (job["epoch"], job["patch"]) != (row["epoch"], row["patch"]):
            raise ValueError("job event lacks its original queued identity")
        if kind == "job-started":
            if job["state"] != "queued":
                raise ValueError("job started more than once")
            job["state"] = "started"
            self.running += 1
            self.peak("actual_running_jobs", self.running)
        elif kind == "job-finished":
            if job["state"] != "started":
                raise ValueError("finished job was not started")
            self.running -= 1
            job["success"] = boolean(row, "succeeded")
            if job["success"]:
                integer(row, "mesh_logical_cpu_bytes")
            elif row["mesh_logical_cpu_bytes"] != "unavailable":
                raise ValueError("failed build has invented CPU bytes")
            active = job["epoch"] == self.active_epoch
            if boolean(row, "epoch_active_at_finish") != active:
                raise ValueError("finish epoch differs from retained epoch retirement")
            self.stale_finishes += int(not active)
            job["state"] = "finished"
            if job["sync"] or not active:
                del self.jobs[n]
        else:
            if job["state"] == "finished":
                raise ValueError("completed job mislabeled cancelled")
            self.running -= int(job["state"] == "started")
            del self.jobs[n]

    def resource(self, kind, row, frame):
        if "extracted_main_frame" in row:
            self.highest_render_stamp = max(self.highest_render_stamp, integer(row, "extracted_main_frame"))
        if kind == "resource-created":
            n = integer(row, "resource")
            if n != len(self.resources) or n == LIMIT or row["role"] not in ROLES:
                raise ValueError("invalid/duplicate/excessive asset lifetime")
            value = {"role": row["role"], "host": row["host_path"], "asset": row["asset"], "main": False, "resident": False, "ever": False, "gpu_bytes": 0}
            value["provenance"] = tuple(row[key] for key in PROVENANCE + ["epoch"])
            value["gpu_ready"] = value["render_seen"] = False
            if row["role"] == "moon-proxy":
                integer(row, "moon_marker")
            elif row["moon_marker"] != "not-a-moon-proxy":
                raise ValueError("non-proxy resource has an invented moon marker")
            if row["role"] in ("moon-proxy", "parent-proxy", "shared-grid"):
                value["vertices"] = integer(row, "vertices")
                value["vertex_bytes"] = integer(row, "logical_vertex_bytes")
                value["index_bytes"] = integer(row, "logical_index_bytes")
            elif row["role"] == "shared-atlas":
                size = integer(row, "width") * integer(row, "height") * integer(row, "layers") * 4
                if row["format"] != "R32Uint" or size != integer(row, "logical_texture_bytes") or integer(row, "mip_levels") != 1 or integer(row, "samples") != 1:
                    raise ValueError("atlas descriptor/size mismatch")
                value["atlas_bytes"] = size
            self.resources[n] = value
            return
        if "resource" in row:
            value = self.resources.get(integer(row, "resource"))
            if value is None:
                raise ValueError("resource stage lacks its creation descriptor")
            if value["provenance"] != tuple(row[key] for key in PROVENANCE + ["epoch"]):
                raise ValueError("resource stage changed its original creation provenance")
            if kind == "main-ownership":
                value["main"] = boolean(row, "in_asset_store")
                value["refs"] = integer(row, "entity_refs")
                integer(row, "known_owner_refs")
                if boolean(row, "all_strong_handles_counted"):
                    raise ValueError("known owners cannot claim every strong handle")
            elif kind == "main-remove-request":
                boolean(row, "removed_from_store")
                if boolean(row, "render_retired"):
                    raise ValueError("main removal was substituted for render retirement")
                if row["physical_gpu_release"] != "unavailable-driver-lifetime":
                    raise ValueError("main removal cannot claim physical GPU release")
            elif kind == "render-resource":
                present = boolean(row, "in_render_store")
                if boolean(row, "previously_resident") != value["ever"]:
                    raise ValueError("render retirement lacks prior observed residence")
                stage = "resident" if present else "absent-after-residence" if value["ever"] else "not-yet-observed-resident"
                if row["stage"] != stage or row["physical_gpu_release"] != "unavailable-driver-lifetime":
                    raise ValueError("render stage or physical GPU release claim differs from observed residence")
                if value["render_seen"] and present == value["resident"]:
                    raise ValueError("duplicate unchanged render-resource transition")
                if not present and value["ever"]:
                    self.retired += 1
                self.resident += int(present) - int(value["resident"])
                value["resident"] = present
                value["ever"] |= present
                value["render_seen"] = True
                value["gpu_ready"] = value["role"] in ("moon-material", "parent-material", "shared-ground-material")
            elif kind == "render-mesh-descriptor":
                if not value["resident"] or integer(row, "vertices") != value["vertices"]:
                    raise ValueError("render mesh differs from its observed source/residence")
                size = integer(row, "vertices") * integer(row, "vertex_stride_bytes")
                if size != integer(row, "logical_gpu_vertex_bytes"):
                    raise ValueError("prepared vertex size is not recomputable")
                value["gpu_bytes"] = size + integer(row, "logical_gpu_index_bytes")
                value["gpu_ready"] = True
                if row["measurement"] != "prepared-render-layout" or boolean(row, "slab_padding_included") or boolean(row, "total_gpu_allocation"):
                    raise ValueError("render mesh descriptor overclaims allocation scope")
            elif kind == "render-atlas-descriptor":
                size = integer(row, "width") * integer(row, "height") * integer(row, "layers") * 4
                if not value["resident"] or row["format"] != "R32Uint" or size != value["atlas_bytes"] or size != integer(row, "base_level_logical_bytes"):
                    raise ValueError("render atlas differs from source descriptor/residence")
                value["gpu_bytes"] = size
                value["gpu_ready"] = True
                if integer(row, "mip_levels") != 1 or integer(row, "samples") != 1 or boolean(row, "total_gpu_allocation"):
                    raise ValueError("render atlas descriptor overclaims allocation scope")
        elif kind == "render-frame":
            observed = integer(row, "extracted_main_frame")
            if observed != self.render_frame + 1 or integer(row, "tracked_resident") != self.resident:
                raise ValueError("missing render checkpoint or inconsistent resident count")
            resident = [v for v in self.resources.values() if v["resident"]]
            if any(not value["gpu_ready"] for value in resident):
                raise ValueError("resident resource lacks its required prepared descriptor")
            if row["checkpoint"] != "after-prepare-assets" or row["total_gpu_allocation"] != "unavailable" or row["actual_draw_calls"] != "unavailable" or row["gpu_execution_ms"] != "unavailable-not-enabled":
                raise ValueError("render checkpoint overclaims unavailable GPU measurements")
            private = [v for v in resident if v["role"] in ("moon-proxy", "moon-material", "parent-proxy", "parent-material")]
            active = sum(v["host"] == row["host_path"] and v["host"] != "unavailable-not-open-planet" for v in private)
            outgoing = len(private) - active + sum(v["role"] == "shared-atlas" and v["asset"] != row["current_atlas"] for v in resident)
            if (integer(row, "active_private_resident"), integer(row, "outgoing_resident"), integer(row, "shared_resident")) != (active, outgoing, len(resident) - active - outgoing):
                raise ValueError("active/outgoing/shared residence differs from original owners/current atlas")
            self.render_frame = observed
            self.peak("tracked_resident_logical_gpu_bytes", sum(v["gpu_bytes"] for v in self.resources.values() if v["resident"]))
        elif kind == "resource-scene":
            if frame != self.main_frame + 1:
                raise ValueError("missing main-world resource checkpoint")
            self.main_frame = frame
            proxies = [v for v in self.resources.values() if v["role"] in ("moon-proxy", "parent-proxy") and v.get("refs", 0)]
            for field, prop in (("proxy_vertices", "vertices"), ("proxy_logical_vertex_bytes", "vertex_bytes"), ("proxy_logical_index_bytes", "index_bytes")):
                if integer(row, field) != sum(v[prop] for v in proxies):
                    raise ValueError("proxy scene total differs from real descriptors/ownership")
            active = sum(v["refs"] for v in proxies if v["host"] == row["host_path"])
            outgoing = sum(v["refs"] for v in proxies) - active
            if active != integer(row, "active_proxy_entities") or outgoing != integer(row, "outgoing_proxy_entities"):
                raise ValueError("proxy active/outgoing owners differ from original host identities")

    def finish(self, end):
        if not boolean(end, "complete") or integer(end, "records") != self.sequence or integer(end, "elapsed_ns") < self.time:
            raise ValueError("incomplete loading result or missing terminal records")
        if end.get("all_jobs_or_retirements_finished") != "not-claimed":
            raise ValueError("loading seal cannot claim idle or fully retired shutdown")
        if integer(end, "main_frame") != self.main_frame or not self.main_frame or not self.render_frame:
            raise ValueError("required main/render checkpoints are missing")
        if self.highest_render_stamp != self.render_frame:
            raise ValueError("missing final render checkpoint for retained render events")
        if not 0 <= self.main_frame - self.render_frame <= 2:
            raise ValueError("render frontier has an unexplained frame gap")
        return {"schema": "input-loading-summary-v1", "complete": True, "events": self.sequence,
                "main_frames": self.main_frame, "rendered_main_frames": self.render_frame,
                "unrendered_exit_tail_frames": self.main_frame - self.render_frame,
                "event_counts": dict(self.counts), "high_water": dict(self.high),
                "single_apply_overruns": self.overruns, "stale_epoch_finishes": self.stale_finishes,
                "stale_tiles_refused": self.stale_tiles, "observed_render_retirements": self.retired,
                "distinct_resource_lifetimes": len(self.resources), "unfinished_jobs_at_seal": len(self.jobs),
                "collected_unapplied_at_seal": len(self.completed),
                "limitations": ["logical descriptors/accounting, not total GPU/process memory",
                                "render store absence is not proof of driver/device allocation release",
                                "seal is not an idle/fully-retired shutdown claim",
                                "logical mesh/material groups are not actual draw calls",
                                "input timing and full-session working-set integration are separate"]}


def summarize(path):
    path = Path(path)
    if not path.is_file() or path.stat().st_size > MAX_BYTES:
        raise ValueError("loading sidecar missing or exceeds 256 MiB")
    evidence = LoadingEvidence()
    header = end = None
    with path.open("r", encoding="utf-8", newline="") as source:
        while text := source.readline(MAX_LINE + 1):
            if len(text) > MAX_LINE or not text.endswith("\n") or "\r" in text:
                raise ValueError("loading line exceeds bound or is truncated/not canonical LF")
            tag, row = fields(text)
            if end is not None:
                raise ValueError("loading records follow terminal boundary")
            if header is None:
                if tag != "LOAD-RUN" or row.get("scope") != "app-session-through-seal":
                    raise ValueError("missing loading header/scope")
                if any(key not in row for key in ("event_limit", "resource_limit", "output_limit_bytes")) or row.get("shutdown_idle_or_gpu_release_claim") != "false":
                    raise ValueError("incomplete loading header or invented shutdown/GPU release claim")
                if integer(row, "event_limit") != LIMIT or integer(row, "resource_limit") != LIMIT or integer(row, "output_limit_bytes") != MAX_BYTES:
                    raise ValueError("unexpected loading bounds")
                header = row
            elif tag == "LOAD-EVENT":
                try:
                    evidence.event(row)
                except (KeyError, TypeError, OverflowError) as exc:
                    raise ValueError(f"missing/invalid loading field: {exc}") from exc
            elif tag == "LOAD-END":
                end = row
            else:
                raise ValueError("loading loss, unknown tag or duplicate header")
    if end is None:
        raise ValueError("loading sidecar has no terminal boundary")
    try:
        return evidence.finish(end)
    except (KeyError, TypeError, OverflowError) as exc:
        raise ValueError(f"missing/invalid loading boundary: {exc}") from exc
