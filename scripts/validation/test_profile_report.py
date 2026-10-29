"""Independent known-value checks for route report accounting."""

from pathlib import Path
import tempfile
import unittest

from profile_report import summarize


class ReportTests(unittest.TestCase):
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
