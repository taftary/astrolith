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

from merge_gate import (  # noqa: E402
    AUTO_CLOSE,
    MARKER,
    SUPERSEDED,
    select_verdict,
    touch_problems,
    verdict_problems,
)

HEAD = "b" * 40
OTHER = "c" * 40

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
    # #506: newest non-superseded verdict wins at selection level.
    old = NORMAL.replace("Requirements: MET 12/12", "Requirements: MET 10/12")
    new = NORMAL
    ok &= check("newest-wins", select_verdict([old, new], HEAD), new)
    ok &= check("single-wins", select_verdict([new], HEAD), new)
    ok &= check("stale-sha-no-verdict",
                select_verdict([new], OTHER), None)
    sup_new = "<!-- validator:superseded sha=" + HEAD + " -->\n" + new
    ok &= check("superseded-newest-ignored",
                select_verdict([old, sup_new], HEAD), old)
    sup_old = "<!-- validator:superseded sha=" + HEAD + " -->\n" + old
    ok &= check("superseded-oldest-ignored",
                select_verdict([sup_old, new], HEAD), new)
    ok &= check("superseded-marker-detected",
                SUPERSEDED.search(sup_new) is not None, True)
    ok &= check("pass-marker-not-superseded",
                SUPERSEDED.search(new) is not None, False)
    # #506 T4: an owner-approved drift citation rides after `Drift: none`
    # without blocking, while unapproved drift still fails.
    cited = NORMAL + "Drift from notion: Quit popup moved, owner approved in spec\n"
    ok &= check("approved-drift-citation-passes", verdict_problems(cited), [])
    ok &= check("unapproved-drift-refused",
                len(verdict_problems(NORMAL.replace("Drift: none",
                                                    "Drift: extra menu"))), 1)

    # In-game Settings and Help touched under a None spec (#1055).
    none_spec = ("## Spec v1\n### Settings candidates\nNone: nothing a player "
                 "sees changes.\n\n### Guides candidates\nNone: no Help text "
                 "mentions it.\n")
    listed = ("## Spec v1\n### Settings candidates\n- **S1 Mode** Add to "
              "Settings?\nAnswers: S1 yes\n\n### Guides candidates\nNone: no "
              "Help text mentions it.\n")
    new, old = "2026-10-29T08:00:00Z", "2026-10-20T08:00:00Z"
    setting = ["crates/universe-render/src/settings_quality_ui.rs"]
    probs = touch_problems(setting, none_spec, new)
    ok &= check("touch-settings-under-none-refused", len(probs), 1)
    ok &= check("touch-message-says-owner", "back to the owner" in probs[0], True)
    ok &= check("touch-listed-accepted", touch_problems(setting, listed, new), [])
    ok &= check("touch-tests-only-accepted",
                touch_problems(["crates/universe-core/src/settings_tests.rs"],
                               none_spec, new), [])
    ok &= check("touch-old-spec-accepted", touch_problems(setting, none_spec, old), [])
    ok &= check("touch-guides-under-none-refused",
                len(touch_problems(["crates/universe-core/src/guides.rs"],
                                   listed, new)), 1)
    ok &= check("touch-other-code-accepted",
                touch_problems(["crates/universe-render/src/forms.rs"],
                               none_spec, new), [])

    if not ok:
        print("fixtures: FAIL")
        return 1
    print("fixtures: all ok (merge gate verdict checks)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
