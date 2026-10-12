"""Fetch every validator input in one call (issue #533).

Usage:
  python scripts/validation/fetch_inputs.py --issue N --comment <id-or-url> [--comment ...]
      [--body] [--pr P]

Prints each named Issue comment (the verbatim notion, the approved Spec, the
owner's answers) under a header with its URL, plus the Issue body with
`--body` and the PR head SHA and CI state with `--pr`. A comment that is not
on the Issue is reported as missing, so the validator can report BLOCKED
naming it. Read-only.
"""
import argparse
import json
import re
import subprocess
import sys


def run(cmd):
    """Run one gh command; tests replace this with a fake runner."""
    return subprocess.run(cmd, capture_output=True, text=True,
                          encoding="utf-8", errors="replace")


def comment_id(ref):
    """Numeric id from `6118342710` or `...#issuecomment-6118342710`."""
    m = re.search(r"(\d+)\s*$", ref or "")
    return int(m.group(1)) if m else None


def main(argv=None):
    ap = argparse.ArgumentParser(description="Fetch validator inputs in one call")
    ap.add_argument("--issue", type=int, required=True)
    ap.add_argument("--comment", action="append", default=[])
    ap.add_argument("--body", action="store_true", help="also print the Issue body")
    ap.add_argument("--pr", type=int, default=None)
    a = ap.parse_args(argv)
    # Issue text holds any character; a Windows console defaults to cp1252
    # and would crash on the first one it cannot encode.
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")

    p = run(["gh", "api", "--paginate", f"repos/{{owner}}/{{repo}}/issues/{a.issue}/comments",
             "--jq", ".[] | {id, html_url, user: .user.login, body}"])
    if p.returncode != 0:
        print(f"MISSING: cannot read issue #{a.issue} comments: {p.stderr.strip()[:300]}")
        return 1
    by_id = {}
    for line in (p.stdout or "").splitlines():
        if line.strip():
            c = json.loads(line)
            by_id[c["id"]] = c
    missing = []
    if a.body:
        b = run(["gh", "issue", "view", str(a.issue), "--json", "title,body,url"])
        if b.returncode == 0:
            d = json.loads(b.stdout or "{}")
            print(f"=== issue #{a.issue} body: {d.get('title', '')} ({d.get('url', '')}) ===")
            print(d.get("body", "") or "(empty)")
            print()
        else:
            missing.append(f"issue #{a.issue} body")
    for ref in a.comment:
        cid = comment_id(ref)
        c = by_id.get(cid)
        if c is None:
            missing.append(ref)
            continue
        print(f"=== comment {cid} by {c['user']} ({c['html_url']}) ===")
        print(c["body"] or "(empty)")
        print()
    if a.pr is not None:
        q = run(["gh", "pr", "view", str(a.pr), "--json", "headRefOid,headRefName,state,statusCheckRollup",
                 "--jq", "{head: .headRefOid, branch: .headRefName, state, "
                         "checks: [.statusCheckRollup[] | {name, conclusion}]}"])
        if q.returncode == 0:
            print(f"=== PR #{a.pr} ===")
            print(q.stdout.strip())
        else:
            missing.append(f"PR #{a.pr}")
    if missing:
        print("MISSING: " + ", ".join(missing))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
