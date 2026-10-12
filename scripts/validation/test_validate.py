"""Fixture harness for validate.py criteria and verdict (issue #533, T2/T3).

No cargo, no network: only the pure helpers. Pins that every
letter-prefixed criterion (C1, R1, ...) is read and counted, that the
criteria-shape refusal (issue #92) covers every prefix too, and that the
verdict validate.py writes and post_verdict.py finishes passes the merge
gate and the verdict compare without hand edits.

Usage:
  python scripts/validation/test_validate.py
Exit 0 when every fixture behaves; non-zero naming the first failure.
"""

import os
import sys
import tempfile
from pathlib import Path

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "..", "gates"))

from compare_verdict import extract_span  # noqa: E402
from merge_gate import MARKER, verdict_problems  # noqa: E402
from post_verdict import finish  # noqa: E402
from validate import check_criteria_shape, parse_criteria, verdict_text  # noqa: E402

REQS = """# Requirements (CONFIRMED) - issue #1

- [ ] C1: The planet shows (verify: contains planet_ok ... ok in cargo-test.log)
- [ ] C2: The ground holds (verify: contains ground_ok ... ok in cargo-test.log)
- [ ] R1: By default split once (verify: contains split_once ... ok in cargo-test.log)
- [ ] R12: Brighter clusters (verify: contains brighter ... ok in cargo-test.log)
- [x] C9: a ticked line is not a criterion
- [ ] note: a lower-case id is not a criterion
Prose mentioning C3: is not a criterion either.
"""


def check(name, got, want):
    if got != want:
        print(f"FAIL {name}: got {got!r}, want {want!r}")
        return False
    print(f"ok {name}")
    return True


def main():
    ok = True
    with tempfile.TemporaryDirectory() as d:
        p = Path(d) / "requirements.md"
        p.write_text(REQS, encoding="utf-8")
        crits, unconfirmed = parse_criteria(p)
    ids = [c.split(":", 1)[0] for c in crits]
    ok &= check("every-prefix-counted", ids, ["C1", "C2", "R1", "R12"])
    ok &= check("criterion-text-kept", crits[2],
                "R1: By default split once (verify: contains split_once ... ok in cargo-test.log)")
    ok &= check("confirmed", unconfirmed, False)

    hits = check_criteria_shape("- [ ] R3: the ci green at the head\n"
                                "- [ ] C1: fine (verify: contains x in cargo-test.log)\n")
    ok &= check("shape-refusal-covers-r", [h[0] for h in hits], ["R3"])

    # The verdict written by validate.py and finished by post_verdict.py
    # passes merge_gate's checks and compare_verdict's span as is (#533).
    sha = "a" * 40
    rows = [{"criterion": c, "status": "MET", "evidence": "found"} for c in crits]
    text = verdict_text("PASS", sha, 1, "20261012T000000Z", 60,
                        ["cargo test --locked --workspace: rc=0 OK"], [], [], rows)
    body = finish(text, "none", "none", ["timed run"])
    ok &= check("verdict-marker", MARKER.search(body).group(1), sha)
    ok &= check("verdict-passes-merge-gate", verdict_problems(body), [])
    span, _ = extract_span(body)
    ok &= check("verdict-span-closes-on-drift",
                (span.startswith("Validator: PASS for " + sha), span.endswith("Drift: none")),
                (True, True))
    crlf, _ = extract_span(body.replace("\n", "\r\n") + "\r\n")
    ok &= check("verdict-span-ignores-crlf", crlf, span)
    drifted = finish(text, "spec adds a menu the notion never asked for", "none", [])
    ok &= check("drift-turns-fail",
                (MARKER.search(drifted), "Validator: FAIL" in drifted), (None, True))
    partial = verdict_text("FAIL", sha, 1, "t", 1, [], [],
                           [], rows[:3] + [{"criterion": "R12: x", "status": "NOT MET", "evidence": "-"}])
    ok &= check("partial-requirements-counted",
                ("Requirements: MET 3/4" in partial, partial.startswith("<!-- validator:fail")),
                (True, True))
    # #506: a heading line smuggled into criterion words fails the verdict
    # instead of posting a block the merge gate will never accept.
    headed_rows = rows[:1] + [{"criterion": "C9: x\n# smuggled heading",
                               "status": "MET", "evidence": "found"}]
    headed = verdict_text("PASS", sha, 1, "t", 1, [], [], [], headed_rows)
    ok &= check("heading-guard-turns-fail",
                (headed.startswith("<!-- validator:fail"),
                 "Validator: FAIL" in headed, "heading-guard" in headed),
                (True, True, True))
    gspan, gerr = extract_span(finish(headed, "none", "none", []))
    ok &= check("heading-guard-block-still-refused",
                (gspan, "heading" in gerr), (None, True))
    headed_body = finish(text.replace("Checked:", "Checked:\n# smuggled", 1),
                         "none", "none", [])
    ok &= check("finish-heading-turns-fail",
                ("Validator: FAIL" in headed_body, "heading-guard" in headed_body),
                (True, True))
    # #506 T4: --drift-from cites the owner approval after `Drift: none`;
    # the gates keep reading the same block as the clean verdict.
    cited_body = finish(text, "none", "none", [],
                        "Quit popup moved, owner approved in spec")
    ok &= check("drift-from-cited",
                ("Drift: none" in cited_body,
                 "Drift from notion: Quit popup moved" in cited_body),
                (True, True))
    ok &= check("drift-from-passes-merge-gate",
                verdict_problems(cited_body), [])
    cspan, _ = extract_span(cited_body)
    clean2, _ = extract_span(finish(text, "none", "none", []))
    ok &= check("drift-from-span-matches-clean", cspan, clean2)

    if not ok:
        print("fixtures: FAIL")
        return 1
    print("fixtures: all ok (validate criteria parsing)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
