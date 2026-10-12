"""Verdict compare (R7): the PR's Validator-pass section must be byte-identical
to the Issue comment's verdict block.

Usage:
  python scripts/gates/compare_verdict.py --issue N --pr P

The Issue verdict comment holds the validator's own output pasted verbatim.
The verdict block is the canonical span from the `Validator:` line through
the closing `Drift: none` line (a fenced block containing a `Validator:` line
is accepted as a legacy form). The PR body holds the same block under a
`Validator pass` section. Any byte difference (after stripping one trailing
newline) fails. Missing blocks fail with a message. Verdict blocks must not
contain Markdown headings (#506): a block with a heading line is refused.
Newest non-superseded verdict comment wins (#506).
"""

import argparse
import json
import re
import subprocess
import sys

FENCE = re.compile(r"```(?:text)?\n(.*?)```", re.DOTALL)
SPAN = re.compile(r"(?m)^Validator:.*?$[\s\S]*?^Drift:\s*none\s*$")
HEADING = re.compile(r"(?m)^\s*#\s*\S")
SUPERSEDED = re.compile(r"<!--\s*validator:superseded\b")
SECTION_START = re.compile(r"^[#]{1,3}\s*validator pass\s*$",
                           re.IGNORECASE | re.MULTILINE)
SECTION_CUT = re.compile(r"^[#]{1,3}\s*validator pass\s*$([\s\S]*?)(?=^[#]{1,3}\s|\Z)",
                         re.IGNORECASE | re.MULTILINE)


def extract_span(text):
    # Line endings are not content: a body posted from a Windows file
    # carries CRLF, the same block pasted elsewhere may not (#533).
    text = (text or "").replace("\r\n", "\n")
    span = None
    f = FENCE.search(text)
    if f and "Validator:" in f.group(1):
        span = f.group(1).rstrip("\n")
    else:
        m = SPAN.search(text)
        if m:
            span = m.group(0).rstrip("\n")
    if span is None:
        return None, "no validator verdict block (fenced Validator block or Validator:-to-Drift:-none span)"
    if HEADING.search(span):
        return None, "verdict block contains a Markdown heading line (verdicts must not contain headings, #506)"
    return span, ""


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


def select_issue_comment(comments, sha):
    """Newest non-superseded comment carrying the pass marker for `sha` (#506)."""
    want = marker_for(sha)
    for c in reversed(comments or []):
        if not c or SUPERSEDED.search(c):
            continue
        if want.search(c):
            return c
    return None


def extract_pr_block(body):
    """Verdict block from a PR body string (pure; `pr_verdict` fetches)."""
    body = (body or "").strip()
    m = SECTION_CUT.search(body)
    section = m.group(1) if m else body
    span, err = extract_span(section or "")
    if span is not None:
        return span, ""
    # The section cut-off stops at the first `#` line, so a headed verdict
    # truncated mid-fence would report a bare miss: when the uncut
    # remainder holds a headed verdict block, name the heading rule.
    s = SECTION_START.search(body)
    if s:
        span2, err2 = extract_span(body[s.end():] or "")
        if span2 is None and "heading" in err2:
            return None, err2
    return None, err


def issue_verdict(issue, sha):
    p = run(["gh", "issue", "view", str(issue), "--json", "comments",
             "--jq", "[.comments[] | .body]"])
    if p.returncode != 0:
        return None, f"cannot read issue #{issue}: {p.stderr.strip()}"
    try:
        comments = json.loads(p.stdout or "[]")
    except json.JSONDecodeError as e:
        return None, f"cannot parse issue #{issue}: {e}"
    c = select_issue_comment(comments, sha)
    if c is None:
        return None, f"no validator verdict block for {sha} on issue #{issue}"
    span, err = extract_span(c)
    if span is None:
        return None, f"marker comment on issue #{issue} holds no verdict block ({err})"
    return span, ""


def pr_verdict(pr):
    p = run(["gh", "pr", "view", str(pr), "--json", "body", "--jq", ".body"])
    if p.returncode != 0:
        return None, f"cannot read PR #{pr}: {p.stderr.strip()}"
    span, err = extract_pr_block(p.stdout or "")
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
