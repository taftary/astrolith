"""Main-agent-only fixed release build and receipt. Not a validation command.

python scripts/build_release.py --out NEW_DIRECTORY
No command, executable, compiler flag or environment payload is accepted as input.
"""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
COMMAND = ["cargo", "build", "--locked", "--release", "-p", "universe-app"]


def capture(*argv):
    return subprocess.check_output(argv, cwd=ROOT, text=True, encoding="utf-8", timeout=30).strip()


def run(out):
    forbidden = ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER",
                 "CARGO_TARGET_DIR", "CARGO_BUILD_TARGET", "CARGO_BUILD_RUSTC", "CARGO_BUILD_RUSTFLAGS")
    if any(os.environ.get(key) for key in forbidden) or any(key.startswith("CARGO_PROFILE_RELEASE_") for key in os.environ):
        raise ValueError("release receipt refuses external compiler/target/profile overrides (values not recorded)")
    tracked = capture("git", "status", "--porcelain", "--untracked-files=no")
    if tracked:
        raise ValueError("commit tracked source before building the release receipt")
    commit = capture("git", "rev-parse", "HEAD")
    tree = capture("git", "rev-parse", "HEAD^{tree}")
    out = out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    record = {"schema": "main-agent-release-build-v1", "command": COMMAND, "cwd": str(ROOT),
              "started_utc": datetime.now(timezone.utc).isoformat(), "source_commit": commit,
              "source_tree": tree, "tracked_before": tracked, "rustc": capture("rustc", "-Vv"),
              "cargo": capture("cargo", "-V"), "exit_code": None,
              "external_compiler_target_profile_overrides": "absent"}
    receipt = out / "receipt.json"

    def save():
        receipt.write_text(json.dumps(record, indent=2, allow_nan=False) + "\n", encoding="utf-8")

    save()
    with (out / "stdout.txt").open("wb") as stdout, (out / "stderr.txt").open("wb") as stderr:
        result = subprocess.run(COMMAND, cwd=ROOT, stdout=stdout, stderr=stderr, check=False)
    record.update(exit_code=result.returncode, completed_utc=datetime.now(timezone.utc).isoformat(),
                  source_commit_after=capture("git", "rev-parse", "HEAD"),
                  source_tree_after=capture("git", "rev-parse", "HEAD^{tree}"),
                  tracked_after=capture("git", "status", "--porcelain", "--untracked-files=no"))
    executable = ROOT / "target/release" / ("universe-app.exe" if os.name == "nt" else "universe-app")
    if result.returncode == 0:
        with executable.open("rb") as stream:
            sha = hashlib.file_digest(stream, "sha256").hexdigest()
        record.update(executable=str(executable), executable_sha256=sha,
                      executable_bytes=executable.stat().st_size)
    save()
    ok = (result.returncode == 0 and record["source_commit_after"] == commit
          and record["source_tree_after"] == tree and not record["tracked_after"])
    print(json.dumps({"ok": ok, "receipt": str(receipt), "sha256": record.get("executable_sha256")}))
    return 0 if ok else 1


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True)
    try:
        sys.exit(run(parser.parse_args().out))
    except (OSError, ValueError, subprocess.SubprocessError) as exc:
        print(json.dumps({"ok": False, "reason": str(exc)}))
        sys.exit(1)
