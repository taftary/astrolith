"""List untaken drafts and whether an Issue already names each path.

Usage:
  python scripts/intake/list_drafts.py

A draft is taken once: before creating an Issue, the take must check no Issue
already names the same draft path. This command prints every Markdown file
under drafts/ (except README files) plus matching Issues found by search.
Exit 0 with a JSON object; human lines go to stderr.
"""

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DRAFTS = ROOT / "drafts"


def search_issues(path):
    p = subprocess.run(
        ["gh", "search", "issues", path, "--json", "number,title,state", "--limit", "10"],
        capture_output=True, text=True, encoding="utf-8", errors="replace")
    if p.returncode != 0:
        return None
    try:
        return json.loads(p.stdout or "[]")
    except json.JSONDecodeError:
        return None


def main():
    if not DRAFTS.is_dir():
        print(json.dumps({"ok": True, "drafts": []}))
        return 0
    drafts = []
    for f in sorted(DRAFTS.rglob("*.md")):
        if f.name.lower() == "readme.md":
            continue
        rel = f.relative_to(ROOT).as_posix()
        hits = search_issues(rel)
        drafts.append({"draft": rel,
                       "taken": bool(hits),
                       "issues": hits if hits else []})
        mark = "TAKEN" if hits else "untaken"
        nums = ", ".join(f"#{h['number']}({h['state']})" for h in (hits or []))
        print(f"{mark}: {rel}" + (f" <- {nums}" if nums else ""), file=sys.stderr)
    print(json.dumps({"ok": True, "drafts": drafts}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
