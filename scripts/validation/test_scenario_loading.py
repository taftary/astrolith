"""Hermetic parser fixtures only; these are not observed application evidence."""

from pathlib import Path
import tempfile
import unittest

from scenario_loading import LoadingEvidence, summarize


HEADER = "LOAD-RUN\tversion=1\tevent_limit=8192\tresource_limit=8192\toutput_limit_bytes=268435456\tscope=app-session-through-seal\tshutdown_idle_or_gpu_release_claim=false"


def event(kind, **values):
    return {"version": "1", "sequence": "0", "elapsed_ns": "0", "observed_main_frame": "1",
            "root_seed": "42", "open_seed": "0", "epoch": "not-a-patch-epoch",
            "host_path": "unavailable-not-open-planet", "surface_owner": "not-a-surface-epoch", "kind": kind, **{k: str(v) for k, v in values.items()}}


def scene(frame):
    return event("resource-scene", observed_main_frame=frame, main_meshes=0, main_images=1,
                 main_standard_materials=0, main_ground_materials=0, mesh_entities=0,
                 logical_submission_groups=0, group_key="mesh-material-pair-not-actual-draw-call",
                 active_proxy_entities=0, outgoing_proxy_entities=0, proxy_vertices=0,
                 proxy_logical_vertex_bytes=0, proxy_logical_index_bytes=0,
                 proxy_bytes_scope="unique-referenced-mesh-source-data")


def render(frame, count):
    return event("render-frame", observed_main_frame=frame, extracted_main_frame=frame,
                 tracked_resident=count, active_private_resident=0, outgoing_resident=0, shared_resident=count,
                 current_atlas="image:fixture-only", render_meshes=0, render_images=count, render_materials=0,
                 checkpoint="after-prepare-assets", total_gpu_allocation="unavailable",
                 actual_draw_calls="unavailable", gpu_execution_ms="unavailable-not-enabled")


def fixture(retirement=True):
    result = [event("resource-created", resource=0, asset="image:fixture-only", role="shared-atlas", moon_marker="not-a-moon-proxy",
                    width=1024, height=1024, layers=2, mip_levels=1, samples=1, format="R32Uint",
                    logical_texture_bytes=8388608, total_gpu_allocation="false"),
              scene(1),
              event("render-resource", resource=0, extracted_main_frame=1, in_render_store="true",
                    previously_resident="false", stage="resident", physical_gpu_release="unavailable-driver-lifetime"),
              event("render-atlas-descriptor", resource=0, extracted_main_frame=1, width=1024, height=1024,
                    layers=2, mip_levels=1, samples=1, format="R32Uint", base_level_logical_bytes=8388608,
                    total_gpu_allocation="false"), render(1, 1),
              event("main-remove-request", resource=0, observed_main_frame=2, removed_from_store="true",
                    render_retired="false", physical_gpu_release="unavailable-driver-lifetime"), scene(2)]
    if retirement:
        result.append(event("render-resource", resource=0, observed_main_frame=2, extracted_main_frame=2,
                            in_render_store="false", previously_resident="true", stage="absent-after-residence",
                            physical_gpu_release="unavailable-driver-lifetime"))
    result.append(render(2, 0 if retirement else 1))
    for n, row in enumerate(result):
        row.update(sequence=str(n), elapsed_ns=str(n))
    return result


def text(rows, complete="true"):
    lines = [HEADER] + ["LOAD-EVENT\t" + "\t".join(f"{k}={v}" for k, v in row.items()) for row in rows]
    lines.append(f"LOAD-END\tversion=1\tcomplete={complete}\tmain_frame=2\trecords={len(rows)}\telapsed_ns={len(rows)}\tall_jobs_or_retirements_finished=not-claimed")
    return "\n".join(lines) + "\n"


class LoadingTests(unittest.TestCase):
    def test_proxy_overrun_is_a_real_unit_not_an_invented_patch_completion(self):
        evidence = LoadingEvidence()
        evidence.event(event("proxy-apply-unit", unit_ms=2, budget_ms=1, single_unit_overrun="true"))
        self.assertEqual(evidence.overruns, 1)
        self.assertFalse(evidence.completed)
        with self.assertRaisesRegex(ValueError, "proxy apply overrun"):
            LoadingEvidence().event(event("proxy-apply-unit", unit_ms=2, budget_ms=1, single_unit_overrun="false"))

    def test_parent_proxy_is_counted_with_moons_without_an_invented_moon_marker(self):
        # Parser fixture, not observed application/GPU evidence.
        evidence = LoadingEvidence()
        host = "7.2.7.1.2.0.0.0.5"
        created = event("resource-created", host_path=host, resource=0,
                        asset="mesh:parent-fixture", role="parent-proxy",
                        moon_marker="not-a-moon-proxy", vertices=162,
                        logical_vertex_bytes=5184, logical_index_bytes=3840)
        evidence.resource("resource-created", created, 1)
        evidence.resource("main-ownership", event("main-ownership", host_path=host,
                          resource=0, in_asset_store="true", entity_refs=1,
                          known_owner_refs=0, all_strong_handles_counted="false"), 1)
        row = scene(1)
        row.update(host_path=host, active_proxy_entities="1", proxy_vertices="162",
                   proxy_logical_vertex_bytes="5184", proxy_logical_index_bytes="3840")
        evidence.resource("resource-scene", row, 1)
        row = scene(2)
        row.update(host_path=host, active_proxy_entities="1")
        with self.assertRaisesRegex(ValueError, "proxy scene total"):
            evidence.resource("resource-scene", row, 2)
        created["moon_marker"] = "1"
        with self.assertRaisesRegex(ValueError, "invented moon marker"):
            LoadingEvidence().resource("resource-created", created, 1)

    def summarize(self, value):
        with tempfile.TemporaryDirectory(prefix="astrolith-loading-parser-") as tmp:
            file = Path(tmp) / "loading.tsv"
            file.write_bytes(value.encode("utf-8"))
            return summarize(file)

    def test_logical_high_water_and_render_retirement_are_recomputable(self):
        report = self.summarize(text(fixture()))
        self.assertTrue(report["complete"])
        self.assertEqual(report["main_frames"], 2)
        self.assertEqual(report["high_water"]["tracked_resident_logical_gpu_bytes"], 8388608)
        self.assertEqual(report["observed_render_retirements"], 1)

    def test_main_removal_is_never_render_retirement(self):
        report = self.summarize(text(fixture(False)))
        self.assertEqual(report["observed_render_retirements"], 0)
        rows = fixture()
        rows[5]["render_retired"] = "true"
        with self.assertRaisesRegex(ValueError, "substituted"):
            self.summarize(text(rows))

    def test_missing_duplicate_reordered_or_incomplete_records_never_pass(self):
        rows = fixture()
        for value in (text(rows[:-1]), text(rows)[len(HEADER) + 1:], text(rows, "false"),
                      text(rows).rsplit("LOAD-END", 1)[0], text(rows) + "LOAD-LOSS\tversion=1\tdropped=1\n",
                      text(rows).replace("LOAD-END", "LOAD-LOSS"), text(rows)[:-1]):
            with self.subTest(value=value[-90:]), self.assertRaises(ValueError):
                self.summarize(value)
        for index in range(len(rows)):
            with self.subTest(missing=index), self.assertRaises(ValueError):
                self.summarize(text(rows[:index] + rows[index + 1:]))

    def test_unknown_missing_nonfinite_and_duplicate_fields_are_refused(self):
        rows = fixture()
        for field, value in (("kind", "invented"), ("width", "nan"), ("logical_texture_bytes", "0")):
            changed = [r.copy() for r in rows]
            changed[0][field] = value
            with self.assertRaises(ValueError):
                self.summarize(text(changed))
        del rows[0]["width"]
        with self.assertRaisesRegex(ValueError, "missing/invalid"):
            self.summarize(text(rows))
        with self.assertRaisesRegex(ValueError, "duplicate"):
            self.summarize(text(fixture()).replace("\twidth=1024", "\twidth=1024\twidth=1024", 1))

    def test_cancellation_and_old_epoch_completion_are_not_active_completed_work(self):
        evidence = LoadingEvidence()
        rows = [event("epoch-start", epoch=0),
                event("job-queued", epoch=0, job=0, patch="0/0/0/0", full="true", synchronous="false"),
                event("job-started", epoch=0, job=0, patch="0/0/0/0"),
                event("epoch-abandoned", epoch=0, pending_discarded=1, completed_discarded=0),
                event("epoch-start", epoch=1),
                event("job-finished", epoch=0, job=0, patch="0/0/0/0", succeeded="true", mesh_logical_cpu_bytes=120, epoch_active_at_finish="false"),
                event("job-queued", epoch=1, job=1, patch="1/0/0/0", full="false", synchronous="false"),
                event("job-cancelled", epoch=1, job=1, patch="1/0/0/0")]
        for n, row in enumerate(rows):
            row.update(sequence=str(n), elapsed_ns=str(n))
            evidence.event(row)
        self.assertEqual(evidence.stale_finishes, 1)
        self.assertEqual(evidence.running, 0)
        self.assertFalse(evidence.jobs)
        self.assertEqual(evidence.high["actual_running_jobs"], 1)

    def test_queues_cannot_copy_requested_or_invented_counts(self):
        evidence = LoadingEvidence()
        evidence.event(event("epoch-start", epoch=0))
        with self.assertRaisesRegex(ValueError, "queue snapshot"):
            evidence.event(event("queues", sequence=1, epoch=0, stage="collected", pending=1,
                                 completed=0, wanted=1, build_candidates=1,
                                 missing_wanted_at_dispatch="unavailable-before-dispatch", ground_cap_bytes=8388608))

    def test_line_and_resource_bounds_do_not_silently_truncate(self):
        with self.assertRaises(ValueError):
            self.summarize(HEADER + "\n" + "X" * 16385 + "\n")
        rows = fixture()
        rows[0]["resource"] = "8192"
        with self.assertRaises(ValueError):
            self.summarize(text(rows))

    def test_incomplete_headers_and_invented_gpu_release_claims_are_refused(self):
        original = text(fixture())
        for field in ("event_limit=8192", "resource_limit=8192", "output_limit_bytes=268435456"):
            with self.subTest(missing=field), self.assertRaises(ValueError):
                self.summarize(original.replace("\t" + field, "", 1))
        with self.assertRaises(ValueError):
            self.summarize(original.replace("shutdown_idle_or_gpu_release_claim=false", "shutdown_idle_or_gpu_release_claim=true", 1))
        with self.assertRaises(ValueError):
            self.summarize(original.replace("all_jobs_or_retirements_finished=not-claimed", "all_jobs_or_retirements_finished=true"))

    def test_old_worker_and_resource_events_cannot_adopt_a_different_host(self):
        evidence = LoadingEvidence()
        evidence.event(event("epoch-start", epoch=0))
        with self.assertRaisesRegex(ValueError, "provenance"):
            evidence.event(event("job-queued", sequence=1, epoch=0, job=0, patch="0/0/0/0",
                                 full="true", synchronous="false", host_path="7.2.7.1.2.0.0.0.5"))
        rows = fixture()
        rows[2]["host_path"] = "7.2.7.1.2.0.0.0.5"
        with self.assertRaisesRegex(ValueError, "provenance"):
            self.summarize(text(rows))

    def test_render_residence_requires_its_actual_descriptor_and_honest_stage(self):
        rows = fixture()
        without = [row.copy() for row in rows if row["kind"] != "render-atlas-descriptor"]
        for n, row in enumerate(without):
            row.update(sequence=str(n), elapsed_ns=str(n))
        with self.assertRaisesRegex(ValueError, "descriptor"):
            self.summarize(text(without))
        for field, value in (("stage", "absent-after-residence"), ("physical_gpu_release", "true")):
            changed = [row.copy() for row in rows]
            changed[2][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.summarize(text(changed))


if __name__ == "__main__":
    unittest.main()
