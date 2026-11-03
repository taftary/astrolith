"""Independent known-value checks for route report accounting."""

from pathlib import Path
import tempfile
import unittest

from profile_report import classify, gpu_passes, improvement, resources, summarize, wait_mode


class ReportTests(unittest.TestCase):
    def test_wait_mode_labels_fast_slow_and_mixed_runs(self):
        self.assertEqual(wait_mode([0.2] * 95 + [33] * 5)["label"], "fast")
        self.assertEqual(wait_mode([33] * 95 + [0.2] * 5)["label"], "slow")
        self.assertEqual(wait_mode([33] * 50 + [0.2] * 50)["label"], "mixed")
        self.assertEqual(wait_mode([8.33, 8.34])["long_wait_share"], 0.5)
        self.assertIsNone(wait_mode([])["label"])

    def test_gpu_passes_reads_means_and_unsupported(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder, "gpu.txt")
            path.write_text("GPU-PASS scenario=ground samples=3 mean=12.360 "
                            "path=ground render/main_opaque_pass_3d/elapsed_gpu\n")
            passes = gpu_passes(path)
            self.assertTrue(passes["supported"])
            self.assertEqual(passes["scenarios"]["ground"]
                             ["ground render/main_opaque_pass_3d/elapsed_gpu"]["mean"], 12.36)
            path.write_text("GPU-UNSUPPORTED reason=no-elapsed-gpu-measurements\n")
            self.assertFalse(gpu_passes(path)["supported"])
            path.write_text("GPU-PASS scenario=ground samples=1 mean=1\n")
            with self.assertRaises(ValueError):
                gpu_passes(path)

    def test_improvement_needs_five_percent_and_twice_the_noise(self):
        steady = [40.0, 40.2, 39.8]
        self.assertEqual(improvement(steady, [37.0, 37.1, 36.9])["verdict"], "better")
        self.assertEqual(improvement(steady, [39.5, 39.4, 39.6])["verdict"], "ambiguous")
        noisy = [40.0, 44.0, 36.0]
        self.assertEqual(improvement(noisy, [37.9, 38.0, 38.1])["verdict"], "ambiguous")
        self.assertEqual(improvement(steady, [41.0, 41.0, 41.0])["verdict"], "not-better")

    def test_classify_promising_rejected_and_ambiguity_extension(self):
        def scene(p95_proto, mean_proto=26.0, ready=(1000.0, 1050.0)):
            return {"p95": ([40.0, 40.2, 39.8], p95_proto),
                    "mean": ([26.0, 26.1, 25.9], [mean_proto] * 3),
                    "readiness_p95": ([ready[0]] * 3, [ready[1]] * 3),
                    "memory_peak": ([100.0] * 3, [104.0] * 3)}
        good = classify({"ground": scene([37.0] * 3)}, pairs=3)
        self.assertEqual(good["verdict"], "promising")
        self.assertTrue(good["rollout_numbers_ok"])
        slower_load = classify({"ground": scene([37.0] * 3, ready=(1000.0, 1200.0))}, pairs=3)
        self.assertEqual(slower_load["verdict"], "promising")
        self.assertFalse(slower_load["rollout_numbers_ok"])
        self.assertEqual(classify({"ground": scene([41.0] * 3)}, pairs=3)["verdict"], "rejected")
        self.assertEqual(classify({"ground": scene([39.5] * 3)}, pairs=3)["verdict"],
                         "extend-to-five-pairs")
        self.assertEqual(classify({"ground": scene([39.5] * 3)}, pairs=5)["verdict"],
                         "inconclusive")
        # Mean may rise by the larger of 5% (1.3 ms of 26 ms) or 0.5 ms.
        self.assertTrue(classify({"ground": scene([37.0] * 3, mean_proto=27.2)}, pairs=3)
                        ["rollout_numbers"]["ground"]["mean"])
        self.assertFalse(classify({"ground": scene([37.0] * 3, mean_proto=27.4)}, pairs=3)
                         ["rollout_numbers"]["ground"]["mean"])

    def test_resource_observations_report_growth_and_reject_missing_required_data(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder, "telemetry.csv")
            header = ("schema,elapsed_s,leg,meshes,images,standard_materials,ground_materials,"
                      "terrain_bytes,terrain_cap_bytes,render_scale,output_width,output_height,"
                      "target_width,target_height,frame_work_available,ring_ready,detail_ready\n")
            first = "1,0,0,2,3,1,1,100,1000,1,1600,900,1600,900,true,false,false\n"
            last = "1,2,1,2,4,1,2,200,1000,0.8,1600,900,1280,720,true,true,true\n"
            path.write_text(header + first + last)
            report = resources(path)
            self.assertEqual(report["observations"]["images"]["net_change"], 1)
            self.assertEqual(len(report["sampling_gaps"]), 1)
            self.assertEqual(len(report["last_settled_sample_per_leg"]), 1)
            path.write_text(header + first.replace(",true,", ",false,"))
            with self.assertRaises(ValueError):
                resources(path)

    def test_warmup_stalls_empty_readiness_and_percentiles(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            root.joinpath("frames.txt").write_text(
                "0 warmup 999 10 8 10 3\n"
                "1 ground 10 5 3 5 2\n"
                "2 ground 20 6 6 4 3\n"
                "3 orbit 120 7 7 5 4\n")
            root.joinpath("route.txt").write_text(
                "ROUTE-LEG name=a scenario=ground ring_ms=150 ready_ms=none\n")
            result = summarize(root)
            self.assertEqual(result["all"]["interval_ms"],
                             {"count": 3, "mean": 50, "p95": 120, "worst": 120})
            self.assertEqual(result["all"]["over_100ms"], 1)
            self.assertEqual(result["all"]["interval_coverage_s"], .15)
            self.assertEqual(result["incomplete_detail_legs"], 1)
            self.assertEqual([b["scenario"] for b in result["blocks"]],
                             ["warmup", "ground", "orbit"])
            self.assertIsNone(result["readiness"][0]["ready_ms"])

    def test_invalid_or_missing_measurements_cannot_produce_summary(self):
        for frames in ("", "0 a nan 1 1 1 1\n", "1 a 1 1 1 1 1\n0 a 1 1 1 1 1\n"):
            with tempfile.TemporaryDirectory() as folder:
                Path(folder, "frames.txt").write_text(frames)
                Path(folder, "route.txt").write_text("")
                with self.assertRaises(ValueError):
                    summarize(folder)


if __name__ == "__main__":
    unittest.main()
