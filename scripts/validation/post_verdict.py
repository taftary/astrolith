"""Post the validator's verdict comment for one commit (issue #533).

Usage:
  python scripts/validation/post_verdict.py --issue N --sha <head-sha> --drift none
      [--drift-from "what the owner approved"] [--scope none] [--note "..."]
      [--judge "C3=MET: reason"]... [--owner-decision <comment URL>] [--dry-run]

validate.py writes verdict.md beside report.md in the exact form the gates
read (marker, `Validator:` line, Checked, Evidence, `Runtime:`,
`Requirements: MET k/n`). This script adds the validator's own judgment
(Scope, Notes, and the closing `Drift:` line), posts it once as an Issue
comment, and prints the `Validator:`-to-`Drift:` block for the PR body, so
compare_verdict.py matches by construction and no round is spent fixing
wording. Drift other than `none` turns a pass into a fail. A second post for
the same commit is refused, so the verdict is posted once. `--drift-from`
records an owner-approved change from the notion on a `Drift from notion:`
line after `Drift: none` (#506); the gates keep reading the `Drift: none`
line, so the citation never blocks the merge.

Hand-judged criteria (#1032, owner Q1 A): `--judge "Ck=MET: reason"` (or
`NOT MET`) records the validator's judgment of a criterion that validate.py
left UNVERIFIABLE because its text says "judged by the validator". The row
reads `MET (judged)` with the reason, `Requirements: MET k/n` is recounted,
and the verdict turns PASS only with `Runtime: PASS` and every row MET. A
criterion the script checks itself cannot be judged by hand. A second
verdict for the same commit is refused unless `--owner-decision` names a
comment on the same Issue written by the repository owner (#1032, Q2 A);
the replacement cites it, and the gates read the newest verdict (#506).

JSON to stdout. Does not edit code and never posts approval words.
"""
import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
AGENT = ROOT / ".agent" / "validation"
SPAN = re.compile(r"(?m)^Validator:.*?$[\s\S]*?^Drift:.*$")


def run(cmd):
    """Run one gh command; tests replace this with a fake runner."""
    return subprocess.run(cmd, capture_output=True, text=True,
                          encoding="utf-8", errors="replace", cwd=str(ROOT))


def newest_verdict(issue, sha, base=None):
    """(run dir, verdict.md text) of the newest run for `sha`, or (None, "")."""
    d = (base or AGENT) / f"issue-{issue}"
    if not d.is_dir():
        return None, ""
    hits = []
    for child in sorted(d.iterdir()):
        v = child / "verdict.md"
        if v.is_file():
            text = v.read_text(encoding="utf-8")
            if f"sha={sha} -->" in text:
                hits.append((child, text))
    return hits[-1] if hits else (None, "")


JUDGE = re.compile(r"^\s*([A-Z]+\d+)\s*=\s*(MET|NOT MET)\s*:\s*(\S.*)$")
ROW = re.compile(r"^- (MET|NOT MET|PARTIAL|UNVERIFIABLE|MET \(judged\)|NOT MET \(judged\)): ([A-Z]+\d+): ")
HAND = "judged by the validator"


def apply_judgments(text, judges):
    """`text` with the validator's `judges` written into its hand-judged
    rows, the count and the verdict redone; `(text, errors)`. Pure (#1032)."""
    errors = []
    wanted = {}
    for j in judges:
        m = JUDGE.match(j or "")
        if not m:
            errors.append(f"--judge {j!r}: write it as 'C3=MET: reason' or 'C3=NOT MET: reason'")
            continue
        wanted[m.group(1)] = (m.group(2), " ".join(m.group(3).split()))
    if not wanted and not errors:
        return text, []
    lines = text.split("\n")
    seen = set()
    for i, line in enumerate(lines):
        m = ROW.match(line)
        if not m or m.group(2) not in wanted:
            continue
        cid = m.group(2)
        seen.add(cid)
        if m.group(1) != "UNVERIFIABLE" or HAND not in line:
            errors.append(f"{cid} is checked by the script ({m.group(1)}); "
                          "it cannot be judged by hand")
            continue
        status, reason = wanted[cid]
        lines[i] = f"- {status} (judged): {line[m.end(1) + 2:]} (judged: {reason})"
    errors += [f"{cid} is not a criterion of this verdict" for cid in wanted if cid not in seen]
    if errors:
        return text, errors
    text = "\n".join(lines)
    rows = [m for m in (ROW.match(line) for line in lines) if m]
    met = sum(1 for m in rows if m.group(1) in ("MET", "MET (judged)"))
    text = re.sub(r"(?m)^Requirements: MET \d+/\d+$",
                  f"Requirements: MET {met}/{len(rows)}", text, count=1)
    if rows and met == len(rows) and re.search(r"(?m)^Runtime: PASS$", text):
        text = text.replace("<!-- validator:fail ", "<!-- validator:pass ", 1)
        text = re.sub(r"(?m)^Validator: FAIL ", "Validator: PASS ", text, count=1)
    else:
        text = text.replace("<!-- validator:pass ", "<!-- validator:fail ", 1)
        text = re.sub(r"(?m)^Validator: PASS ", "Validator: FAIL ", text, count=1)
    return text, []


def owner_decision_problem(issue, url):
    """Why `url` is not an owner's decision comment on `issue`; None when it is."""
    m = re.search(r"/issues/(\d+)#issuecomment-(\d+)$", url or "")
    if not m or int(m.group(1)) != issue:
        return f"--owner-decision must be a comment URL on issue #{issue}"
    p = run(["gh", "api", f"repos/{{owner}}/{{repo}}/issues/comments/{m.group(2)}",
             "--jq", "[.user.login, .issue_url] | @tsv"])
    o = run(["gh", "repo", "view", "--json", "owner", "--jq", ".owner.login"])
    if p.returncode != 0 or o.returncode != 0:
        return f"cannot read the decision comment {url}"
    author, issue_url = ((p.stdout or "").strip().split("\t") + ["", ""])[:2]
    if not issue_url.endswith(f"/issues/{issue}"):
        return f"the decision comment is not on issue #{issue}"
    if author != (o.stdout or "").strip():
        return f"the decision comment is written by {author}, not the repository owner"
    return None


def finish(text, drift, scope, notes, drift_from=""):
    """The full comment: verdict.md plus Scope, Notes, and the closing Drift line."""
    drift = " ".join((drift or "").split())
    if drift.lower() != "none":
        text = text.replace("<!-- validator:pass ", "<!-- validator:fail ", 1)
        text = re.sub(r"(?m)^Validator: PASS ", "Validator: FAIL ", text, count=1)
    # Backstop for #506, mirroring validate.py: a headed verdict fails
    # instead of posting a block the merge gate will never accept.
    headed = bool(re.search(r"(?m)^\s*#", text or ""))
    if headed:
        text = text.replace("<!-- validator:pass ", "<!-- validator:fail ", 1)
        text = re.sub(r"(?m)^Validator: PASS ", "Validator: FAIL ", text, count=1)
    lines = [text.rstrip("\n"), f"Scope: {' '.join((scope or 'none').split())}"]
    lines += [f"Note: {' '.join(n.split())}" for n in notes]
    if headed:
        lines.append("Note: heading-guard: verdict block must not contain Markdown heading lines (#506)")
    lines.append(f"Drift: {drift}")
    if (drift_from or "").strip():
        lines.append(f"Drift from notion: {' '.join(drift_from.split())}")
    return "\n".join(lines) + "\n"


def already_posted(issue, sha):
    p = run(["gh", "issue", "view", str(issue), "--json", "comments", "--jq", "[.comments[] | .body]"])
    if p.returncode != 0:
        return None
    try:
        bodies = json.loads(p.stdout or "[]")
    except json.JSONDecodeError:
        return None
    want = re.compile(r"<!--\s*validator:(pass|fail|blocked)\s+sha=%s\s*-->" % re.escape(sha))
    return any(want.search(b or "") for b in bodies)


def main(argv=None):
    ap = argparse.ArgumentParser(description="Post the validator verdict comment")
    ap.add_argument("--issue", type=int, required=True)
    ap.add_argument("--sha", required=True)
    ap.add_argument("--drift", required=True,
                    help="`none`, or what drifts from the original notion")
    ap.add_argument("--drift-from", default="",
                    help="owner-approved change from the notion, cited on a "
                         "`Drift from notion:` line after `Drift: none` (#506)")
    ap.add_argument("--scope", default="none",
                    help="`none`, or what was built but not asked")
    ap.add_argument("--note", action="append", default=[])
    ap.add_argument("--judge", action="append", default=[],
                    help="'Ck=MET: reason' or 'Ck=NOT MET: reason' for a criterion "
                         "judged by the validator (#1032)")
    ap.add_argument("--owner-decision", default="",
                    help="owner comment URL on this issue; allows a replacement "
                         "verdict for the same commit (#1032)")
    ap.add_argument("--dry-run", action="store_true", help="print, do not post")
    a = ap.parse_args(argv)

    run_dir, text = newest_verdict(a.issue, a.sha)
    if run_dir is None:
        print(json.dumps({"ok": False, "error": f"no verdict.md for {a.sha} under "
                                                f".agent/validation/issue-{a.issue}/; run validate.py first"}))
        return 1
    text, judge_errors = apply_judgments(text, a.judge)
    if judge_errors:
        print(json.dumps({"ok": False, "error": "; ".join(judge_errors)}))
        return 1
    notes = list(a.note)
    if a.owner_decision:
        problem = owner_decision_problem(a.issue, a.owner_decision)
        if problem:
            print(json.dumps({"ok": False, "error": problem}))
            return 1
        notes.append("replaces the earlier verdict for this commit on the owner's "
                     f"decision {a.owner_decision}")
    body = finish(text, a.drift, a.scope, notes, a.drift_from)
    block = SPAN.search(body).group(0)
    # LF on every platform, so the posted comment and the PR block match.
    with open(run_dir / "verdict-posted.md", "w", encoding="utf-8", newline="\n") as f:
        f.write(body)
    if a.dry_run:
        print(json.dumps({"ok": True, "posted": False, "body": body, "block": block}))
        return 0
    posted = already_posted(a.issue, a.sha)
    if posted is None:
        print(json.dumps({"ok": False, "error": f"cannot read issue #{a.issue} comments"}))
        return 1
    if posted and not a.owner_decision:
        print(json.dumps({"ok": False, "error": f"a verdict for {a.sha} is already on issue #{a.issue}; "
                                                "a replacement needs --owner-decision"}))
        return 1
    p = run(["gh", "issue", "comment", str(a.issue), "--body-file", str(run_dir / "verdict-posted.md")])
    if p.returncode != 0:
        print(json.dumps({"ok": False, "error": f"gh issue comment failed: {p.stderr.strip()[:300]}"}))
        return 1
    print(json.dumps({"ok": True, "posted": True, "url": (p.stdout or "").strip(),
                      "file": str(run_dir / "verdict-posted.md"), "block": block}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
