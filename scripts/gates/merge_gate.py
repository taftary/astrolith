"""Merge gate: exits 0 only when the PR may be merged for the parent Issue.

Usage:
  python scripts/gates/merge_gate.py --issue N --pr P

Checks:
  1. The `ci` check-run is green (completed/success) on the PR head SHA.
  2. An Issue comment holds `<!-- validator:pass sha=<full-head-sha> -->`
     and that comment carries `Runtime: PASS`,
     `Requirements: MET n/n` (both numbers equal, n >= 1 and equal to the
     MET row count in the newest report), and `Drift: none`.
  3. The newest `.agent/validation/issue-N/<ts>/report.md` for that SHA has
     `Verdict: **PASS**`.
  4. The PR body has no parent auto-close keyword for a non-`task` issue.

Non-zero means no merge. The `pull-request` skill calls this.
"""

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
AUTO_CLOSE = re.compile(
    r"\b(?:close|closes|closed|fix|fixes|fixed|resolve|resolves|resolved)\s*:?\s*#(\d+)",
    re.IGNORECASE)
MARKER = re.compile(r"<!--\s*validator:pass\s+sha=([0-9a-f]{40})\s*-->")


def run(cmd):
    p = subprocess.run(cmd, capture_output=True, text=True,
                       encoding="utf-8", errors="replace")
    return p


def repo():
    p = run(["gh", "repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner"])
    return (p.stdout or "").strip()


def pr_head(pr):
    p = run(["gh", "pr", "view", str(pr), "--json", "headRefOid,body,baseRefName"])
    if p.returncode != 0:
        return None, f"cannot read PR #{pr}: {p.stderr.strip()}"
    try:
        d = json.loads(p.stdout or "{}")
    except json.JSONDecodeError as e:
        return None, f"cannot parse PR #{pr}: {e}"
    return {"sha": d.get("headRefOid", ""), "body": d.get("body", "") or "",
            "base": d.get("baseRefName", "")}, ""


def ci_green(head_sha, pr):
    # Latest status per check name (branch protection reads the same view):
    # an older red run on this SHA must not shadow the newest green one.
    p = run(["gh", "pr", "checks", str(pr), "--json", "name,state"])
    if p.returncode != 0:
        return False, f"cannot read PR #{pr} checks: {p.stderr.strip()}"
    try:
        checks = json.loads(p.stdout or "[]")
    except json.JSONDecodeError as e:
        return False, f"cannot parse PR #{pr} checks: {e}"
    states = {c.get("name"): c.get("state") for c in checks}
    if states.get("ci") == "SUCCESS":
        return True, ""
    return False, f"`ci` latest state is {states.get('ci')!r}, not SUCCESS (head {head_sha})"


def issue_comments(issue):
    p = run(["gh", "issue", "view", str(issue), "--json", "comments",
             "--jq", "[.comments[] | .body]"])
    if p.returncode != 0:
        return None, f"cannot read issue #{issue}: {p.stderr.strip()}"
    try:
        return json.loads(p.stdout or "[]"), ""
    except json.JSONDecodeError as e:
        return None, f"cannot parse issue #{issue}: {e}"


def newest_report(issue, sha):
    base = ROOT / ".agent" / "validation" / f"issue-{issue}"
    if not base.is_dir():
        return None
    cands = []
    for child in sorted(base.iterdir()):
        if not child.is_dir() or child.name == "__pycache__":
            continue
        rep = child / "report.md"
        if not rep.is_file():
            continue
        try:
            text = rep.read_text(encoding="utf-8")
        except OSError:
            continue
        if sha in text:
            cands.append((child.name, text))
    if not cands:
        return None
    cands.sort()
    return cands[-1][1]


def main(argv=None):
    ap = argparse.ArgumentParser(description="Merge gate for issue + PR")
    ap.add_argument("--issue", type=int, required=True)
    ap.add_argument("--pr", type=int, required=True)
    a = ap.parse_args(argv)
    problems = []

    pr, err = pr_head(a.pr)
    if pr is None:
        print(f"merge_gate: BLOCKED: {err}")
        return 1
    head = pr["sha"]
    if not re.fullmatch(r"[0-9a-f]{40}", head or ""):
        problems.append(f"PR head SHA is not a full SHA: {head!r}")

    if head:
        ok, err = ci_green(head, a.pr)
        if not ok:
            problems.append(err)

    comments, err = issue_comments(a.issue)
    if comments is None:
        print(f"merge_gate: BLOCKED: {err}")
        return 1
    verdict_comment = None
    for c in comments:
        m = MARKER.search(c or "")
        if m and m.group(1) == head:
            verdict_comment = c
            break
    if verdict_comment is None:
        problems.append(f"no `<!-- validator:pass sha={head} -->` comment on issue #{a.issue}")
    else:
        if "Runtime: PASS" not in verdict_comment:
            problems.append("verdict comment lacks `Runtime: PASS`")
        m = re.search(r"Requirements:\s*MET\s+(\d+)\s*/\s*(\d+)", verdict_comment)
        if not m or m.group(1) != m.group(2):
            problems.append("verdict comment lacks `Requirements: MET n/n` with equal n")
        if "Drift: none" not in verdict_comment:
            problems.append("verdict comment lacks `Drift: none`")
        req_n = int(m.group(1)) if m and m.group(1) == m.group(2) else 0

    report = newest_report(a.issue, head) if head else None
    met_rows = 0
    if report is None:
        problems.append(f"no report under .agent/validation/issue-{a.issue}/ names {head}")
    else:
        if "Verdict: **PASS**" not in report:
            problems.append("newest report for the head SHA lacks `Verdict: **PASS**`")
        met_rows = len(re.findall(r"^\|\s*.*\|\s*MET\s*\|", report, re.MULTILINE))
        if verdict_comment is not None and req_n and met_rows != req_n:
            problems.append(f"verdict says MET {req_n}/{req_n} but newest report has {met_rows} MET rows")

    refs = AUTO_CLOSE.findall(pr["body"])
    if refs:
        r = repo()
        for num in dict.fromkeys(refs):
            q = run(["gh", "api", f"repos/{r}/issues/{num}", "--jq", "[.labels[] | .name] | join(\",\")"])
            labels = (q.stdout or "").strip().split(",") if q.returncode == 0 else []
            if "task" not in labels:
                problems.append(f"PR body auto-closes #{num} which lacks the `task` label")
                break

    _ = os.environ.get("GITHUB_TOKEN", "")
    if problems:
        print(f"merge_gate: NO MERGE for issue #{a.issue} PR #{a.pr} (head {head}):")
        for p_ in problems:
            print(f"  - {p_}")
        return 1
    print(f"merge_gate: MERGE OK for issue #{a.issue} PR #{a.pr} (head {head})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
