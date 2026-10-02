"""Verdict compare (R7): the PR's Validator-pass section must be byte-identical
to the Issue comment's verdict block.

Usage:
  python scripts/gates/compare_verdict.py --issue N --pr P

The Issue verdict comment holds the validator's own output pasted verbatim
(a fenced block containing a `Validator:` line). The PR body holds the same
block under a `Validator pass` section. Any byte difference (after stripping
one trailing newline) fails. Missing blocks fail with a message.
"""

import argparse
import json
import re
import subprocess
import sys

FENCE = re.compile(r"```(?:text)?\n(.*?)```", re.DOTALL)


def run(cmd):
    p = subprocess.run(cmd, capture_output=True, text=True,
                       encoding="utf-8", errors="replace")
    return p


def issue_verdict(issue):
    p = run(["gh", "issue", "view", str(issue), "--json", "comments",
             "--jq", "[.comments[] | .body]"])
    if p.returncode != 0:
        return None, f"cannot read issue #{issue}: {p.stderr.strip()}"
    try:
        comments = json.loads(p.stdout or "[]")
    except json.JSONDecodeError as e:
        return None, f"cannot parse issue #{issue}: {e}"
    blocks = []
    for c in comments:
        for m in FENCE.finditer(c or ""):
            if "Validator:" in m.group(1):
                blocks.append(m.group(1).rstrip("\n"))
    if not blocks:
        return None, f"no validator verdict block on issue #{issue}"
    return blocks[-1], ""


def pr_verdict(pr):
    p = run(["gh", "pr", "view", str(pr), "--json", "body", "--jq", ".body"])
    if p.returncode != 0:
        return None, f"cannot read PR #{pr}: {p.stderr.strip()}"
    body = (p.stdout or "").strip()
    m = re.search(r"^[#]{1,3}\s*validator pass\s*$([\s\S]*?)(?=^[#]{1,3}\s|\Z)",
                  body, re.IGNORECASE | re.MULTILINE)
    section = m.group(1) if m else body
    f = FENCE.search(section or "")
    if not f or "Validator:" not in f.group(1):
        return None, f"no validator verdict block in PR #{pr} body"
    return f.group(1).rstrip("\n"), ""


def main(argv=None):
    ap = argparse.ArgumentParser(description="Compare PR verdict block to Issue verdict")
    ap.add_argument("--issue", type=int, required=True)
    ap.add_argument("--pr", type=int, required=True)
    a = ap.parse_args(argv)

    want, err = issue_verdict(a.issue)
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
