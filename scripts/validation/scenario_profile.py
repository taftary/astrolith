"""Marked disposable production profiles, never the real configuration tree.

Only initialization copies bytes. All later writes belong to the production app.
Checks are repeated before launch and before retaining disposable settings bytes.
"""

from pathlib import Path
import stat

from profile_run import digest, fingerprint

MARKER = ".astrolith-profile"
USED = ".astrolith-input-used"
MAX_SETTINGS_BYTES = 131_072


def plain_path(path):
    """Inspect the supplied spelling before resolving away links/junctions."""
    path = Path(path).absolute()
    for part in (path, *path.parents):
        try:
            info = part.lstat()
        except FileNotFoundError:
            continue
        if stat.S_ISLNK(info.st_mode) or getattr(info, "st_file_attributes", 0) & 0x400:
            raise ValueError("profile/evidence paths cannot contain links or reparse points")
        if stat.S_ISREG(info.st_mode) and info.st_nlink != 1:
            raise ValueError("profile/evidence files cannot have multiple hard links")
    return path.resolve()


def overlap(a, b):
    return a == b or a.is_relative_to(b) or b.is_relative_to(a)


def outside_real(path, real_root):
    path = plain_path(path)
    if overlap(path, real_root.resolve()):
        raise ValueError("profile/evidence must be outside the real configuration root")
    return path


def settings_path(profile):
    return plain_path(profile / "Astrolith" / "settings.toml")


def exact_marker(path, text):
    path = plain_path(path)
    if not path.is_file() or path.stat().st_size > 64 or path.read_bytes() != text:
        raise ValueError("existing profile lacks the exact disposable input marker")


def prepare(profile, real_root, out, seed=None):
    """Create/inspect only an isolated, explicitly chosen disposable profile."""
    profile = outside_real(profile, real_root)
    if overlap(profile, out):
        raise ValueError("explicit profile and evidence directories must be separate")
    new = not profile.exists()
    if not new:
        exact_marker(profile / MARKER, b"disposable-v1\n")
    if seed is not None and not new:
        raise ValueError("profile fixture may only seed a new disposable profile")
    seed_bytes = None
    if seed is not None:
        seed = outside_real(seed, real_root)
        if not seed.is_file() or seed.stat().st_size > MAX_SETTINGS_BYTES:
            raise ValueError("profile fixture must be a file no larger than 128 KiB")
        seed_bytes = seed.read_bytes()
    target = settings_path(profile)
    if target.exists() and (not target.is_file() or target.stat().st_size > MAX_SETTINGS_BYTES):
        raise ValueError("disposable settings must be a file no larger than 128 KiB")
    used = profile / USED
    if used.exists():
        exact_marker(used, b"input-profile-v1\n")
    if new:
        profile.mkdir(parents=True, exist_ok=False)
        (profile / MARKER).write_bytes(b"disposable-v1\n")
        if seed_bytes is not None:
            target.parent.mkdir()
            target.write_bytes(seed_bytes)
    return profile, {"new_profile": new, "restart": used.exists(),
                     "initial_fixture": str(seed) if seed is not None else None,
                     "initial_fixture_sha256": digest(seed) if seed is not None else None}


def retain(profile, out, label):
    """Copy only marked disposable settings, never real-player file contents."""
    exact_marker(profile / MARKER, b"disposable-v1\n")
    target = settings_path(profile)
    result = fingerprint(target)
    if result["exists"]:
        if not target.is_file() or target.stat().st_size > MAX_SETTINGS_BYTES:
            raise ValueError("disposable settings exceed the bounded evidence contract")
        copy = out / f"profile-{label}.toml"
        with copy.open("xb") as stream:
            stream.write(target.read_bytes())
        result["evidence_file"] = str(copy)
    return result
