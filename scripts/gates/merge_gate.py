"""Merge gate: exits 0 only when the PR may be merged for the parent Issue.

Usage:
  python scripts/gates/merge_gate.py --issue N --pr P

Checks (per docs/workflow.md, Branching and Merge: CI green, a validator
pass recorded on the Issue for the exact commit being merged, and no
auto-close keyword for the parent Issue):
  1. The `ci` check-run is green (completed/success) on the PR head SHA.
  2. An Issue comment holds `<!-- validator:pass sha=<full-head-sha> -->`
     and that comment carries `Runtime: PASS`,
     `Requirements: MET n/n` (both numbers equal, n >= 1; the `n of n`
     wording means the same and is accepted), `Drift: none`, and a `Visual:` frame-proof line.
  3. The newest `.agent/validation/issue-N/<ts>/report.md` for that SHA
     shows clean runtime findings: every recorded `rc=` is 0, the
     determinism edge probe holds (`identical=True`), and the tree was
     clean (`dirty=False`). It deliberately does NOT require every
     criterion row MET: docs/workflow.md lets the validator return
     MET / PARTIAL / NOT MET / UNVERIFIABLE with evidence, and
     UNVERIFIABLE plus multi-probe criteria can never read MET from the
     script. Judgment stays with the independent validator pass above;
     this check only blocks on red runtime evidence.
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
# Degraded form: "Visual: BLOCKED (infra: <cause>)" (issue #198, AC4).
# The only gate that may weaken: an infra-only picture-proof failure may
# still merge, but only when the record names the cause, the
# no-capturable-change reason, and the byte-identical golden tests.
DEGRADED = re.compile(r"Visual:\s*BLOCKED\s*\(infra:\s*([^)]+)\)", re.IGNORECASE)


def degraded_visual_ok(comment):
    """The degraded Visual line, or None when the line is not degraded.

    Returns True only when the exact AC4 record holds: a named infra
    cause plus the no-capturable-byte-changed reason plus the golden
    tests. Any other BLOCKED wording is refused (False).
    """
    m = DEGRADED.search(comment or "")
    if not m:
        return None
    if not (m.group(1) or "").strip():
        return False
    low = (comment or "").lower()
    return ("no capturable byte changed" in low and "golden" in low)


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
        m = re.search(r"Requirements:\s*MET\s+(\d+)\s*(?:/|of)\s*(\d+)", verdict_comment)
        if not m or m.group(1) != m.group(2) or int(m.group(1)) < 1:
            problems.append("verdict comment lacks `Requirements: MET n/n` with equal n (n >= 1)")
        if "Drift: none" not in verdict_comment:
            problems.append("verdict comment lacks `Drift: none`")
        if "Visual:" not in verdict_comment:
            problems.append("verdict comment lacks a `Visual:` frame-proof line "
                            "(validator pastes the frame-proof.py fragment into the verdict)")
        else:
            visual_line = verdict_comment.split("Visual:", 1)[1].split("\n", 1)[0]
            if "BLOCKED" in visual_line.upper():
                if degraded_visual_ok(verdict_comment) is not True:
                    problems.append("degraded `Visual: BLOCKED` line must read "
                                    "`Visual: BLOCKED (infra: <cause>)` and name the "
                                    "no-capturable-byte-changed reason and the golden tests")

    report = newest_report(a.issue, head) if head else None
    if report is None:
        problems.append(f"no report under .agent/validation/issue-{a.issue}/ names {head}")
    else:
        rcs = re.findall(r"rc=(\d+)", report)
        if not rcs or any(int(x) != 0 for x in rcs):
            problems.append("newest report for the head SHA shows a non-zero rc")
        if "identical=True" not in report:
            problems.append("newest report for the head SHA lacks the determinism edge probe (identical=True)")
        if "dirty=False" not in report:
            problems.append("newest report for the head SHA was not on a clean tree (dirty=False)")

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
