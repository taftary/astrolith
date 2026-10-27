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
from validate import (  # noqa: E402
    check_criteria_shape,
    extract_step,
    test_lines,
    parse_criteria,
    pick_ci_run,
    verdict_text,
)

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


CI_LOG = (
    "ci\tCargo clippy\t2026-10-20T07:25:34.1Z clippy is clean\n"
    "ci\tCargo test\t\ufeff2026-10-20T07:30:00.1234567Z running 3 tests\n"
    "ci\tCargo test\t2026-10-20T07:30:01.0Z test planet_ok ... ok\n"
    "ci\tCargo test\t2026-10-20T07:30:02.0Z test result: ok. 3 passed; 0 failed\n"
    "ci\tPython gate fixtures\t2026-10-20T07:31:00.0Z all ok\n"
)


CI_LOG_UNKNOWN = (
    "ci\tUNKNOWN STEP\t2026-10-20T07:29:59.0Z ##[group]Run cargo clippy\n"
    "ci\tUNKNOWN STEP\t2026-10-20T07:29:59.5Z clippy is clean\n"
    "ci\tUNKNOWN STEP\t2026-10-20T07:30:00.0Z ##[group]Run cargo test --locked --workspace -- --include-ignored\n"
    "ci\tUNKNOWN STEP\t2026-10-20T07:30:01.0Z test planet_ok ... ok\n"
    "ci\tUNKNOWN STEP\t2026-10-20T07:30:02.0Z ##[group]Run python scripts/x.py\n"
    "ci\tUNKNOWN STEP\t2026-10-20T07:30:03.0Z all ok\n"
)


def reuses_ci_results():
    """test_validate_reuses_ci_results: CI's run is picked only when it is
    for this exact commit and passed, and only its `Cargo test` step is
    kept, without timestamps."""
    sha = "a" * 40
    runs = [
        {"databaseId": 1, "headSha": "b" * 40, "status": "completed", "conclusion": "success"},
        {"databaseId": 2, "headSha": sha, "status": "completed", "conclusion": "failure"},
        {"databaseId": 3, "headSha": sha, "status": "in_progress", "conclusion": ""},
        {"databaseId": 4, "headSha": sha, "status": "completed", "conclusion": "success"},
    ]
    ok = check("ci-run-picked", pick_ci_run(runs, sha), 4)
    ok &= check("ci-run-none-when-failed", pick_ci_run(runs[:3], sha), None)
    ok &= check("ci-run-none-when-empty", pick_ci_run([], sha), None)
    step = extract_step(CI_LOG, "Cargo test")
    ok &= check("ci-step-lines", step.splitlines(),
                ["running 3 tests", "test planet_ok ... ok",
                 "test result: ok. 3 passed; 0 failed"])
    ok &= check("ci-step-absent", extract_step(CI_LOG, "Golden tests"), "")
    ok &= check("ci-step-by-marker",
                extract_step(CI_LOG_UNKNOWN, "Cargo test").splitlines(),
                ["##[group]Run cargo test --locked --workspace -- --include-ignored",
                 "test planet_ok ... ok"])
    noisy = "\x1b[1m\x1b[92m   Compiling\x1b[0m const_panic v0.2.17\ntest planet_ok ... ok\n    Finished `test` profile\ntest result: ok."
    noisy += "\n^[[1m^[[92m   Compiling^[[0m const_panic v0.2.17"
    ok &= check("ci-build-lines-dropped", test_lines(noisy).splitlines(),
                ["test planet_ok ... ok", "test result: ok."])
    if ok:
        print("test_validate_reuses_ci_results ... ok")
    return ok


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

    # #847 T3: the validator reuses CI's passed run on the same commit.
    ok &= reuses_ci_results()

    # #1032: hand-judged criteria and the owner-decision replacement.
    ok &= judges_hand_judged_criteria(sha)

    if not ok:
        print("fixtures: FAIL")
        return 1
    print("fixtures: all ok (validate criteria parsing)")
    return 0


def judges_hand_judged_criteria(sha):
    """#1032 Q1 A / Q2 A: the validator's judgment counts, only on rows the
    spec marks judged by the validator, and a replacement needs an owner
    decision comment on the same Issue."""
    import post_verdict
    ok = True
    hand = "(judged by the validator from timing.txt; no script probe)"
    rows = [{"criterion": "C1: The planet shows (verify: contains planet_ok ... ok in cargo-test.log)",
             "status": "MET", "evidence": "found"},
            {"criterion": f"C2: Ready sooner {hand}", "status": "UNVERIFIABLE", "evidence": "no probe"},
            {"criterion": f"C3: Frames steady {hand}", "status": "UNVERIFIABLE", "evidence": "no probe"}]
    text = verdict_text("FAIL", sha, 1, "t", 1, ["cargo test: rc=0 OK"], [], [], rows)
    apply = post_verdict.apply_judgments
    judged, err = apply(text, ["C2=MET: 56% of today", "C3=MET: within noise"])
    body = finish(judged, "none", "none", [])
    ok &= check("judged-all-met-passes",
                (err, "Requirements: MET 3/3" in body, verdict_problems(body),
                 MARKER.search(body).group(1)),
                ([], True, [], sha))
    ok &= check("judged-row-shows-reason",
                "- MET (judged): C2: Ready sooner" in body and "(judged: 56% of today)" in body,
                True)
    one_not, err = apply(text, ["C2=MET: fine", "C3=NOT MET: 9 ms slower"])
    ok &= check("judged-not-met-fails",
                (err, "Validator: FAIL" in one_not, "Requirements: MET 2/3" in one_not),
                ([], True, True))
    left, err = apply(text, ["C2=MET: fine"])
    ok &= check("unjudged-hand-row-keeps-fail",
                (err, "Validator: FAIL" in left, "Requirements: MET 2/3" in left),
                ([], True, True))
    _, err = apply(text, ["C1=MET: looks fine"])
    ok &= check("script-row-refused", ("C1 is checked by the script" in " ".join(err)), True)
    _, err = apply(text, ["C9=MET: x", "C2 is fine"])
    ok &= check("unknown-and-malformed-refused", len(err), 2)
    runtime_fail = text.replace("Runtime: PASS", "Runtime: FAIL")
    still, _ = apply(runtime_fail, ["C2=MET: a", "C3=MET: b"])
    ok &= check("judged-never-hides-runtime-fail", "Validator: FAIL" in still, True)
    same, err = apply(text, [])
    ok &= check("no-judges-leaves-text", (same, err), (text, []))

    class Fake:
        def __init__(self, out, rc=0):
            self.stdout, self.returncode, self.stderr = out, rc, ""
    real = post_verdict.run
    url = "https://github.com/o/r/issues/7#issuecomment-55"
    try:
        def owner_on_7(cmd):
            if cmd[:2] == ["gh", "api"]:
                return Fake("taftary\thttps://api.github.com/repos/o/r/issues/7")
            return Fake("taftary")
        post_verdict.run = owner_on_7
        ok &= check("owner-decision-accepted", post_verdict.owner_decision_problem(7, url), None)
        ok &= check("decision-on-other-issue-refused",
                    "on issue #8" in post_verdict.owner_decision_problem(8, url), True)

        def someone_else(cmd):
            if cmd[:2] == ["gh", "api"]:
                return Fake("helper-bot\thttps://api.github.com/repos/o/r/issues/7")
            return Fake("taftary")
        post_verdict.run = someone_else
        ok &= check("non-owner-decision-refused",
                    "not the repository owner" in post_verdict.owner_decision_problem(7, url), True)
        ok &= check("non-comment-url-refused",
                    "comment URL" in post_verdict.owner_decision_problem(7, "https://x/issues/7"), True)
    finally:
        post_verdict.run = real
    return ok


if __name__ == "__main__":
    sys.exit(main())
