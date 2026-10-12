"""Fixture harness for validate.py criteria parsing (issue #533, T2).

No cargo, no network: only the pure helpers. Pins that every
letter-prefixed criterion (C1, R1, ...) is read and counted, and that the
criteria-shape refusal (issue #92) covers every prefix too.

Usage:
  python scripts/validation/test_validate.py
Exit 0 when every fixture behaves; non-zero naming the first failure.
"""

import os
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from validate import check_criteria_shape, parse_criteria  # noqa: E402

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

    if not ok:
        print("fixtures: FAIL")
        return 1
    print("fixtures: all ok (validate criteria parsing)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
