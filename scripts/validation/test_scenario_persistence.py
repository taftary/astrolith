"""Hermetic report/encoding tests; actual restart proof requires real app runs."""

from pathlib import Path
import tempfile
import unittest

from scenario_persistence import observation, serialized, saved_matches, unescape


class PersistenceTests(unittest.TestCase):
    def test_escaped_text_is_lossless_and_invalid_sequences_are_refused(self):
        self.assertEqual(unescape(r"a\\b\tline\nnext\r"), "a\\b\tline\nnext\r")
        for invalid in ("\\", r"\q"):
            with self.assertRaises(ValueError):
                unescape(invalid)

    def test_stale_or_missing_saved_bytes_do_not_satisfy_live_quit_state(self):
        with tempfile.TemporaryDirectory(prefix="astrolith-input-report-") as tmp:
            root = Path(tmp)
            trace = root / "scenario.tsv"
            fields = {"labels": "off", "quality_mode": "auto", "planet_detail": "normal", "game_time": "123"}
            lines = ["OBSERVE\tterminal\tstep=1"]
            for key, val in fields.items():
                lines += [f"VALUE\tterminal\tsettings.saved.{key}\t{val}",
                          f"VALUE\tterminal\tsettings.saved.{key}.encoding\tbackslash-escaped-utf8"]
            trace.write_text("\n".join(lines) + "\n", encoding="utf-8")
            file = root / "settings.toml"
            file.write_text("".join(f"{k} = {v}\n" for k, v in fields.items()), encoding="utf-8")
            self.assertEqual(serialized(observation(trace, "terminal")), fields)
            self.assertTrue(saved_matches(trace, file))
            file.write_text("labels = on\n", encoding="utf-8")
            self.assertFalse(saved_matches(trace, file))
            trace.write_text("\n".join(lines + [lines[1]]) + "\n", encoding="utf-8")
            with self.assertRaises(ValueError):
                observation(trace, "terminal")
            with self.assertRaises(ValueError):
                serialized({})


if __name__ == "__main__":
    unittest.main()
