"""Hermetic report/encoding tests; actual restart proof requires real app runs."""

from pathlib import Path
import tempfile
import unittest

from scenario_persistence import loaded_context, observation, serialized, saved_matches, unescape


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

    def test_stored_player_is_not_proof_of_live_restoration_or_clock_resume(self):
        old = {"player.remembered.path": "planet", "player.remembered.position": "1,2,3",
               "player.remembered.position.frame": "planet-metres", "player.remembered.yaw_rad": "0.5"}
        loaded = {**old, "player.selected": "true", "navigation.destination.path": "planet",
                  "navigation.destination.anchored": "true", "clock.game_s": "123.1"}
        saved = {"game_time": "123", "player": "saved-production-record"}
        differences, unexpected, clock = loaded_context(saved, old, loaded)
        self.assertEqual(differences, [])
        self.assertEqual(unexpected, [])
        self.assertGreater(clock["startup_advance_game_s"], 0)
        bad = {**loaded, "player.remembered.position": "0,0,0"}
        self.assertIn("player.remembered.position", loaded_context(saved, old, bad)[1])
        differences, unexpected, _ = loaded_context({"game_time": "123"}, old, bad)
        self.assertIn("player.remembered.position", differences)
        self.assertEqual(unexpected, [], "an unsaved live pose is not promised on restart after Views Reset")
        differences, unexpected, _ = loaded_context({"game_time": "123"}, old, {**loaded, "player.selected": "false"})
        self.assertIn("loaded-player-selection-and-destination", unexpected)
        for value in ("0", "nan", "inf"):
            with self.assertRaises(ValueError):
                loaded_context({"game_time": "123"}, old, {**loaded, "clock.game_s": value})
        with self.assertRaises(ValueError):
            loaded_context({"game_time": "123"}, {}, loaded)


if __name__ == "__main__":
    unittest.main()
