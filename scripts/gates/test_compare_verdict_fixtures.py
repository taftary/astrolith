"""Fixture harness for compare_verdict heading + selection checks (issue #506).

Canned verdict text, no network. Pins that a verdict block holding a
Markdown heading line is refused, that the newest non-superseded Issue
comment wins, and that a headed PR section names the heading rule instead
of reporting a bare miss.

Usage:
  python scripts/gates/test_compare_verdict_fixtures.py
Exit 0 when every fixture behaves; non-zero naming the first failure.
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from compare_verdict import (  # noqa: E402
    extract_pr_block,
    extract_span,
    marker_for,
    select_issue_comment,
)

HEAD = "d" * 40

CLEAN = (
    "<!-- validator:pass sha=" + HEAD + " -->\n"
    "Validator: PASS for " + HEAD + "\n"
    "Checked: cargo test; headless verify twice\n"
    "Evidence: .agent/validation/issue-506/ts/\n"
    "Runtime: PASS\n"
    "Requirements: MET 2/2\n"
    "Drift: none\n"
)

HEADED = CLEAN.replace("Checked: cargo test;",
                       "### Checked\ncargo test;")
FENCED_HEADED = ("<!-- validator:pass sha=" + HEAD + " -->\n"
                 "```text\n"
                 "Validator: PASS for " + HEAD + "\n"
                 "### Checked\n"
                 "Runtime: PASS\n"
                 "Requirements: MET 1/1\n"
                 "Drift: none\n"
                 "```\n")

PR_CLEAN = ("## Summary\nnotes\n\n## Validator pass\n" + CLEAN
            + "\n## Test plan\nsteps\n")
PR_HEADED_FENCED = ("## Validator pass\n" + FENCED_HEADED
                    + "\n## Test plan\nsteps\n")


def check(name, got, want):
    if got != want:
        print(f"fixture {name}: FAIL (got {got!r}, want {want!r})")
        return False
    print(f"fixture {name}: ok")
    return True


def main():
    ok = True
    span, _ = extract_span(CLEAN)
    ok &= check("clean-span-found", span is not None, True)
    span, err = extract_span(HEADED)
    ok &= check("unfenced-heading-refused", span, None)
    ok &= check("unfenced-heading-named", "heading" in err, True)
    span, err = extract_span(FENCED_HEADED)
    ok &= check("fenced-heading-refused", span, None)
    ok &= check("fenced-heading-named", "heading" in err, True)
    ok &= check("dash-line-not-heading",
                extract_span(CLEAN.replace("Runtime: PASS",
                                           "Runtime: PASS\n- note"))[0] is not None,
                True)
    old = CLEAN.replace("MET 2/2", "MET 1/2")
    ok &= check("issue-newest-wins",
                select_issue_comment([old, CLEAN], HEAD), CLEAN)
    sup = "<!-- validator:superseded sha=" + HEAD + " -->\n" + CLEAN
    ok &= check("issue-superseded-ignored",
                select_issue_comment([old, sup], HEAD), old)
    ok &= check("issue-stale-none",
                select_issue_comment([CLEAN], "e" * 40), None)
    ok &= check("marker-matches",
                marker_for(HEAD).search(CLEAN) is not None, True)
    span, _ = extract_pr_block(PR_CLEAN)
    ok &= check("pr-clean-found", span is not None, True)
    ok &= check("pr-clean-matches-issue", span,
                extract_span(CLEAN)[0])
    span, err = extract_pr_block(PR_HEADED_FENCED)
    ok &= check("pr-headed-refused", span, None)
    ok &= check("pr-headed-named", "heading" in err, True)
    if not ok:
        print("fixtures: FAIL")
        return 1
    print("fixtures: all ok (compare verdict checks)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
