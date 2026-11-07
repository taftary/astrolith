"""Hermetic profile boundary and initialization tests, not persistence proof."""

import os
from pathlib import Path
import tempfile
import unittest

import scenario_profile as profile


class ProfileTests(unittest.TestCase):
    def test_initial_copy_is_once_only_and_restart_retains_real_bytes(self):
        with tempfile.TemporaryDirectory(prefix="astrolith-input-profile-") as tmp:
            root = Path(tmp).resolve()
            real = root / "real"
            real.mkdir()
            out = root / "out"
            out.mkdir()
            seed = root / "seed.toml"
            seed.write_bytes(b"labels = off\n")
            target, meta = profile.prepare(root / "profile", real, out, seed)
            self.assertTrue(meta["new_profile"])
            self.assertFalse(meta["restart"])
            self.assertEqual(profile.settings_path(target).read_bytes(), seed.read_bytes())
            (target / profile.USED).write_bytes(b"input-profile-v1\n")
            with self.assertRaises(ValueError):
                profile.prepare(target, real, out, seed)
            _, meta = profile.prepare(target, real, out)
            self.assertTrue(meta["restart"])
            retained = profile.retain(target, out, "before")
            self.assertEqual(Path(retained["evidence_file"]).read_bytes(), seed.read_bytes())
            self.assertEqual(list(real.iterdir()), [])

    def test_real_overlapping_unmarked_and_bad_markers_are_refused(self):
        with tempfile.TemporaryDirectory(prefix="astrolith-input-path-") as tmp:
            root = Path(tmp).resolve()
            real = root / "real"
            real.mkdir()
            out = root / "out"
            for bad in (real, real / "inside", root, out / "profile", out):
                with self.assertRaises(ValueError):
                    profile.prepare(bad, real, out)
            unmarked = root / "unmarked"
            unmarked.mkdir()
            with self.assertRaises(ValueError):
                profile.prepare(unmarked, real, out)
            (unmarked / profile.MARKER).write_bytes(b"disposable-v1\r\n")
            with self.assertRaises(ValueError):
                profile.prepare(unmarked, real, out)

    def test_hardlinked_settings_cannot_redirect_production_writes(self):
        with tempfile.TemporaryDirectory(prefix="astrolith-input-links-") as tmp:
            root = Path(tmp).resolve()
            real = root / "real"
            real.mkdir()
            original = real / "settings.toml"
            original.write_bytes(b"private-test-placeholder\n")
            target, _ = profile.prepare(root / "profile", real, root / "out")
            settings = profile.settings_path(target)
            settings.parent.mkdir()
            os.link(original, settings)
            with self.assertRaises(ValueError):
                profile.prepare(target, real, root / "out")
            self.assertEqual(original.read_bytes(), b"private-test-placeholder\n")

    def test_symlink_spelling_is_refused_before_canonical_resolution(self):
        with tempfile.TemporaryDirectory(prefix="astrolith-input-symlink-") as tmp:
            root = Path(tmp).resolve()
            dest = root / "destination"
            dest.mkdir()
            link = root / "link"
            try:
                link.symlink_to(dest, target_is_directory=True)
            except OSError as exc:
                self.skipTest(f"symlink creation unavailable: {exc}")
            with self.assertRaises(ValueError):
                profile.plain_path(link / "not-yet-created")


if __name__ == "__main__":
    unittest.main()
