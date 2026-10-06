"""Fixture harness for merge_gate verdict checks (issue #198, T5/AC4/AC5).

Canned verdict comments, no network: normal Visual lines pass the
degraded check untouched, the exact degraded wording passes, and any
other BLOCKED wording is refused. The SHA-match refusal and the
empty-body pass live in the same check family and are pinned here.

Usage:
  python scripts/gates/test_merge_gate_fixtures.py
Exit 0 when every fixture behaves; non-zero naming the first failure.
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from merge_gate import degraded_visual_ok  # noqa: E402
from merge_gate import AUTO_CLOSE, MARKER  # noqa: E402

HEAD = "b" * 40

NORMAL = (
    "<!-- validator:pass sha=" + HEAD + " -->\n"
    "Validator: PASS for " + HEAD + "\n"
    "Runtime: PASS\n"
    "Requirements: MET 12/12\n"
    "Drift: none\n"
    "\nVisual: none (11 frames identical to main)\n"
)

DEGRADED_GOOD = (
    "<!-- validator:pass sha=" + HEAD + " -->\n"
    "Validator: PASS for " + HEAD + "\n"
    "Runtime: PASS\n"
    "Requirements: MET 12/12\n"
    "Drift: none\n"
    "\nVisual: BLOCKED (infra: disk-full) \u2014 "
    "<no capturable byte changed: base aaaaaaa...head bbbbbbb: "
    "3 file(s) differ, none under crates/, Cargo.toml, or Cargo.lock>\n"
    "\n- infra cause: disk-full\n"
    "- capturable files changed: none (golden tests rc=0)\n"
    "- golden tests: PASS byte-identical\n"
)

DEGRADED_NO_GOLDEN = (
    DEGRADED_GOOD
    .replace("- capturable files changed: none (golden tests rc=0)\n",
             "- capturable files changed: none (not checked)\n")
    .replace("- golden tests: PASS byte-identical\n", "")
)

DEGRADED_NO_CAUSE = (
    "<!-- validator:pass sha=" + HEAD + " -->\n"
    "Validator: PASS for " + HEAD + "\n"
    "Runtime: PASS\n"
    "Requirements: MET 12/12\n"
    "Drift: none\n"
    "\nVisual: BLOCKED disk full, no pictures\n"
)


def check(name, got, want):
    if got != want:
        print(f"fixture {name}: FAIL (got {got!r}, want {want!r})")
        return False
    print(f"fixture {name}: ok")
    return True


def main():
    ok = True
    ok &= check("normal-untouched", degraded_visual_ok(NORMAL), None)
    ok &= check("degraded-good", degraded_visual_ok(DEGRADED_GOOD), True)
    ok &= check("degraded-no-golden-refused",
                degraded_visual_ok(DEGRADED_NO_GOLDEN), False)
    ok &= check("degraded-bare-refused",
                degraded_visual_ok(DEGRADED_NO_CAUSE), None)
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
