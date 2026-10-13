"""One-time and on-demand cleanup of untracked validation evidence (#613).

Selects, in the main checkout's `.agent/validation/`:
  - every untracked item of a **closed** issue (its evidence after Done);
  - for **open** issues, every untracked timestamped run folder except the
    newest one.
Tracked files (`requirements.md` and the like) are never selected.

Without `--yes` it only lists what it would delete and the total size; with
`--yes` it deletes them. JSON to stdout.

Usage:
  python scripts/sidebar/cleanup_evidence.py            # list only
  python scripts/sidebar/cleanup_evidence.py --yes      # delete
"""
import argparse
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts" / "validation"))

import evidence  # noqa: E402

ISSUE_DIR = re.compile(r"^issue-(\d+)$")


def run(cmd):
    """Run one gh command; tests replace this with a fake runner."""
    return subprocess.run(cmd, capture_output=True, text=True,
                          encoding="utf-8", errors="replace")


def closed_issues():
    """Numbers of closed issues (state from GitHub), or None on failure."""
    p = run(["gh", "issue", "list", "--state", "closed", "--limit", "1000",
             "--json", "number", "--jq", "[.[].number]"])
    if p.returncode != 0:
        return None
    try:
        return set(json.loads(p.stdout or "[]"))
    except json.JSONDecodeError:
        return None


def select(root, closed):
    """(path, reason) of every untracked item to delete."""
    base = evidence.evidence_dir(root)
    if not base.is_dir():
        return []
    picked = []
    for issue_dir in sorted(base.iterdir()):
        m = ISSUE_DIR.match(issue_dir.name)
        if not m or not issue_dir.is_dir():
            continue
        issue = int(m.group(1))
        untracked = evidence.untracked_children(root, issue)
        if issue in closed:
            picked.extend((p, f"issue #{issue} is closed") for p in untracked)
            continue
        old_runs = [r for r in evidence.runs(issue_dir) if r in untracked][:-1]
        picked.extend((p, f"older run of open issue #{issue}") for p in old_runs)
    return picked


def main(argv=None):
    ap = argparse.ArgumentParser(description="Untracked evidence cleanup")
    ap.add_argument("--yes", action="store_true", help="delete (default: list only)")
    a = ap.parse_args(argv)
    closed = closed_issues()
    if closed is None:
        print(json.dumps({"ok": False, "error": "cannot read closed issues from GitHub"}))
        return 1
    picked = select(ROOT, closed)
    total = sum(evidence.size(p) if p.is_dir() else p.stat().st_size for p, _ in picked)
    if a.yes:
        for path, _ in picked:
            if path.is_dir():
                shutil.rmtree(path, ignore_errors=True)
            else:
                path.unlink(missing_ok=True)
    print(json.dumps({"ok": True, "deleted": a.yes, "items": len(picked),
                      "totalGB": round(total / 1024 ** 3, 2),
                      "sample": [f"{p} ({why})" for p, why in picked[:10]]}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
