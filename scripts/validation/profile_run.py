"""Bounded, opt-in release-app measurement driver (#1087).

Usage: python scripts/validation/profile_run.py --route FILE --out NEW_DIRECTORY
Build with cargo build --locked --release -p universe-app first. This driver
launches that executable directly, never an arbitrary command or shell payload.
"""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import subprocess
import sys
import tempfile

from profile_process import observe, physical_ram, write_json
from profile_report import summarize

ROOT = Path(__file__).resolve().parents[2]
APP = ROOT / "target" / "release" / ("universe-app.exe" if os.name == "nt" else "universe-app")


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def finite_positive(text):
    value = float(text)
    if not math.isfinite(value) or value <= 0:
        raise argparse.ArgumentTypeError("must be a finite positive number")
    return value


def positive_integer(text):
    value = int(text)
    if value <= 0:
        raise argparse.ArgumentTypeError("must be positive")
    return value


def git(*args):
    result = subprocess.run(["git", *args], cwd=ROOT, text=True, capture_output=True,
                            check=True, timeout=15)
    return result.stdout.strip()


def player_settings():
    if os.name != "nt" or not os.environ.get("APPDATA"):
        raise OSError("Windows APPDATA must be available to verify settings isolation")
    return Path(os.environ["APPDATA"]) / "Astrolith" / "settings.toml"


def fingerprint(path):
    return {"exists": path.exists(), "sha256": digest(path) if path.exists() else None}


class RunLock:
    """OS-released lock: concurrent runners must not contaminate each other."""

    def __enter__(self):
        import msvcrt
        self.file = (Path(tempfile.gettempdir()) / "astrolith-profile.lock").open("a+b")
        self.file.seek(0, 2)
        if self.file.tell() == 0:
            self.file.write(b"0")
            self.file.flush()
        self.file.seek(0)
        try:
            msvcrt.locking(self.file.fileno(), msvcrt.LK_NBLCK, 1)
        except OSError:
            self.file.close()
            raise OSError("another profiling runner holds the measurement lock") from None
        return self

    def __exit__(self, *args):
        import msvcrt
        self.file.seek(0)
        msvcrt.locking(self.file.fileno(), msvcrt.LK_UNLCK, 1)
        self.file.close()


def parser():
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument("--route", type=Path, required=True)
    result.add_argument("--out", type=Path, required=True)
    result.add_argument("--timeout", type=finite_positive, default=1500.0)
    result.add_argument("--workers", type=positive_integer, default=5)
    result.add_argument("--width", type=positive_integer, default=1600)
    result.add_argument("--height", type=positive_integer, default=900)
    result.add_argument("--collection", choices=("on", "off"), default="on")
    result.add_argument("--refresh-hz", type=finite_positive)
    result.add_argument("--power-note", default="unavailable: not observed")
    return result


def run(args):
    if os.name != "nt":
        raise OSError("this working-set runner currently supports Windows only")
    route = args.route.resolve(strict=True)
    if not route.is_file():
        raise ValueError("route must be a file")
    if not APP.is_file():
        raise ValueError("release app missing; build it before profiling")
    out = args.out.resolve()
    settings = player_settings()
    before = fingerprint(settings)
    with RunLock():
        # Atomic refusal, even if an existing directory is empty.
        out.mkdir(parents=True, exist_ok=False)
        local_route = out / "fixture.route"
        local_route.write_bytes(route.read_bytes())
        argv = [str(APP), "--route", str(local_route), str(out / "route"),
                "--size", f"{args.width}x{args.height}", "--build-workers", str(args.workers)]
        env = os.environ.copy()
        env["APPDATA"] = str(out / "profile")
        env["ASTROLITH_PROFILE"] = "1" if args.collection == "on" else "0"
        manifest = {
            "schema": 1, "started_utc": datetime.now(timezone.utc).isoformat(),
            "commit": git("rev-parse", "HEAD"),
            "tracked_changes": git("status", "--porcelain", "--untracked-files=no"),
            "executable_sha256": digest(APP), "executable_mtime_ns": APP.stat().st_mtime_ns,
            "build_identity_note": "executable hash is authoritative; build provenance must accompany comparisons",
            "fixture_sha256": digest(local_route), "argv": argv, "cwd": str(ROOT),
            "platform": platform.platform(), "cpu": platform.processor(),
            "logical_cpu_count": os.cpu_count(),
            "physical_ram_bytes": physical_ram(),
            "refresh_hz_observed": args.refresh_hz, "power_note_observed": args.power_note,
            "backend_driver_source": "route/route.txt ROUTE-RUN (actual adapter)",
            "output_pixels": [args.width, args.height], "workers": args.workers,
            "collection": args.collection, "timeout_s": args.timeout,
            "settings_before": before,
            "limits": ["working set is process resident memory, not GPU memory",
                       "one-second observed peak can miss transient peaks",
                       "unknown or unequal environment facts prevent a matched-comparison claim"],
        }
        write_json(out / "manifest.json", manifest)
        result = observe(argv, out, ROOT, env, args.timeout, args.collection == "on")
        manifest["process"] = result
        manifest["settings_after"] = fingerprint(settings)
        manifest["settings_unchanged"] = before == manifest["settings_after"]
        completed = result["status"] == "completed"
        if completed:
            text = (out / "stdout.txt").read_text(encoding="utf-8", errors="replace")
            completed = "ROUTE-OK " in text and all(
                (out / "route" / name).is_file() for name in ("route.txt", "frames.txt"))
            if completed:
                try:
                    write_json(out / "summary.json", summarize(out / "route"))
                except (OSError, ValueError) as exc:
                    completed = False
                    manifest["summary_problem"] = str(exc)
        manifest["complete"] = completed and manifest["settings_unchanged"]
        write_json(out / "manifest.json", manifest)
        print(json.dumps({"ok": manifest["complete"], "out": str(out), "process": result["status"]}))
        return 0 if manifest["complete"] else 1


def main():
    args = parser().parse_args()
    try:
        return run(args)
    except (OSError, ValueError, subprocess.SubprocessError) as exc:
        print(json.dumps({"ok": False, "reason": str(exc)}))
        return 1


if __name__ == "__main__":
    sys.exit(main())
