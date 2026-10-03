"""Fixture harness for spec_gate version binding (issue #91, sub-issue #109).

Canned comment lists, no network: superseded approval stays BLOCKED naming
the newer version, approval after the newest spec is OPEN, and sections that
live only in an old version stay BLOCKED.

Usage:
  python scripts/gates/test_spec_gate_fixtures.py
Exit 0 when every fixture behaves; non-zero naming the first failure.
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from spec_gate import (  # noqa: E402
    approvals_after,
    find_newest_spec,
    spec_has_sections,
)

FULL_SPEC = """## Spec v3 — Thing

### Goal
goal text

### Non-goals
none

### Acceptance criteria
- AC1

### Test plan
steps

### Testability needs
none
"""

def check(name, got, want):
    if got != want:
        print(f"fixture {name}: FAIL (got {got!r}, want {want!r})")
        return False
    print(f"fixture {name}: ok")
    return True


def main():
    ok = True

    # 1. Superseded approval: v3 approved, then v4 posted without approval.
    comments = [FULL_SPEC, "approved",
                FULL_SPEC.replace("Spec v3", "Spec v4")]
    idx, ver = find_newest_spec(comments)
    ok &= check("superseded-newest", (idx, ver), (2, 4))
    votes = approvals_after(comments, idx)
    ok &= check("superseded-no-approval", votes,
                {"approved": False, "owner_test_only": False})
    ok &= check("superseded-sections", spec_has_sections(comments[idx]), True)

    # 2. Approval after the newest spec.
    comments = [FULL_SPEC, "approved",
                FULL_SPEC.replace("Spec v3", "Spec v4"), "approved"]
    idx, ver = find_newest_spec(comments)
    ok &= check("fresh-newest", (idx, ver), (2, 4))
    votes = approvals_after(comments, idx)
    ok &= check("fresh-approval", votes,
                {"approved": True, "owner_test_only": False})

    # 3. Sections only in an old version: newest spec comment lacks them.
    comments = [FULL_SPEC, "## Spec v4 — Thing\n\nJust an idea, no sections."]
    idx, ver = find_newest_spec(comments)
    ok &= check("thin-newest", (idx, ver), (1, 4))
    ok &= check("thin-sections", spec_has_sections(comments[idx]), False)
    ok &= check("thin-old-kept", spec_has_sections(comments[0]), True)

    # 4. No spec version anywhere.
    comments = ["looks good", "approved"]
    ok &= check("nospec-newest", find_newest_spec(comments), (None, None))
    ok &= check("nospec-votes", approvals_after(comments, None),
                {"approved": False, "owner_test_only": False})

    # 5. owner-test-only after the newest spec waives like approved.
    comments = [FULL_SPEC, "owner-test-only approved"]
    idx, ver = find_newest_spec(comments)
    ok &= check("oto-newest", (idx, ver), (0, 3))
    ok &= check("oto-vote", approvals_after(comments, idx),
                {"approved": False, "owner_test_only": True})

    if not ok:
        print("fixtures: FAIL")
        return 1
    print("fixtures: all ok (superseded stays BLOCKED)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
