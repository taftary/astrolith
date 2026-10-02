"""Verdict compare (R7): the PR's Validator-pass section must be byte-identical
to the Issue comment's verdict block.

Usage:
  python scripts/gates/compare_verdict.py --issue N --pr P

The Issue verdict comment holds the validator's own output pasted verbatim.
The verdict block is the canonical span from the `Validator:` line through
the closing `Drift: none` line (a fenced block containing a `Validator:` line
is accepted as a legacy form). The PR body holds the same block under a
`Validator pass` section. Any byte difference (after stripping one trailing
newline) fails. Missing blocks fail with a message.
"""

import argparse
import json
import re
import subprocess
import sys

FENCE = re.compile(r"```(?:text)?\n(.*?)```", re.DOTALL)
SPAN = re.compile(r"(?m)^Validator:.*?$[\s\S]*?^Drift:\s*none\s*$")


def extract_span(text):
    f = FENCE.search(text or "")
    if f and "Validator:" in f.group(1):
        return f.group(1).rstrip("\n"), ""
    m = SPAN.search(text or "")
    if m:
        return m.group(0).rstrip("\n"), ""
    return None, "no validator verdict block (fenced Validator block or Validator:-to-Drift:-none span)"


def run(cmd):
    p = subprocess.run(cmd, capture_output=True, text=True,
                       encoding="utf-8", errors="replace")
    return p


def marker_for(sha):
    return re.compile(r"<!--\s*validator:pass\s+sha=%s\s*-->" % re.escape(sha))


def pr_head_sha(pr):
    p = run(["gh", "pr", "view", str(pr), "--json", "headRefOid", "--jq", ".headRefOid"])
    if p.returncode != 0:
        return None, f"cannot read PR #{pr}: {p.stderr.strip()}"
    sha = (p.stdout or "").strip()
    if not re.fullmatch(r"[0-9a-f]{40}", sha):
        return None, f"PR #{pr} head is not a full SHA: {sha!r}"
    return sha, ""


def issue_verdict(issue, sha):
    p = run(["gh", "issue", "view", str(issue), "--json", "comments",
             "--jq", "[.comments[] | .body]"])
    if p.returncode != 0:
        return None, f"cannot read issue #{issue}: {p.stderr.strip()}"
    try:
        comments = json.loads(p.stdout or "[]")
    except json.JSONDecodeError as e:
        return None, f"cannot parse issue #{issue}: {e}"
    want = marker_for(sha)
    for c in reversed(comments):
        if want.search(c or ""):
            span, err = extract_span(c or "")
            if span is None:
                return None, f"marker comment on issue #{issue} holds no verdict block"
            return span, ""
    return None, f"no validator verdict block for {sha} on issue #{issue}"


def pr_verdict(pr):
    p = run(["gh", "pr", "view", str(pr), "--json", "body", "--jq", ".body"])
    if p.returncode != 0:
        return None, f"cannot read PR #{pr}: {p.stderr.strip()}"
    body = (p.stdout or "").strip()
    m = re.search(r"^[#]{1,3}\s*validator pass\s*$([\s\S]*?)(?=^[#]{1,3}\s|\Z)",
                  body, re.IGNORECASE | re.MULTILINE)
    section = m.group(1) if m else body
    span, err = extract_span(section or "")
    if span is None:
        return None, f"no validator verdict block in PR #{pr} body ({err})"
    return span, ""


def main(argv=None):
    ap = argparse.ArgumentParser(description="Compare PR verdict block to Issue verdict")
    ap.add_argument("--issue", type=int, required=True)
    ap.add_argument("--pr", type=int, required=True)
    a = ap.parse_args(argv)

    sha, err = pr_head_sha(a.pr)
    if sha is None:
        print(f"compare_verdict: DIFFER: {err}")
        return 1
    want, err = issue_verdict(a.issue, sha)
    if want is None:
        print(f"compare_verdict: DIFFER: {err}")
        return 1
    got, err = pr_verdict(a.pr)
    if got is None:
        print(f"compare_verdict: DIFFER: {err}")
        return 1
    if want != got:
        print(f"compare_verdict: DIFFER for issue #{a.issue} PR #{a.pr}: "
              f"PR block is not byte-identical to the Issue verdict block "
              f"(issue {len(want)} chars, PR {len(got)} chars)")
        return 1
    print(f"compare_verdict: IDENTICAL for issue #{a.issue} PR #{a.pr} ({len(want)} chars)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
