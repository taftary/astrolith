"""Spec gate: exits 0 only when the Issue holds everything implementation needs.

Usage:
  python scripts/gates/spec_gate.py --issue N

Requires on the Issue:
  1. An owner approval comment: trimmed body exactly `approved`
     (or `owner-test-only approved`, which also waives rule 3).
  2. A spec comment containing all sections: goal, non-goals,
     acceptance criteria, test plan, testability needs.
  3. A preflight pass comment (or the owner-test-only approval).

Exit 0 when all hold; non-zero printing exactly what is missing.
The `implementation` skill calls this as step 1 and stops on non-zero.
"""

import argparse
import json
import re
import subprocess
import sys

SECTIONS = ["goal", "non-goals", "acceptance criteria", "test plan",
            "testability needs"]


def gh_issue(issue):
    p = subprocess.run(
        ["gh", "issue", "view", str(issue), "--json", "title,body,comments"],
        capture_output=True, text=True, encoding="utf-8", errors="replace")
    if p.returncode != 0:
        print(f"spec_gate: cannot read issue #{issue}: {p.stderr.strip()}")
        return None
    try:
        return json.loads(p.stdout or "{}")
    except json.JSONDecodeError as e:
        print(f"spec_gate: cannot parse issue #{issue}: {e}")
        return None


def main(argv=None):
    ap = argparse.ArgumentParser(description="Spec gate for a parent Issue")
    ap.add_argument("--issue", type=int, required=True)
    a = ap.parse_args(argv)

    data = gh_issue(a.issue)
    if data is None:
        return 2
    comments = [c.get("body", "") or "" for c in data.get("comments", []) or []]
    missing = []

    owner_test_only = any(c.strip() == "owner-test-only approved" for c in comments)
    approved = any(c.strip() == "approved" for c in comments)
    if not approved and not owner_test_only:
        missing.append("owner approval comment (`approved` or `owner-test-only approved`)")

    bodies = [(data.get("body", "") or "")] + comments
    spec_ok = False
    for b in bodies:
        low = b.lower()
        if all(s in low for s in SECTIONS):
            spec_ok = True
            break
    if not spec_ok:
        missing.append("spec comment with sections: " + ", ".join(SECTIONS))

    if not owner_test_only:
        preflight = any(re.search(r"preflight[\s\S]{0,40}pass", c, re.IGNORECASE)
                        for c in comments)
        if not preflight:
            missing.append("preflight pass comment")

    if missing:
        print(f"spec_gate: BLOCKED for issue #{a.issue}; missing:")
        for m in missing:
            print(f"  - {m}")
        return 1
    kind = "owner-test-only" if owner_test_only else "approved"
    print(f"spec_gate: OPEN for issue #{a.issue} ({kind}, spec sections present"
          + (", preflight waived" if owner_test_only else ", preflight pass present") + ")")
    return 0


if __name__ == "__main__":
    sys.exit(main())
