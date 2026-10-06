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
    capability_rows,
    find_newest_spec,
    preflight_decision,
    preflight_satisfied,
    spec_has_sections,
    validate_skip_record,
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

    # 6. Conditional preflight (issue #198, AC8): skip only when every
    #    need is covered by a proven registry row; a bare skip is refused.
    caps = (
        "# Validator Capabilities\n\n"
        "| capability | how proven | Issue | date |\n"
        "|---|---|---|---|\n"
        "| run the headless app check | preflight on main | #63 | 2026-10-01 |\n"
        "| publish pictures to the evidence release | live run | #134 | 2026-10-04 |\n"
        "\n**Traps that cost time.**\n\n"
        "| refused | note |\n"
        "| step log text without admin | 403 | #87 |\n"
    )
    ok &= check("preflight-cap-rows", len(capability_rows(caps)), 2)
    decision, uncovered = preflight_decision(
        ["headless app check", "publish pictures"], caps)
    ok &= check("preflight-all-proven-skips", (decision, uncovered), ("skip", []))
    decision, uncovered = preflight_decision(
        ["headless app check", "a signed-in test account"], caps)
    ok &= check("preflight-gap-forces", decision, "force")
    ok &= check("preflight-gap-names-need", uncovered, ["a signed-in test account"])
    ok &= check("preflight-empty-needs-forces",
                preflight_decision([], caps)[0], "force")
    good_skip = ("preflight skip: every need is covered by "
                 "docs/validator-capabilities.md — run the headless app check "
                 "and publish pictures to the evidence release.")
    ok &= check("preflight-skip-valid",
                validate_skip_record(good_skip, caps), (True, ""))
    bare_skip = "preflight skip: nothing was unknown, no need to run anything."
    good, reason = validate_skip_record(bare_skip, caps)
    ok &= check("preflight-bare-skip-refused", good, False)
    ok &= check("preflight-bare-skip-reason", reason,
                "skip names no capabilities file")
    nameless_skip = ("preflight skip per docs/validator-capabilities.md "
                     "with no entry quoted.")
    ok &= check("preflight-nameless-skip-refused",
                validate_skip_record(nameless_skip, caps)[0], False)
    ok &= check("preflight-pass-still-counts",
                preflight_satisfied(["preflight result: PASS on env"], caps),
                (True, "pass"))
    ok &= check("preflight-valid-skip-counts",
                preflight_satisfied([good_skip], caps), (True, "skip"))
    ok &= check("preflight-bare-skip-no-gate",
                preflight_satisfied([bare_skip], caps), (False, ""))

    if not ok:
        print("fixtures: FAIL")
        return 1
    print("fixtures: all ok (superseded stays BLOCKED)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
