"""Read-only release receipt checks. Never compiles or evaluates a command."""

import json
from datetime import datetime
import re
from pathlib import Path

from profile_run import digest, git

COMMAND = ["cargo", "build", "--locked", "--release", "-p", "universe-app"]


def verify(path, executable, expected_commit):
    executable = Path(executable)
    return verify_record(path, expected_commit, digest(executable), executable.stat().st_size)


def verify_record(path, expected_commit, executable_hash, executable_size):
    """Audit retained attribution without demanding an old executable still exist."""
    path = Path(path)
    if not path.is_file() or path.stat().st_size > 131_072:
        raise ValueError("build receipt missing or exceeds 128 KiB")
    record = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(record, dict) or record.get("schema") != "main-agent-release-build-v1" or record.get("command") != COMMAND:
        raise ValueError("build receipt has no fixed main-agent release command")
    commit, tree = record.get("source_commit", ""), record.get("source_tree", "")
    if not isinstance(commit, str) or not isinstance(tree, str) or not re.fullmatch(r"[a-f0-9]{40}", commit) or not re.fullmatch(r"[a-f0-9]{40}", tree):
        raise ValueError("build receipt source identity is malformed")
    if (commit != expected_commit or record.get("source_commit_after") != commit
            or record.get("source_tree_after") != tree or git("rev-parse", f"{commit}^{{tree}}") != tree):
        raise ValueError("build receipt source commit/tree mismatch")
    if record.get("tracked_before") != "" or record.get("tracked_after") != "" or type(record.get("exit_code")) is not int or record["exit_code"] != 0:
        raise ValueError("build receipt is dirty or did not complete successfully")
    for key in ("rustc", "cargo", "started_utc", "completed_utc", "cwd"):
        if not isinstance(record.get(key), str) or not record[key].strip():
            raise ValueError(f"build receipt is missing {key}")
    start, end = (datetime.fromisoformat(record[key]) for key in ("started_utc", "completed_utc"))
    if start.tzinfo is None or end.tzinfo is None or start >= end:
        raise ValueError("build receipt completion timestamps are missing or unordered")
    if record.get("executable_sha256") != executable_hash or record.get("executable_bytes") != executable_size:
        raise ValueError("build receipt executable hash/size mismatch")
    return {"verified": True, "receipt_sha256": digest(path), "source_commit": commit,
            "source_tree": tree, "executable_sha256": record["executable_sha256"],
            "trust": "main-agent build record, not a reproducible-build attestation"}
