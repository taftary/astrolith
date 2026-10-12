"""Fixture harness for merge_gate verdict checks (issues #198, #533).

Canned verdict comments, no network. Pins that a verdict passes on
`Runtime: PASS`, an equal `Requirements: MET n/n`, and a `Drift: none`
line, with no `Visual:` picture line needed (#533), and that each missing
or wrong piece is refused. The SHA-match refusal and the empty-body pass
live in the same check family and are pinned here.

Usage:
  python scripts/gates/test_merge_gate_fixtures.py
Exit 0 when every fixture behaves; non-zero naming the first failure.
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from merge_gate import AUTO_CLOSE, MARKER, verdict_problems  # noqa: E402

HEAD = "b" * 40

NORMAL = (
    "<!-- validator:pass sha=" + HEAD + " -->\n"
    "Validator: PASS for " + HEAD + "\n"
    "Runtime: PASS\n"
    "Requirements: MET 12/12\n"
    "Drift: none\n"
)


def check(name, got, want):
    if got != want:
        print(f"fixture {name}: FAIL (got {got!r}, want {want!r})")
        return False
    print(f"fixture {name}: ok")
    return True


def main():
    ok = True
    ok &= check("pass-without-visual", verdict_problems(NORMAL), [])
    ok &= check("runtime-fail-refused",
                len(verdict_problems(NORMAL.replace("Runtime: PASS", "Runtime: FAIL"))), 1)
    ok &= check("partial-requirements-refused",
                len(verdict_problems(NORMAL.replace("MET 12/12", "MET 11/12"))), 1)
    ok &= check("zero-requirements-refused",
                len(verdict_problems(NORMAL.replace("MET 12/12", "MET 0/0"))), 1)
    ok &= check("n-of-n-accepted",
                verdict_problems(NORMAL.replace("MET 12/12", "MET 12 of 12")), [])
    ok &= check("drift-with-notes-refused",
                len(verdict_problems(NORMAL.replace("Drift: none", "Drift: none beyond notes"))), 1)
    # AC5: an empty PR body carries no auto-close keyword (the verdict
    # lives on the Issue), while the marker binds the verdict to the
    # exact head SHA and a stale SHA never matches.
    ok &= check("empty-body-no-autoclose", AUTO_CLOSE.findall(""), [])
    ok &= check("empty-body-no-autoclose-plain",
                AUTO_CLOSE.findall("Related to #198"), [])
    m = MARKER.search(NORMAL)
    ok &= check("marker-matches-head",
                m.group(1) if m else None, HEAD)
    ok &= check("marker-rejects-stale",
                MARKER.search(NORMAL.replace(HEAD, "c" * 40)) is not None
                and MARKER.search(
                    NORMAL.replace(HEAD, "c" * 40)).group(1) == HEAD,
                False)
    if not ok:
        print("fixtures: FAIL")
        return 1
    print("fixtures: all ok (merge gate verdict checks)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
