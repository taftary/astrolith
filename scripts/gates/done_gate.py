"""Done gate: exits 0 only when the Issue may be closed / moved to Done.

Usage:
  python scripts/gates/done_gate.py --issue N

Requires:
  1. The owner's acceptance word: a comment whose trimmed body is exactly
     `accept` (the AI never writes it).
  2. A post-merge validation record: an Issue comment naming the merge
     commit on `main` (full 40-hex SHA co-occurring with `post-merge`
     and `PASS`).
  3. No open `Needs correction` round: no `<!-- correction:round ... -->`
     marker newer than the latest post-merge record.

Non-zero means the Issue must stay open.
"""

import argparse
import json
import re
import subprocess
import sys

ACCEPT = "accept"
CORRECTION = re.compile(r"<!--\s*correction:round\s+(\d+)\s+owner-fails=(\d+)\s+validator-fails=(\d+)\s*-->")
SHA = re.compile(r"\b([0-9a-f]{40})\b")


def issue_comments(issue):
    p = subprocess.run(
        ["gh", "issue", "view", str(issue), "--json", "comments"],
        capture_output=True, text=True, encoding="utf-8", errors="replace")
    if p.returncode != 0:
        return None, f"cannot read issue #{issue}: {p.stderr.strip()}"
    try:
        data = json.loads(p.stdout or "{}")
    except json.JSONDecodeError as e:
        return None, f"cannot parse issue #{issue}: {e}"
    return [c.get("body", "") or "" for c in data.get("comments", []) or []], ""


def main(argv=None):
    ap = argparse.ArgumentParser(description="Done gate for a parent Issue")
    ap.add_argument("--issue", type=int, required=True)
    a = ap.parse_args(argv)

    comments, err = issue_comments(a.issue)
    if comments is None:
        print(f"done_gate: BLOCKED: {err}")
        return 1
    problems = []

    if not any(c.strip() == ACCEPT for c in comments):
        problems.append(f"no owner `{ACCEPT}` comment")

    post_merge_idx = -1
    for i, c in enumerate(comments):
        low = c.lower()
        if "post-merge" in low and "pass" in low and SHA.search(c):
            post_merge_idx = i
    if post_merge_idx < 0:
        problems.append("no post-merge validation record naming the merge commit SHA")

    open_round = False
    for c in comments[post_merge_idx + 1:]:
        if CORRECTION.search(c):
            open_round = True
            break
    if post_merge_idx < 0:
        open_round = any(CORRECTION.search(c) for c in comments)
    if open_round:
        problems.append("open correction round after the latest post-merge record")

    if problems:
        print(f"done_gate: STAY OPEN for issue #{a.issue}:")
        for p_ in problems:
            print(f"  - {p_}")
        return 1
    print(f"done_gate: DONE OK for issue #{a.issue} (`{ACCEPT}` + post-merge record, no open correction)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
